// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* mp_litmus.c — message-passing litmus test on two Hypervisor.framework vCPUs (Sprint 009).
 *
 * Writer: x[i] = 1; y[i] = 1 (i = 0..N-1).  Reader: spin until y[i] == 1, then read x[i].
 * Reading x[i] == 0 after seeing y[i] == 1 is the MP outcome x86-TSO forbids and AArch64 allows.
 * Rounds alternate ACTLR_EL1.EnTSO off/on on both vCPUs. The guest MMU is on with Normal
 * write-back memory: with the MMU off, EL1 data accesses are Device-nGnRnE and hide reordering.
 *
 * Usage: mp_litmus <rounds> <iterations_per_round>
 * Prints one JSON line per mode plus one summary line.
 */
#include <Hypervisor/Hypervisor.h>
#include <pthread.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>

#define CODE_IPA 0x10000000ULL
#define DATA_IPA 0x20000000ULL
#define X_OFF 0x000000ULL
#define Y_OFF 0x800000ULL
#define DATA_SIZE 0x1000000ULL
#define TABLE_IPA 0x30000000ULL
#define ENTSO (1ULL << 1)

/* Guest code, assembled by the host compiler (host and guest are both AArch64). */
__asm__(".text\n.p2align 4\n"
        ".globl _g_writer_start\n_g_writer_start:\n"
        "  mov w3, #1\n"
        "1: str w3, [x0], #4\n"
        "  str w3, [x1], #4\n"
        "  subs x2, x2, #1\n"
        "  b.ne 1b\n"
        "  hvc #0\n"
        ".globl _g_writer_end\n_g_writer_end:\n"
        ".p2align 4\n"
        ".globl _g_reader_start\n_g_reader_start:\n"
        "  mov x4, #0\n"
        "1: ldr w3, [x1]\n"
        "  cbz w3, 1b\n"
        "  ldr w5, [x0]\n"
        "  cbnz w5, 2f\n"
        "  add x4, x4, #1\n"
        "2: add x0, x0, #4\n"
        "  add x1, x1, #4\n"
        "  subs x2, x2, #1\n"
        "  b.ne 1b\n"
        "  hvc #0\n"
        ".globl _g_reader_end\n_g_reader_end:\n");
extern const uint8_t g_writer_start[], g_writer_end[], g_reader_start[], g_reader_end[];
#define WRITER_OFF 0x0
#define READER_OFF 0x800

static uint64_t n_iter;
static int rounds;
static volatile int cur_round = -1, entso_on = 0;
static pthread_mutex_t mu = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t cv = PTHREAD_COND_INITIALIZER;
static int arrived = 0, generation = 0;

static void barrier(void) {
    pthread_mutex_lock(&mu);
    int gen = generation;
    if (++arrived == 3) {
        arrived = 0;
        generation++;
        pthread_cond_broadcast(&cv);
    } else {
        while (gen == generation) pthread_cond_wait(&cv, &mu);
    }
    pthread_mutex_unlock(&mu);
}

typedef struct {
    int is_reader;
    uint64_t violations[2], iters[2], exits[2], actlr_bad[2], other_exits;
    char fail[200];
} Worker;

static int setup_vcpu(hv_vcpu_t v) {
    int bad = 0;
    bad |= hv_vcpu_set_sys_reg(v, HV_SYS_REG_MAIR_EL1, 0xFF);
    bad |= hv_vcpu_set_sys_reg(v, HV_SYS_REG_TCR_EL1,
                               25 | (1ULL << 8) | (1ULL << 10) | (3ULL << 12) | (1ULL << 23) |
                                   (1ULL << 32));
    bad |= hv_vcpu_set_sys_reg(v, HV_SYS_REG_TTBR0_EL1, TABLE_IPA);
    bad |= hv_vcpu_set_sys_reg(v, HV_SYS_REG_SCTLR_EL1, 0x30D01805ULL);
    return bad;
}

