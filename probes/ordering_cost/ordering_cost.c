/* ordering_cost.c — per-access cost of x86-TSO ordering on this Mac (Sprint 011).
 *
 * Kernel: for r < reps: for i < n: a[i] += b[i]   (2 loads + 1 store per element).
 * Native variants run in this process; VM variants run the plain kernel in a guest with
 * ACTLR_EL1.EnTSO off/on (guest MMU on, Normal WB, 4 KB IPA granule).
 *
 * Usage:
 *   ordering_cost mp <iterations>                     native MP litmus: plain vs ldapr/stlr
 *   ordering_cost native <variant> <n> <reps>         variant: plain|ldapr_stlr|ldar_stlr|dmb_fences
 *   ordering_cost vm <entso_off|entso_on|entso_on_16k> <n> <reps>   (16k: default IPA granule)
 * Prints one JSON line.
 */
#include <Hypervisor/Hypervisor.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <time.h>

/* Every variant: same loop shape, explicit address adds; only the memory ops differ. */
#define KERNEL(name, LD1, LD2, PRE_ST, ST)                                                         \
    ".p2align 4\n.globl _" #name "\n_" #name ":\n"                                                 \
    "  mov x9, x3\n"                                                                               \
    "1: mov x5, x0\n  mov x6, x1\n  mov x7, x2\n"                                                  \
    "2: " LD1 " w10, [x5]\n"                                                                       \
    "  " LD2 " w11, [x6]\n"                                                                        \
    "  add w10, w10, w11\n"                                                                        \
    "  " PRE_ST "\n"                                                                               \
    "  " ST " w10, [x5]\n"                                                                         \
    "  add x5, x5, #4\n  add x6, x6, #4\n"                                                         \
    "  subs x7, x7, #1\n  b.ne 2b\n"                                                               \
    "  subs x9, x9, #1\n  b.ne 1b\n"

__asm__(".text\n"
        KERNEL(k_plain, "ldr", "ldr", "nop", "str") "  ret\n"
        KERNEL(k_ldapr_stlr, "ldapr", "ldapr", "nop", "stlr") "  ret\n"
        KERNEL(k_ldar_stlr, "ldar", "ldar", "nop", "stlr") "  ret\n"
        KERNEL(k_dmb_fences, "ldr", "ldr", "dmb ishld\n  dmb ishst", "str") "  ret\n"
        KERNEL(g_plain, "ldr", "ldr", "nop", "str") "  hvc #0\n"
        ".globl _g_plain_end\n_g_plain_end:\n");

typedef void (*kernel_fn)(int32_t *, int32_t *, uint64_t, uint64_t);
extern void k_plain(int32_t *, int32_t *, uint64_t, uint64_t);
extern void k_ldapr_stlr(int32_t *, int32_t *, uint64_t, uint64_t);
extern void k_ldar_stlr(int32_t *, int32_t *, uint64_t, uint64_t);
extern void k_dmb_fences(int32_t *, int32_t *, uint64_t, uint64_t);
extern const uint8_t g_plain[], g_plain_end[];

static uint64_t now_ns(void) { return clock_gettime_nsec_np(CLOCK_UPTIME_RAW); }

/* ---- native MP litmus: writer x=1;y=1, reader waits for y then reads x ---- */
static uint64_t mp_n;
static volatile uint32_t *mx, *my;
static int mp_lowered;
static atomic_int mp_go;

static void *mp_writer(void *arg) {
    (void)arg;
    while (!atomic_load(&mp_go)) {}
    for (uint64_t i = 0; i < mp_n; i++) {
        if (mp_lowered) {
            __asm__ volatile("stlr %w0, [%1]" ::"r"(1), "r"(&mx[i]) : "memory");
            __asm__ volatile("stlr %w0, [%1]" ::"r"(1), "r"(&my[i]) : "memory");
        } else {
            __asm__ volatile("str %w0, [%1]" ::"r"(1), "r"(&mx[i]) : "memory");
            __asm__ volatile("str %w0, [%1]" ::"r"(1), "r"(&my[i]) : "memory");
        }
    }
    return NULL;
}

