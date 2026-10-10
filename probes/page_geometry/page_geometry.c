// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* page_geometry.c — 4 KB guest page protection on a 16 KB-page Mac (Sprint 012).
 *
 * v0.2 section 8 example: [A, A+4096) readable/writable, [A+4096, A+8192) inaccessible.
 *
 * Usage:
 *   page_geometry native_correctness      mprotect at 4 KB granularity; software shadow check
 *   page_geometry vm_correctness          guest 4 KB stage-1 pages; one invalid page; guest fault
 *   page_geometry native <plain|check_each|check_page_cross> <n> <reps>
 *   page_geometry vm <pages_4k|blocks_2m> <n> <reps>
 * Prints one JSON line.
 */
#include <Hypervisor/Hypervisor.h>
#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <time.h>
#include <unistd.h>

/* x0 a, x1 b, x2 n, x3 reps, x4 shadow (1 byte per 4 KB page: bit0 read, bit1 write),
 * x5 base of the shadowed region. Returns 0, or the faulting address. */
#define CHK(reg, bit, out) \
    "  sub x12, " reg ", x5\n  lsr x12, x12, #12\n  ldrb w13, [x4, x12]\n  tbz w13, #" bit ", " out "\n"

__asm__(".text\n"
        ".p2align 4\n.globl _k_plain\n_k_plain:\n"
        "  mov x9, x3\n"
        "1: mov x6, x0\n  mov x7, x1\n  mov x8, x2\n"
        "2: ldr w10, [x6]\n  ldr w11, [x7]\n  add w10, w10, w11\n  str w10, [x6]\n"
        "  add x6, x6, #4\n  add x7, x7, #4\n  subs x8, x8, #1\n  b.ne 2b\n"
        "  subs x9, x9, #1\n  b.ne 1b\n  mov x0, #0\n  ret\n"

        ".p2align 4\n.globl _k_check_each\n_k_check_each:\n"
        "  mov x9, x3\n"
        "1: mov x6, x0\n  mov x7, x1\n  mov x8, x2\n"
        "2:\n" CHK("x6", "0", "9f") "  ldr w10, [x6]\n" CHK("x7", "0", "8f") "  ldr w11, [x7]\n"
        "  add w10, w10, w11\n" CHK("x6", "1", "9f") "  str w10, [x6]\n"
        "  add x6, x6, #4\n  add x7, x7, #4\n  subs x8, x8, #1\n  b.ne 2b\n"
        "  subs x9, x9, #1\n  b.ne 1b\n  mov x0, #0\n  ret\n"
        "9: mov x0, x6\n  ret\n8: mov x0, x7\n  ret\n"

        /* Valid only when a and b are 4 KB-aligned with equal stride (asserted by the caller)
         * and no protection change can occur during the loop. */
        ".p2align 4\n.globl _k_check_page_cross\n_k_check_page_cross:\n"
        "  mov x9, x3\n"
        "1: mov x6, x0\n  mov x7, x1\n  mov x8, x2\n"
        /* common path falls through; the page-entry check is out of line (taken 1 in 1024) */
        "2: tst x6, #0xfff\n  b.eq 7f\n"
        "3: ldr w10, [x6]\n  ldr w11, [x7]\n  add w10, w10, w11\n  str w10, [x6]\n"
        "  add x6, x6, #4\n  add x7, x7, #4\n  subs x8, x8, #1\n  b.ne 2b\n"
        "  subs x9, x9, #1\n  b.ne 1b\n  mov x0, #0\n  ret\n"
        "7:\n" CHK("x6", "0", "9f") "  tbz w13, #1, 9f\n" CHK("x7", "0", "8f") "  b 3b\n"
        "9: mov x0, x6\n  ret\n8: mov x0, x7\n  ret\n"

        /* guest: plain kernel ending in hvc #0 */
        ".p2align 4\n.globl _g_plain\n_g_plain:\n"
        "  mov x9, x3\n"
        "1: mov x6, x0\n  mov x7, x1\n  mov x8, x2\n"
        "2: ldr w10, [x6]\n  ldr w11, [x7]\n  add w10, w10, w11\n  str w10, [x6]\n"
        "  add x6, x6, #4\n  add x7, x7, #4\n  subs x8, x8, #1\n  b.ne 2b\n"
        "  subs x9, x9, #1\n  b.ne 1b\n  hvc #0\n"
        ".globl _g_plain_end\n_g_plain_end:\n"
        /* guest: read page 0 then page 1, then hvc #0 */
        ".p2align 4\n.globl _g_fault\n_g_fault:\n"
        "  ldr w0, [x2]\n  ldr w1, [x3]\n  hvc #0\n"
        ".globl _g_fault_end\n_g_fault_end:\n");

