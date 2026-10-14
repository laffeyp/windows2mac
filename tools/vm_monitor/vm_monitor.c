// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* vm_monitor.c — VM monitor for the w2m managed-VM back end (Sprint 030).
 *
 * Holds no policy: src/w2m/vm_backend.py builds the page tables, compiles the code, and services every
 * exit. This program maps one shared memory file into the guest at a fixed IPA, sets registers
 * on command, runs the single vCPU, and reports each exit as one line.
 *
 * Usage: vm_monitor <memfile> <ipa_base_hex> <size_hex>
 * Commands on stdin, one per line; each gets one line on stdout:
 *   sys <id_hex> <value_hex>            HV_SYS_REG_* id from hv_vcpu_types.h      -> ok | err <rc>
 *   set <pc> <cpsr> <x0> ... <x30>      33 hex values                              -> ok
 *   run <ms>                            run until an exit the host must see, or ms elapse
 *       -> exit <kind> <syndrome> <pc> <cpsr> <esr_el1> <far_el1> <elr_el1> <x0> ... <x30>
 *          kind: hvc (EC 0x16; syndrome low 16 bits = immediate), exc (other EC), cancel
 *          (time limit reached), fail (hv_vcpu_run error; syndrome = hv_return_t)
 *   vget                                -> v <q0> ... <q15> <fpcr> <fpsr>  (q as 32 hex digits, low byte last)
 *   vset <q0> ... <q15> <fpcr> <fpsr>   -> ok   (Sprint 041: XMM i lives in V i)
 *   spin <n>                            resume through n HVC exits in C   -> spin <taken> <ns>  (Sprint 048)
 *   quit
 * VTIMER exits are masked and the run continues (the guest never enables the timer).
 */
#include <Hypervisor/Hypervisor.h>
#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <pthread/qos.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <time.h>

static hv_vcpu_t vcpu;
static pthread_mutex_t mu = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t cv = PTHREAD_COND_INITIALIZER;
static int running, fired;
static struct timespec deadline;

/* Cancels the vCPU when a run passes its deadline. hv_vcpus_exit may be called from any thread
 * (hv_vcpu.h). */
static void *watchdog(void *arg) {
    (void)arg;
    pthread_mutex_lock(&mu);
    for (;;) {
        while (!running || fired) pthread_cond_wait(&cv, &mu);
        if (pthread_cond_timedwait(&cv, &mu, &deadline) == ETIMEDOUT && running && !fired) {
            fired = 1;
            hv_vcpus_exit(&vcpu, 1);
        }
    }
    return NULL;
}

static void report(const char *kind, uint64_t syndrome) {
    uint64_t pc = 0, cpsr = 0, esr = 0, far = 0, elr = 0, x;
    hv_vcpu_get_reg(vcpu, HV_REG_PC, &pc);
    hv_vcpu_get_reg(vcpu, HV_REG_CPSR, &cpsr);
    hv_vcpu_get_sys_reg(vcpu, HV_SYS_REG_ESR_EL1, &esr);
    hv_vcpu_get_sys_reg(vcpu, HV_SYS_REG_FAR_EL1, &far);
    hv_vcpu_get_sys_reg(vcpu, HV_SYS_REG_ELR_EL1, &elr);
    printf("exit %s %llx %llx %llx %llx %llx %llx", kind, (unsigned long long)syndrome,
           (unsigned long long)pc, (unsigned long long)cpsr, (unsigned long long)esr,
           (unsigned long long)far, (unsigned long long)elr);
    for (int i = 0; i <= 30; i++) {
        hv_vcpu_get_reg(vcpu, (hv_reg_t)(HV_REG_X0 + i), &x);
        printf(" %llx", (unsigned long long)x);
    }
    printf("\n");
}

static void run(long ms, hv_vcpu_exit_t *ex) {
    struct timespec now;
    clock_gettime(CLOCK_REALTIME, &now);
    pthread_mutex_lock(&mu);
    deadline.tv_sec = now.tv_sec + ms / 1000;
    deadline.tv_nsec = now.tv_nsec + (ms % 1000) * 1000000L;
    if (deadline.tv_nsec >= 1000000000L) {
        deadline.tv_sec++;
        deadline.tv_nsec -= 1000000000L;
    }
    running = 1;
    fired = 0;
    pthread_cond_signal(&cv);
    pthread_mutex_unlock(&mu);
    const char *kind = NULL;
    uint64_t syn = 0;
    while (!kind) {
        hv_return_t r = hv_vcpu_run(vcpu);
        if (r != HV_SUCCESS) {
            kind = "fail";
            syn = (uint64_t)r;
        } else if (ex->reason == HV_EXIT_REASON_VTIMER_ACTIVATED) {
            hv_vcpu_set_vtimer_mask(vcpu, true);
        } else if (ex->reason == HV_EXIT_REASON_CANCELED) {
            pthread_mutex_lock(&mu);
            if (fired) kind = "cancel";
            pthread_mutex_unlock(&mu);
        } else if (ex->reason == HV_EXIT_REASON_EXCEPTION) {
            syn = ex->exception.syndrome;
            kind = ((syn >> 26) & 0x3f) == 0x16 ? "hvc" : "exc";
        } else {
            kind = "fail";
            syn = 0x10000u | ex->reason;
        }
    }
    pthread_mutex_lock(&mu);
    running = 0;
    pthread_mutex_unlock(&mu);
    report(kind, syn);
}