static uint64_t mp_run(int lowered) {
    mp_lowered = lowered;
    memset((void *)mx, 0, mp_n * 4);
    memset((void *)my, 0, mp_n * 4);
    atomic_store(&mp_go, 0);
    pthread_t t;
    pthread_create(&t, NULL, mp_writer, NULL);
    uint64_t bad = 0;
    atomic_store(&mp_go, 1);
    for (uint64_t i = 0; i < mp_n; i++) {
        uint32_t f, d;
        do {
            if (lowered) __asm__ volatile("ldapr %w0, [%1]" : "=r"(f) : "r"(&my[i]) : "memory");
            else __asm__ volatile("ldr %w0, [%1]" : "=r"(f) : "r"(&my[i]) : "memory");
        } while (!f);
        if (lowered) __asm__ volatile("ldapr %w0, [%1]" : "=r"(d) : "r"(&mx[i]) : "memory");
        else __asm__ volatile("ldr %w0, [%1]" : "=r"(d) : "r"(&mx[i]) : "memory");
        bad += (d == 0);
    }
    pthread_join(t, NULL);
    return bad;
}

/* ---- managed VM: plain kernel in a guest, EnTSO off/on ---- */
#define CODE_IPA 0x10000000ULL
#define TABLE_IPA 0x0F000000ULL
#define DATA_IPA 0x20000000ULL

static int guest_run(hv_vcpu_t v, hv_vcpu_exit_t *ex) {
    for (;;) {
        if (hv_vcpu_run(v)) return 1;
        if (ex->reason == HV_EXIT_REASON_VTIMER_ACTIVATED) {
            hv_vcpu_set_vtimer_mask(v, true);
            continue;
        }
        if (ex->reason == HV_EXIT_REASON_CANCELED) continue;
        if (ex->reason == HV_EXIT_REASON_EXCEPTION && ((ex->exception.syndrome >> 26) & 0x3f) == 0x16)
            return 0;
        fprintf(stderr, "guest exit %u esr 0x%llx\n", ex->reason,
                (unsigned long long)ex->exception.syndrome);
        return 1;
    }
}

static int vm_run(int entso, int granule4k, uint64_t n, uint64_t reps, uint64_t *ns) {
    hv_vm_config_t cfg = hv_vm_config_create();
    if (granule4k && hv_vm_config_set_ipa_granule(cfg, HV_IPA_GRANULE_4KB)) return 1;
    if (hv_vm_create(cfg)) return 1;
    size_t bytes = ((n * 4 + 0x3fff) & ~0x3fffULL);
    uint8_t *code = mmap(NULL, 0x4000, PROT_READ | PROT_WRITE, MAP_ANON | MAP_PRIVATE, -1, 0);
    uint64_t *table = mmap(NULL, 0x4000, PROT_READ | PROT_WRITE, MAP_ANON | MAP_PRIVATE, -1, 0);
    uint8_t *data = mmap(NULL, 2 * bytes, PROT_READ | PROT_WRITE, MAP_ANON | MAP_PRIVATE, -1, 0);
    memcpy(code, g_plain, (size_t)(g_plain_end - g_plain));
    memset(table, 0, 0x4000);
    table[0] = 0x1 | (3ULL << 8) | (1ULL << 10); /* 0..1 GB block, Normal WB (attr0), ISH, AF */
    memset(data, 1, 2 * bytes);
    if (hv_vm_map(code, CODE_IPA, 0x4000, HV_MEMORY_READ | HV_MEMORY_EXEC) ||
        hv_vm_map(table, TABLE_IPA, 0x4000, HV_MEMORY_READ | HV_MEMORY_WRITE) ||
        hv_vm_map(data, DATA_IPA, 2 * bytes, HV_MEMORY_READ | HV_MEMORY_WRITE))
        return 1;
    hv_vcpu_t v;
    hv_vcpu_exit_t *ex;
    if (hv_vcpu_create(&v, &ex, NULL)) return 1;
    hv_vcpu_set_sys_reg(v, HV_SYS_REG_MAIR_EL1, 0xFF);
    hv_vcpu_set_sys_reg(v, HV_SYS_REG_TCR_EL1, 25 | (1ULL << 8) | (1ULL << 10) | (3ULL << 12) |
                                                   (1ULL << 23) | (1ULL << 32));
    hv_vcpu_set_sys_reg(v, HV_SYS_REG_TTBR0_EL1, TABLE_IPA);
    hv_vcpu_set_sys_reg(v, HV_SYS_REG_SCTLR_EL1, 0x30D01805ULL);
    hv_vcpu_set_sys_reg(v, HV_SYS_REG_ACTLR_EL1, entso ? (1ULL << 1) : 0);
    /* pass 0 warms caches, TLBs, and stage-2 mappings (as the native path's warm call does) */
    for (int pass = 0; pass < 2; pass++) {
        hv_vcpu_set_reg(v, HV_REG_PC, CODE_IPA);
        hv_vcpu_set_reg(v, HV_REG_CPSR, 0x3c5);
        hv_vcpu_set_reg(v, HV_REG_X0, DATA_IPA);
        hv_vcpu_set_reg(v, HV_REG_X1, DATA_IPA + bytes);
        hv_vcpu_set_reg(v, HV_REG_X2, n);
        hv_vcpu_set_reg(v, HV_REG_X3, pass ? reps : 1);
        uint64_t t0 = now_ns();
        if (guest_run(v, ex)) return 1;
        *ns = now_ns() - t0;
    }
    hv_vcpu_destroy(v);
    hv_vm_destroy();
    return 0;
}