typedef uint64_t (*kfn)(int32_t *, int32_t *, uint64_t, uint64_t, const uint8_t *, uintptr_t);
extern uint64_t k_plain(int32_t *, int32_t *, uint64_t, uint64_t, const uint8_t *, uintptr_t);
extern uint64_t k_check_each(int32_t *, int32_t *, uint64_t, uint64_t, const uint8_t *, uintptr_t);
extern uint64_t k_check_page_cross(int32_t *, int32_t *, uint64_t, uint64_t, const uint8_t *,
                                   uintptr_t);
extern const uint8_t g_plain[], g_plain_end[], g_fault[], g_fault_end[];

static uint64_t now_ns(void) { return clock_gettime_nsec_np(CLOCK_UPTIME_RAW); }

/* ---------------- native ---------------- */
static int native_correctness(void) {
    uint8_t *r = mmap(NULL, 16384, PROT_READ | PROT_WRITE, MAP_ANON | MAP_PRIVATE, -1, 0);
    int rc = mprotect(r + 4096, 4096, PROT_NONE);
    int e = errno;
    uint8_t shadow[4] = {3, 0, 3, 3}; /* page 0 RW, page 1 no access */
    uintptr_t base = (uintptr_t)r;
    uint64_t f_ok = k_check_each((int32_t *)r, (int32_t *)r, 1024, 1, shadow, base);
    uint64_t f_bad = k_check_each((int32_t *)r, (int32_t *)(r + 4096), 1024, 1, shadow, base);
    uint64_t f_cross = k_check_page_cross((int32_t *)r, (int32_t *)(r + 4096), 1024, 1, shadow,
                                          base);
    printf("{\"mode\":\"native_correctness\",\"host_page\":%d,\"mprotect_4k_rc\":%d,"
           "\"mprotect_4k_errno\":%d,\"check_each_page0_fault\":%llu,"
           "\"check_each_page1_fault_offset\":%lld,\"check_page_cross_page1_fault_offset\":%lld}\n",
           getpagesize(), rc, rc ? e : 0, (unsigned long long)f_ok,
           f_bad ? (long long)(f_bad - base) : -1, f_cross ? (long long)(f_cross - base) : -1);
    return 0;
}

static int native_cost(const char *variant, uint64_t n, uint64_t reps, uint64_t *ns) {
    kfn k = !strcmp(variant, "plain")              ? k_plain
            : !strcmp(variant, "check_each")       ? k_check_each
            : !strcmp(variant, "check_page_cross") ? k_check_page_cross
                                                   : NULL;
    if (!k) return 2;
    size_t bytes = (n * 4 + 0x3fff) & ~0x3fffULL;
    uint8_t *r = mmap(NULL, 2 * bytes, PROT_READ | PROT_WRITE, MAP_ANON | MAP_PRIVATE, -1, 0);
    uint8_t *shadow = malloc(2 * bytes / 4096);
    memset(shadow, 3, 2 * bytes / 4096);
    memset(r, 1, 2 * bytes);
    int32_t *a = (int32_t *)r, *b = (int32_t *)(r + bytes);
    if (k(a, b, n, 1, shadow, (uintptr_t)r)) return 1; /* warm; must not fault */
    uint64_t t0 = now_ns();
    uint64_t f = k(a, b, n, reps, shadow, (uintptr_t)r);
    *ns = now_ns() - t0;
    return f ? 1 : 0;
}

/* ---------------- VM ---------------- */
#define TABLE_IPA 0x0F000000ULL
#define TABLE_BYTES 0x100000ULL
#define CODE_IPA 0x10000000ULL
#define VBAR_OFF 0x800ULL
#define DATA_IPA 0x20000000ULL

typedef struct {
    hv_vcpu_t v;
    hv_vcpu_exit_t *ex;
    uint64_t *tables; /* L1 at +0, L2 at +0x1000, L3 tables from +0x2000 */
    uint8_t *code, *data;
} Vm;

static uint64_t *tbl(Vm *m, uint64_t off) { return (uint64_t *)((uint8_t *)m->tables + off); }

