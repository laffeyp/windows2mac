// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* hv_fp.c — AArch64 floating-point control in a Hypervisor.framework guest (Sprint 039).
 *
 * Usage: hv_fp
 * Prints one JSON object per line. First the ID register and FPCR read-back:
 *   {"check":"id_aa64mmfr1","value":...}            ID_AA64MMFR1_EL1 as the guest reads it (mrs)
 *   {"check":"fpcr_readback","set":...,"read":...}  FPCR after the host sets AH|FIZ|NEP, as the guest reads it
 * then one line per (test, FPCR):
 *   {"test":...,"fpcr":...,"s1":...,"s2":...,"s0":...,"x0":...,"fpsr":...}
 * Each test is one instruction followed by `hvc #0` in its own 8-byte slot, run at EL1 with the
 * MMU off and CPACR_EL1.FPEN = 0b11. S1 and S2 are the low words of Q1 and Q2; S0 (Q0's low word)
 * and X0 are read after the exit, with FPSR. scripts/probe_fp_control.py judges each line against
 * the x86 rule (src/w2m/x86fp.py).
 */
#include <Hypervisor/Hypervisor.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define GUEST_IPA 0x10000000ULL
#define HVC0 0xD4000002u
#define FPCR_FIZ (1u << 0)
#define FPCR_AH (1u << 1)
#define FPCR_NEP (1u << 2)

struct test { const char *name; uint32_t insn; uint32_t s1, s2; };

static const struct test TESTS[] = {
    {"fmin_number_qnan", 0x1E225820, 0x3F800000, 0x7FC00001},
    {"fmin_qnan_number", 0x1E225820, 0x7FC00001, 0x3F800000},
    {"fmin_number_snan", 0x1E225820, 0x3F800000, 0x7F800001},
    {"fmin_pzero_nzero", 0x1E225820, 0x00000000, 0x80000000},
    {"fmin_nzero_pzero", 0x1E225820, 0x80000000, 0x00000000},
    {"fmax_pzero_nzero", 0x1E224820, 0x00000000, 0x80000000},
    {"fmax_nzero_pzero", 0x1E224820, 0x80000000, 0x00000000},
    {"fmax_number_qnan", 0x1E224820, 0x3F800000, 0x7FC00001},
    {"fmul_tiny_before_not_after_rounding", 0x1E220820, 0x3F7FFFFE, 0x00800001},
    {"fmul_tiny_after_rounding", 0x1E220820, 0x3F000000, 0x00800001},
    {"fsub_inf_inf_default_nan", 0x1E223820, 0x7F800000, 0x7F800000},
    {"fadd_qnan_snan_priority", 0x1E222820, 0x7FC00001, 0x7F800002},
    {"fadd_denormal_input", 0x1E222820, 0x00000001, 0x00000000},
    {"fcvtzs_w_overflow", 0x1E380020, 0x4F32D05E, 0},
    {"fcvtzs_w_negative_overflow", 0x1E380020, 0xCF32D05E, 0},
    {"fcvtzs_w_nan", 0x1E380020, 0x7FC00000, 0},
    {"fcvtzs_x_overflow", 0x9E380020, 0x5F32D05E, 0},
};
#define NTESTS (sizeof TESTS / sizeof TESTS[0])

static hv_vcpu_t vcpu;
static hv_vcpu_exit_t *exit_info;

static void set_s(hv_simd_fp_reg_t r, uint32_t v) {
    hv_simd_fp_uchar16_t q;
    memset(&q, 0, sizeof q);
    memcpy(&q, &v, 4);
    hv_vcpu_set_simd_fp_reg(vcpu, r, q);
}

static uint32_t get_s(hv_simd_fp_reg_t r) {
    hv_simd_fp_uchar16_t q;
    hv_vcpu_get_simd_fp_reg(vcpu, r, &q);
    uint32_t v;
    memcpy(&v, &q, 4);
    return v;
}

static int run_at(uint64_t pc) {
    hv_vcpu_set_reg(vcpu, HV_REG_PC, pc);
    hv_vcpu_set_reg(vcpu, HV_REG_CPSR, 0x3c5);
    if (hv_vcpu_run(vcpu) != HV_SUCCESS || exit_info->reason != HV_EXIT_REASON_EXCEPTION) return 0;
    return ((exit_info->exception.syndrome >> 26) & 0x3f) == 0x16;       /* HVC from AArch64 */
}

int main(void) {
    hv_vm_config_t cfg = hv_vm_config_create();
    if (hv_vm_create(cfg) != HV_SUCCESS) { fprintf(stderr, "hv_vm_create failed\n"); return 1; }
    uint32_t *mem = NULL;
    if (posix_memalign((void **)&mem, 16384, 16384) != 0) return 1;
    memset(mem, 0, 16384);
    /* slot 0: mrs x0, id_aa64mmfr1_el1; slot 1: mrs x0, fpcr; slots 2..: the tests */
    mem[0] = 0xD5380720; mem[1] = HVC0;
    mem[2] = 0xD53B4400; mem[3] = HVC0;
    for (unsigned i = 0; i < NTESTS; i++) { mem[4 + 2 * i] = TESTS[i].insn; mem[5 + 2 * i] = HVC0; }
    if (hv_vm_map(mem, GUEST_IPA, 16384, HV_MEMORY_READ | HV_MEMORY_EXEC) != HV_SUCCESS) return 1;
    if (hv_vcpu_create(&vcpu, &exit_info, NULL) != HV_SUCCESS) return 1;
    hv_vcpu_set_sys_reg(vcpu, HV_SYS_REG_CPACR_EL1, 3ULL << 20);         /* FPEN: no FP trap at EL1 */

    uint64_t x0 = 0;
    int ok = run_at(GUEST_IPA);
    hv_vcpu_get_reg(vcpu, HV_REG_X0, &x0);
    printf("{\"check\":\"id_aa64mmfr1\",\"ok\":%s,\"value\":%llu}\n", ok ? "true" : "false", (unsigned long long)x0);

    uint64_t want = FPCR_AH | FPCR_FIZ | FPCR_NEP, host = 0;
    hv_vcpu_set_reg(vcpu, HV_REG_FPCR, want);
    ok = run_at(GUEST_IPA + 8);
    hv_vcpu_get_reg(vcpu, HV_REG_X0, &x0);
    hv_vcpu_get_reg(vcpu, HV_REG_FPCR, &host);
    printf("{\"check\":\"fpcr_readback\",\"ok\":%s,\"set\":%llu,\"read\":%llu,\"host_read\":%llu}\n", ok ? "true" : "false",
           (unsigned long long)want, (unsigned long long)x0, (unsigned long long)host);

    static const uint64_t FPCRS[] = {0, FPCR_AH, FPCR_AH | FPCR_FIZ};
    for (unsigned i = 0; i < NTESTS; i++) {
        for (unsigned f = 0; f < sizeof FPCRS / sizeof FPCRS[0]; f++) {
            hv_vcpu_set_reg(vcpu, HV_REG_FPCR, FPCRS[f]);
            hv_vcpu_set_reg(vcpu, HV_REG_FPSR, 0);
            hv_vcpu_set_reg(vcpu, HV_REG_X0, 0);
            set_s(HV_SIMD_FP_REG_Q0, 0xDEADBEEF);
            set_s(HV_SIMD_FP_REG_Q1, TESTS[i].s1);
            set_s(HV_SIMD_FP_REG_Q2, TESTS[i].s2);
            ok = run_at(GUEST_IPA + 16 + 8 * i);
            uint64_t fpsr = 0;
            hv_vcpu_get_reg(vcpu, HV_REG_X0, &x0);
            hv_vcpu_get_reg(vcpu, HV_REG_FPSR, &fpsr);
            printf("{\"test\":\"%s\",\"ok\":%s,\"fpcr\":%llu,\"s1\":%u,\"s2\":%u,\"s0\":%u,\"x0\":%llu,\"fpsr\":%llu}\n",
                   TESTS[i].name, ok ? "true" : "false", (unsigned long long)FPCRS[f], TESTS[i].s1, TESTS[i].s2,
                   get_s(HV_SIMD_FP_REG_Q0), (unsigned long long)x0, (unsigned long long)fpsr);
        }
    }
    hv_vcpu_destroy(vcpu);
    hv_vm_destroy();
    return 0;
}