int main(int argc, char **argv) {
    if (argc != 4) {
        fprintf(stderr, "usage: vm_monitor <memfile> <ipa_base_hex> <size_hex>\n");
        return 2;
    }
    uint64_t ipa = strtoull(argv[2], NULL, 16), size = strtoull(argv[3], NULL, 16);
    int fd = open(argv[1], O_RDWR);
    if (fd < 0) {
        perror("open");
        return 1;
    }
    void *mem = mmap(NULL, size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    if (mem == MAP_FAILED) {
        perror("mmap");
        return 1;
    }
    /* the vCPU runs on this thread: user-interactive QoS keeps it on the performance cores when the Mac is loaded
     * (Sprint 068 item F: at the default QoS a loaded Mac doubled a census's CPU time, 81 s to 146 s, the same work) */
    pthread_set_qos_class_self_np(QOS_CLASS_USER_INTERACTIVE, 0);
    hv_vm_config_t cfg = hv_vm_config_create();
    hv_return_t r;
    /* 40-bit IPA (1 TB; TCR.IPS 2 in vm_backend.py): the VirtualAlloc arena is 512 GB of sparse file */
    if ((r = hv_vm_config_set_ipa_granule(cfg, HV_IPA_GRANULE_4KB)) || (r = hv_vm_config_set_ipa_size(cfg, 40)) ||
        (r = hv_vm_create(cfg)) ||
        (r = hv_vm_map(mem, ipa, size, HV_MEMORY_READ | HV_MEMORY_WRITE | HV_MEMORY_EXEC))) {
        fprintf(stderr, "vm setup failed: hv_return 0x%x\n", r);
        return 1;
    }
    hv_vcpu_exit_t *ex;
    if ((r = hv_vcpu_create(&vcpu, &ex, NULL))) {
        fprintf(stderr, "vcpu_create failed: hv_return 0x%x\n", r);
        return 1;
    }
    pthread_t wd;
    pthread_create(&wd, NULL, watchdog, NULL);
    setvbuf(stdout, NULL, _IOLBF, 0);
    char line[4096];
    while (fgets(line, sizeof line, stdin)) {
        char *p = line;
        if (!strncmp(p, "sys ", 4)) {
            uint64_t id = strtoull(p + 4, &p, 16), v = strtoull(p, &p, 16);
            r = hv_vcpu_set_sys_reg(vcpu, (hv_sys_reg_t)id, v);
            if (r) printf("err %x\n", r);
            else printf("ok\n");
        } else if (!strncmp(p, "set ", 4)) {
            p += 4;
            hv_vcpu_set_reg(vcpu, HV_REG_PC, strtoull(p, &p, 16));
            hv_vcpu_set_reg(vcpu, HV_REG_CPSR, strtoull(p, &p, 16));
            for (int i = 0; i <= 30; i++)
                hv_vcpu_set_reg(vcpu, (hv_reg_t)(HV_REG_X0 + i), strtoull(p, &p, 16));
            printf("ok\n");
        } else if (!strncmp(p, "vget", 4)) {
            printf("v");
            for (int i = 0; i < 16; i++) {
                hv_simd_fp_uchar16_t q;
                hv_vcpu_get_simd_fp_reg(vcpu, (hv_simd_fp_reg_t)(HV_SIMD_FP_REG_Q0 + i), &q);
                unsigned char b[16];
                memcpy(b, &q, 16);
                printf(" ");
                for (int k = 15; k >= 0; k--) printf("%02x", b[k]);
            }
            uint64_t fpcr = 0, fpsr = 0;
            hv_vcpu_get_reg(vcpu, HV_REG_FPCR, &fpcr);
            hv_vcpu_get_reg(vcpu, HV_REG_FPSR, &fpsr);
            printf(" %llx %llx\n", (unsigned long long)fpcr, (unsigned long long)fpsr);
        } else if (!strncmp(p, "vset ", 5)) {
            p += 5;
            int bad = 0;
            for (int i = 0; i < 16 && !bad; i++) {
                while (*p == ' ') p++;
                unsigned char b[16];
                for (int k = 15; k >= 0; k--) {
                    unsigned v;
                    if (sscanf(p, "%2x", &v) != 1) { bad = 1; break; }
                    b[k] = (unsigned char)v;
                    p += 2;
                }
                hv_simd_fp_uchar16_t q;
                memcpy(&q, b, 16);
                if (!bad) hv_vcpu_set_simd_fp_reg(vcpu, (hv_simd_fp_reg_t)(HV_SIMD_FP_REG_Q0 + i), q);
            }
            uint64_t fpcr = strtoull(p, &p, 16), fpsr = strtoull(p, &p, 16);
            hv_return_t r1 = hv_vcpu_set_reg(vcpu, HV_REG_FPCR, fpcr), r2 = hv_vcpu_set_reg(vcpu, HV_REG_FPSR, fpsr);
            if (bad || r1 || r2) printf("err vset %d %x %x\n", bad, r1, r2);
            else printf("ok\n");
        } else if (!strncmp(p, "vswap ", 6)) {
            /* a thread switch's vectors (Sprint 068 item B): Q0-Q15, FPCR and FPSR to the outgoing thread's slot at
             * file offset `save`, then from the incoming thread's slot at `load`; 0 skips either. A slot is 16
             * 16-byte registers, then FPCR and FPSR as 8 bytes each (vm_backend.py VECTOR_SLOT). The host sends this
             * in the same write as the next `set`, so a switch adds no round trip and no hex. */
            uint64_t save = strtoull(p + 6, &p, 16), load = strtoull(p, &p, 16);
            if ((save && save + 288 > size) || (load && load + 288 > size)) {
                printf("err vswap range\n");
                continue;
            }
            hv_return_t bad = 0;
            if (save) {
                unsigned char *s = (unsigned char *)mem + save;
                for (int i = 0; i < 16; i++) {
                    hv_simd_fp_uchar16_t q;
                    bad |= hv_vcpu_get_simd_fp_reg(vcpu, (hv_simd_fp_reg_t)(HV_SIMD_FP_REG_Q0 + i), &q);
                    memcpy(s + 16 * i, &q, 16);
                }
                uint64_t fpcr = 0, fpsr = 0;
                bad |= hv_vcpu_get_reg(vcpu, HV_REG_FPCR, &fpcr) | hv_vcpu_get_reg(vcpu, HV_REG_FPSR, &fpsr);
                memcpy(s + 256, &fpcr, 8);
                memcpy(s + 264, &fpsr, 8);
            }
            if (load) {
                unsigned char *s = (unsigned char *)mem + load;
                for (int i = 0; i < 16; i++) {
                    hv_simd_fp_uchar16_t q;
                    memcpy(&q, s + 16 * i, 16);
                    bad |= hv_vcpu_set_simd_fp_reg(vcpu, (hv_simd_fp_reg_t)(HV_SIMD_FP_REG_Q0 + i), q);
                }
                uint64_t fpcr, fpsr;
                memcpy(&fpcr, s + 256, 8);
                memcpy(&fpsr, s + 264, 8);
                bad |= hv_vcpu_set_reg(vcpu, HV_REG_FPCR, fpcr) | hv_vcpu_set_reg(vcpu, HV_REG_FPSR, fpsr);
            }
            if (bad) printf("err vswap %x\n", bad);
            else printf("ok\n");
        } else if (!strncmp(p, "run ", 4)) {
            run(strtol(p + 4, NULL, 10), ex);
        } else if (!strncmp(p, "spin ", 5)) {
            /* the raw cost of a VM exit (Sprint 048; review 2026-10-10 F1): run until n HVC exits, resuming each in
             * C with no reply on the pipe; prints the exits taken and the nanoseconds (CLOCK_MONOTONIC_RAW) */
            long want = strtol(p + 5, NULL, 10), took = 0;
            struct timespec t0, t1;
            clock_gettime(CLOCK_MONOTONIC_RAW, &t0);
            while (took < want) {
                hv_return_t r = hv_vcpu_run(vcpu);
                if (r != HV_SUCCESS || ex->reason != HV_EXIT_REASON_EXCEPTION ||
                    ((ex->exception.syndrome >> 26) & 0x3f) != 0x16) break;
                took++;
            }
            clock_gettime(CLOCK_MONOTONIC_RAW, &t1);
            printf("spin %ld %lld\n", took, (long long)((t1.tv_sec - t0.tv_sec) * 1000000000LL + (t1.tv_nsec - t0.tv_nsec)));
        } else if (!strncmp(p, "quit", 4)) {
            break;
        } else {
            printf("err bad-command\n");
        }
    }
    hv_vcpu_destroy(vcpu);
    hv_vm_destroy();
    return 0;
}