/* data: `bytes` at DATA_IPA, mapped either as 4 KB pages (L3) or 2 MB blocks (L2) */
static int vm_setup(Vm *m, size_t bytes, int pages4k, int64_t invalid_page) {
    hv_vm_config_t cfg = hv_vm_config_create();
    if (hv_vm_config_set_ipa_granule(cfg, HV_IPA_GRANULE_4KB) || hv_vm_create(cfg)) return 1;
    m->tables = mmap(NULL, TABLE_BYTES, PROT_READ | PROT_WRITE, MAP_ANON | MAP_PRIVATE, -1, 0);
    m->code = mmap(NULL, 0x4000, PROT_READ | PROT_WRITE, MAP_ANON | MAP_PRIVATE, -1, 0);
    m->data = mmap(NULL, bytes, PROT_READ | PROT_WRITE, MAP_ANON | MAP_PRIVATE, -1, 0);
    memset(m->tables, 0, TABLE_BYTES);
    memset(m->data, 1, bytes);
    const uint64_t attr = (3ULL << 8) | (1ULL << 10); /* AttrIndx 0 (Normal WB), ISH, AF */
    tbl(m, 0)[0] = (TABLE_IPA + 0x1000) | 0x3;       /* L1[0] -> L2 */
    tbl(m, 0x1000)[CODE_IPA >> 21] = CODE_IPA | attr | 0x1; /* 2 MB block for code */
    size_t blocks = bytes >> 21;
    if (2 + blocks > TABLE_BYTES / 0x1000) return 1;
    for (size_t k = 0; k < blocks; k++) {
        uint64_t ipa = DATA_IPA + (k << 21);
        if (pages4k) {
            uint64_t l3off = 0x2000 + k * 0x1000;
            tbl(m, 0x1000)[ipa >> 21] = (TABLE_IPA + l3off) | 0x3;
            for (int j = 0; j < 512; j++) {
                int64_t page = (int64_t)(k * 512 + (size_t)j);
                tbl(m, l3off)[j] = page == invalid_page ? 0 : ((ipa + ((uint64_t)j << 12)) | attr | 0x3);
            }
        } else {
            tbl(m, 0x1000)[ipa >> 21] = ipa | attr | 0x1;
        }
    }
    uint32_t *vec = (uint32_t *)(m->code + VBAR_OFF);
    for (int i = 0; i < 512; i++) vec[i] = 0xD4000022u; /* hvc #1 in every vector slot */
    if (hv_vm_map(m->tables, TABLE_IPA, TABLE_BYTES, HV_MEMORY_READ | HV_MEMORY_WRITE) ||
        hv_vm_map(m->code, CODE_IPA, 0x4000, HV_MEMORY_READ | HV_MEMORY_EXEC) ||
        hv_vm_map(m->data, DATA_IPA, bytes, HV_MEMORY_READ | HV_MEMORY_WRITE))
        return 1;
    if (hv_vcpu_create(&m->v, &m->ex, NULL)) return 1;
    hv_vcpu_set_sys_reg(m->v, HV_SYS_REG_MAIR_EL1, 0xFF);
    hv_vcpu_set_sys_reg(m->v, HV_SYS_REG_TCR_EL1, 25 | (1ULL << 8) | (1ULL << 10) |
                                                      (3ULL << 12) | (1ULL << 23) | (1ULL << 32));
    hv_vcpu_set_sys_reg(m->v, HV_SYS_REG_TTBR0_EL1, TABLE_IPA);
    hv_vcpu_set_sys_reg(m->v, HV_SYS_REG_VBAR_EL1, CODE_IPA + VBAR_OFF);
    hv_vcpu_set_sys_reg(m->v, HV_SYS_REG_SCTLR_EL1, 0x30D01805ULL);
    return 0;
}

/* run until an HVC exit; returns the HVC immediate, or -1 */
static int vm_go(Vm *m) {
    for (;;) {
        if (hv_vcpu_run(m->v)) return -1;
        if (m->ex->reason == HV_EXIT_REASON_VTIMER_ACTIVATED) {
            hv_vcpu_set_vtimer_mask(m->v, true);
            continue;
        }
        if (m->ex->reason == HV_EXIT_REASON_CANCELED) continue;
        uint64_t esr = m->ex->exception.syndrome;
        if (m->ex->reason == HV_EXIT_REASON_EXCEPTION && ((esr >> 26) & 0x3f) == 0x16)
            return (int)(esr & 0xffff);
        fprintf(stderr, "guest exit %u esr 0x%llx\n", m->ex->reason, (unsigned long long)esr);
        return -1;
    }
}

