/* hv_capability.c — destination capability probe for the managed-VM backend (Sprint 008).
 *
 * Usage: hv_capability 4k|16k
 * Prints one JSON object per line: {"check": ..., "ok": true|false, "detail": ...}.
 *
 * 4k:  VM with HV_IPA_GRANULE_4KB; map one 4 KB page at IPA 0x10001000 (4 KB-aligned, not
 *      16 KB-aligned); run guest code `movz x0,#42; hvc #0` from it; expect an HVC exit with
 *      X0 == 42. Set ACTLR_EL1 bit 1 (EnTSO) and read it back before and after the run.
 * 16k: VM with the default granule; the same 4 KB mapping is expected to be refused.
 */
#include <Hypervisor/Hypervisor.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define GUEST_IPA 0x10001000ULL
#define PAGE4K 0x1000
#define ENTSO (1ULL << 1)

static void report(const char *check, int ok, const char *fmt, ...) {
    char detail[256];
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(detail, sizeof detail, fmt, ap);
    va_end(ap);
    printf("{\"check\":\"%s\",\"ok\":%s,\"detail\":\"%s\"}\n", check, ok ? "true" : "false",
           detail);
}

int main(int argc, char **argv) {
    if (argc != 2 || (strcmp(argv[1], "4k") && strcmp(argv[1], "16k"))) {
        fprintf(stderr, "usage: hv_capability 4k|16k\n");
        return 2;
    }
    int want4k = strcmp(argv[1], "4k") == 0;

    hv_ipa_granule_t def = 0;
    hv_return_t r = hv_vm_config_get_default_ipa_granule(&def);
    report("default_granule", r == HV_SUCCESS, "hv_return 0x%x default %s", r,
           def == HV_IPA_GRANULE_4KB ? "4KB" : def == HV_IPA_GRANULE_16KB ? "16KB" : "other");

    hv_vm_config_t cfg = hv_vm_config_create();
    if (want4k) {
        r = hv_vm_config_set_ipa_granule(cfg, HV_IPA_GRANULE_4KB);
        report("set_granule_4k", r == HV_SUCCESS, "hv_return 0x%x", r);
        if (r != HV_SUCCESS) return 1;
    }
    r = hv_vm_create(cfg);
    report("vm_create", r == HV_SUCCESS, "hv_return 0x%x", r);
    if (r != HV_SUCCESS) return 1;

    void *mem = NULL;
    if (posix_memalign(&mem, 16384, 16384) != 0) return 1;
    uint32_t code[2] = {0xD2800540u /* movz x0, #42 */, 0xD4000002u /* hvc #0 */};
    memcpy(mem, code, sizeof code);
    r = hv_vm_map(mem, GUEST_IPA, PAGE4K, HV_MEMORY_READ | HV_MEMORY_EXEC);
    report("map_4k_page_at_0x10001000", r == HV_SUCCESS, "hv_return 0x%x", r);
    if (!want4k) return r == HV_SUCCESS ? 1 : 0; /* control: 16 KB granule must refuse */
    if (r != HV_SUCCESS) return 1;

    hv_vcpu_t vcpu;
    hv_vcpu_exit_t *exit_info;
    r = hv_vcpu_create(&vcpu, &exit_info, NULL);
    report("vcpu_create", r == HV_SUCCESS, "hv_return 0x%x", r);
    if (r != HV_SUCCESS) return 1;

    uint64_t actlr = 0;
    hv_return_t rs = hv_vcpu_set_sys_reg(vcpu, HV_SYS_REG_ACTLR_EL1, ENTSO);
    hv_return_t rg = hv_vcpu_get_sys_reg(vcpu, HV_SYS_REG_ACTLR_EL1, &actlr);
    report("entso_set_readback", rs == HV_SUCCESS && rg == HV_SUCCESS && (actlr & ENTSO),
           "set 0x%x get 0x%x actlr 0x%llx", rs, rg, (unsigned long long)actlr);

    hv_vcpu_set_reg(vcpu, HV_REG_PC, GUEST_IPA);
    hv_vcpu_set_reg(vcpu, HV_REG_CPSR, 0x3c5);
    r = hv_vcpu_run(vcpu);
    uint64_t x0 = 0;
    hv_vcpu_get_reg(vcpu, HV_REG_X0, &x0);
    uint64_t esr = exit_info->exception.syndrome;
    unsigned ec = (unsigned)((esr >> 26) & 0x3f);
    report("guest_runs_from_4k_page",
           r == HV_SUCCESS && exit_info->reason == HV_EXIT_REASON_EXCEPTION && ec == 0x16 &&
               x0 == 42,
           "hv_return 0x%x exit_reason %u esr_ec 0x%x x0 %llu", r, exit_info->reason, ec,
           (unsigned long long)x0);

    actlr = 0;
    rg = hv_vcpu_get_sys_reg(vcpu, HV_SYS_REG_ACTLR_EL1, &actlr);
    report("entso_after_run", rg == HV_SUCCESS && (actlr & ENTSO), "get 0x%x actlr 0x%llx", rg,
           (unsigned long long)actlr);

    hv_vcpu_destroy(vcpu);
    hv_vm_destroy();
    return 0;
}