static void *worker(void *arg) {
    Worker *w = (Worker *)arg;
    hv_vcpu_t v;
    hv_vcpu_exit_t *ex;
    if (hv_vcpu_create(&v, &ex, NULL) != HV_SUCCESS || setup_vcpu(v)) {
        snprintf(w->fail, sizeof w->fail, "vcpu create/setup failed");
    }
    for (int r = 0; r < rounds; r++) {
        barrier(); /* round parameters published */
        int mode = entso_on;
        if (!w->fail[0]) {
            hv_vcpu_set_sys_reg(v, HV_SYS_REG_ACTLR_EL1, mode ? ENTSO : 0);
            hv_vcpu_set_reg(v, HV_REG_PC, CODE_IPA + (w->is_reader ? READER_OFF : WRITER_OFF));
            hv_vcpu_set_reg(v, HV_REG_CPSR, 0x3c5);
            hv_vcpu_set_reg(v, HV_REG_X0, DATA_IPA + X_OFF);
            hv_vcpu_set_reg(v, HV_REG_X1, DATA_IPA + Y_OFF);
            hv_vcpu_set_reg(v, HV_REG_X2, n_iter);
            for (;;) {
                hv_return_t rr = hv_vcpu_run(v);
                w->exits[mode]++;
                uint64_t a = 0;
                hv_vcpu_get_sys_reg(v, HV_SYS_REG_ACTLR_EL1, &a);
                if ((a & ENTSO) != (mode ? ENTSO : 0)) w->actlr_bad[mode]++;
                if (rr != HV_SUCCESS) {
                    snprintf(w->fail, sizeof w->fail, "hv_vcpu_run 0x%x", rr);
                    break;
                }
                if (ex->reason == HV_EXIT_REASON_VTIMER_ACTIVATED) {
                    hv_vcpu_set_vtimer_mask(v, true);
                    w->other_exits++;
                    continue;
                }
                if (ex->reason == HV_EXIT_REASON_CANCELED) {
                    w->other_exits++;
                    continue;
                }
                if (ex->reason == HV_EXIT_REASON_EXCEPTION &&
                    ((ex->exception.syndrome >> 26) & 0x3f) == 0x16)
                    break;
                snprintf(w->fail, sizeof w->fail, "exit reason %u esr 0x%llx far 0x%llx",
                         ex->reason, (unsigned long long)ex->exception.syndrome,
                         (unsigned long long)ex->exception.virtual_address);
                break;
            }
            if (w->is_reader) {
                uint64_t x4 = 0;
                hv_vcpu_get_reg(v, HV_REG_X4, &x4);
                w->violations[mode] += x4;
            }
            w->iters[mode] += n_iter;
        }
        barrier(); /* round done */
    }
    hv_vcpu_destroy(v);
    return NULL;
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: mp_litmus <rounds> <iterations_per_round>\n");
        return 2;
    }
    rounds = atoi(argv[1]);
    n_iter = strtoull(argv[2], NULL, 10);
    if (n_iter * 4 > Y_OFF) {
        fprintf(stderr, "iterations_per_round too large (max %llu)\n", Y_OFF / 4);
        return 2;
    }
    hv_vm_config_t cfg = hv_vm_config_create();
    if (hv_vm_config_set_ipa_granule(cfg, HV_IPA_GRANULE_4KB) || hv_vm_create(cfg)) {
        fprintf(stderr, "vm setup failed\n");
        return 1;
    }
    uint8_t *code = mmap(NULL, 0x4000, PROT_READ | PROT_WRITE, MAP_ANON | MAP_PRIVATE, -1, 0);
    uint8_t *data = mmap(NULL, DATA_SIZE, PROT_READ | PROT_WRITE, MAP_ANON | MAP_PRIVATE, -1, 0);
    uint64_t *table = mmap(NULL, 0x4000, PROT_READ | PROT_WRITE, MAP_ANON | MAP_PRIVATE, -1, 0);
    memcpy(code + WRITER_OFF, g_writer_start, (size_t)(g_writer_end - g_writer_start));
    memcpy(code + READER_OFF, g_reader_start, (size_t)(g_reader_end - g_reader_start));
    memset(table, 0, 0x4000);
    table[0] = 0x0ULL | 0x1 | (3ULL << 8) | (1ULL << 10); /* 0..1 GB block, Normal WB, ISH, AF */
    if (hv_vm_map(code, CODE_IPA, 0x4000, HV_MEMORY_READ | HV_MEMORY_EXEC) ||
        hv_vm_map(data, DATA_IPA, DATA_SIZE, HV_MEMORY_READ | HV_MEMORY_WRITE) ||
        hv_vm_map(table, TABLE_IPA, 0x4000, HV_MEMORY_READ | HV_MEMORY_WRITE)) {
        fprintf(stderr, "hv_vm_map failed\n");
        return 1;
    }

    Worker w[2] = {{.is_reader = 0}, {.is_reader = 1}};
    pthread_t t[2];
    for (int i = 0; i < 2; i++) pthread_create(&t[i], NULL, worker, &w[i]);
    for (int r = 0; r < rounds; r++) {
        memset(data, 0, n_iter * 4);
        memset(data + Y_OFF, 0, n_iter * 4);
        cur_round = r;
        entso_on = r & 1; /* alternate off, on, off, on ... */
        barrier();
        barrier();
    }
    for (int i = 0; i < 2; i++) pthread_join(t[i], NULL);

    for (int m = 0; m < 2; m++)
        printf("{\"entso\":%s,\"iterations\":%llu,\"forbidden_outcomes\":%llu,"
               "\"vcpu_exits\":%llu,\"actlr_mismatch\":%llu}\n",
               m ? "true" : "false", (unsigned long long)w[1].iters[m],
               (unsigned long long)w[1].violations[m],
               (unsigned long long)(w[0].exits[m] + w[1].exits[m]),
               (unsigned long long)(w[0].actlr_bad[m] + w[1].actlr_bad[m]));
    printf("{\"fail\":\"%s%s%s\",\"other_exits\":%llu}\n", w[0].fail, w[0].fail[0] ? "; " : "",
           w[1].fail, (unsigned long long)(w[0].other_exits + w[1].other_exits));
    hv_vm_destroy();
    return (w[0].fail[0] || w[1].fail[0]) ? 1 : 0;
}