static int vm_correctness(void) {
    Vm m;
    if (vm_setup(&m, 2 << 20, 1, 1)) return 1; /* page 1 (A+4096) invalid */
    memcpy(m.code, g_fault, (size_t)(g_fault_end - g_fault));
    ((uint32_t *)m.data)[0] = 0x11223344u;
    hv_vcpu_set_reg(m.v, HV_REG_PC, CODE_IPA);
    hv_vcpu_set_reg(m.v, HV_REG_CPSR, 0x3c5);
    hv_vcpu_set_reg(m.v, HV_REG_X2, DATA_IPA);
    hv_vcpu_set_reg(m.v, HV_REG_X3, DATA_IPA + 4096);
    int imm = vm_go(&m);
    uint64_t x0 = 0, esr1 = 0, far1 = 0, elr1 = 0;
    hv_vcpu_get_reg(m.v, HV_REG_X0, &x0);
    hv_vcpu_get_sys_reg(m.v, HV_SYS_REG_ESR_EL1, &esr1);
    hv_vcpu_get_sys_reg(m.v, HV_SYS_REG_FAR_EL1, &far1);
    hv_vcpu_get_sys_reg(m.v, HV_SYS_REG_ELR_EL1, &elr1);
    printf("{\"mode\":\"vm_correctness\",\"hvc_imm\":%d,\"page0_value\":\"0x%llx\","
           "\"guest_esr_ec\":\"0x%llx\",\"guest_far\":\"0x%llx\",\"guest_elr_offset\":%lld}\n",
           imm, (unsigned long long)x0, (unsigned long long)((esr1 >> 26) & 0x3f),
           (unsigned long long)far1, (long long)(elr1 - CODE_IPA));
    return 0;
}

static int vm_cost(const char *variant, uint64_t n, uint64_t reps, uint64_t *ns) {
    int pages4k = !strcmp(variant, "pages_4k");
    if (!pages4k && strcmp(variant, "blocks_2m")) return 2;
    size_t per = (n * 4 + 0x1fffff) & ~0x1fffffULL; /* each array on its own 2 MB boundary */
    Vm m;
    if (vm_setup(&m, 2 * per, pages4k, -1)) return 1;
    memcpy(m.code, g_plain, (size_t)(g_plain_end - g_plain));
    for (int pass = 0; pass < 2; pass++) {
        hv_vcpu_set_reg(m.v, HV_REG_PC, CODE_IPA);
        hv_vcpu_set_reg(m.v, HV_REG_CPSR, 0x3c5);
        hv_vcpu_set_reg(m.v, HV_REG_X0, DATA_IPA);
        hv_vcpu_set_reg(m.v, HV_REG_X1, DATA_IPA + per);
        hv_vcpu_set_reg(m.v, HV_REG_X2, n);
        hv_vcpu_set_reg(m.v, HV_REG_X3, pass ? reps : 1);
        uint64_t t0 = now_ns();
        if (vm_go(&m) != 0) return 1;
        *ns = now_ns() - t0;
    }
    return 0;
}

int main(int argc, char **argv) {
    if (argc == 2 && !strcmp(argv[1], "native_correctness")) return native_correctness();
    if (argc == 2 && !strcmp(argv[1], "vm_correctness")) return vm_correctness();
    if (argc != 5) {
        fprintf(stderr, "usage: see header\n");
        return 2;
    }
    uint64_t n = strtoull(argv[3], NULL, 10), reps = strtoull(argv[4], NULL, 10), ns = 0;
    int rc = !strcmp(argv[1], "native") ? native_cost(argv[2], n, reps, &ns)
             : !strcmp(argv[1], "vm")   ? vm_cost(argv[2], n, reps, &ns)
                                        : 2;
    if (rc) {
        fprintf(stderr, "%s %s failed (%d)\n", argv[1], argv[2], rc);
        return rc;
    }
    printf("{\"mode\":\"%s\",\"variant\":\"%s\",\"n\":%llu,\"reps\":%llu,\"ns\":%llu,"
           "\"ns_per_access\":%.4f}\n",
           argv[1], argv[2], (unsigned long long)n, (unsigned long long)reps,
           (unsigned long long)ns, (double)ns / (3.0 * (double)n * (double)reps));
    return 0;
}