int main(int argc, char **argv) {
    if (argc >= 3 && !strcmp(argv[1], "mp")) {
        mp_n = strtoull(argv[2], NULL, 10);
        mx = mmap(NULL, mp_n * 4, PROT_READ | PROT_WRITE, MAP_ANON | MAP_PRIVATE, -1, 0);
        my = mmap(NULL, mp_n * 4, PROT_READ | PROT_WRITE, MAP_ANON | MAP_PRIVATE, -1, 0);
        uint64_t plain = mp_run(0), lowered = mp_run(1);
        printf("{\"mode\":\"mp\",\"iterations\":%llu,\"plain_forbidden\":%llu,"
               "\"ldapr_stlr_forbidden\":%llu}\n",
               (unsigned long long)mp_n, (unsigned long long)plain, (unsigned long long)lowered);
        return 0;
    }
    if (argc != 5) {
        fprintf(stderr, "usage: see header\n");
        return 2;
    }
    uint64_t n = strtoull(argv[3], NULL, 10), reps = strtoull(argv[4], NULL, 10), ns = 0;
    if (!strcmp(argv[1], "native")) {
        kernel_fn k = !strcmp(argv[2], "plain")        ? k_plain
                      : !strcmp(argv[2], "ldapr_stlr") ? k_ldapr_stlr
                      : !strcmp(argv[2], "ldar_stlr")  ? k_ldar_stlr
                      : !strcmp(argv[2], "dmb_fences") ? k_dmb_fences
                                                       : NULL;
        if (!k) return 2;
        int32_t *a = mmap(NULL, n * 4, PROT_READ | PROT_WRITE, MAP_ANON | MAP_PRIVATE, -1, 0);
        int32_t *b = mmap(NULL, n * 4, PROT_READ | PROT_WRITE, MAP_ANON | MAP_PRIVATE, -1, 0);
        memset(a, 1, n * 4);
        memset(b, 1, n * 4);
        k(a, b, n, 1); /* warm */
        uint64_t t0 = now_ns();
        k(a, b, n, reps);
        ns = now_ns() - t0;
    } else if (!strcmp(argv[1], "vm")) {
        int on = !strncmp(argv[2], "entso_on", 8), g4k = strcmp(argv[2], "entso_on_16k") != 0;
        if (vm_run(on, g4k, n, reps, &ns)) {
            fprintf(stderr, "vm run failed\n");
            return 1;
        }
    } else {
        return 2;
    }
    double accesses = 3.0 * (double)n * (double)reps;
    printf("{\"mode\":\"%s\",\"variant\":\"%s\",\"n\":%llu,\"reps\":%llu,\"ns\":%llu,"
           "\"ns_per_access\":%.4f}\n",
           argv[1], argv[2], (unsigned long long)n, (unsigned long long)reps,
           (unsigned long long)ns, (double)ns / accesses);
    return 0;
}
