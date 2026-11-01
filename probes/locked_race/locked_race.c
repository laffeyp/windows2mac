// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* locked_race.c — a misaligned locked add raced by an aligned one that shares a byte (Sprint 331; the run-loop
 * design's card 4d1).
 *
 * No C runtime. In one 64-byte-aligned line, thread A runs `lock xadd` of 1 to the dword at offset 1, ADDS times;
 * thread B runs `lock xadd` of 1 to the dword at offset 4 until A is done, counting its adds. Byte 4 is A's high byte
 * and B's low byte; A's adds never carry into it (ADDS < 2^24). x86 makes each locked access atomic at any alignment,
 * so A's low 24 bits read ADDS and B's dword reads B's count on every run. w2m runs A's misaligned form in its model
 * (`hvc #11`) as a load and a store under the runtime word, and B's aligned form in the VM as an exclusive pair, which
 * takes no word; unless the model's step stops every other vCPU, B's adds made between the step's load and its store
 * are written back over, and B's dword reads less than its count. Rows: the operands' offsets, the waits, A's sum, and
 * B's adds lost.
 */
#include <windows.h>

#define ADDS 100000u

static char out[1024];
static unsigned n;

static void put(const char *s) { while (*s && n < sizeof out - 64) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
/* Each row is written when it ends, so a fault leaves the rows before it. */
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}

static __declspec(align(64)) volatile unsigned char line[128];
static volatile LONG go, a_done;
static volatile unsigned b_count;

static DWORD WINAPI thread_a(LPVOID param) {
    (void)param;
    volatile unsigned *at = (volatile unsigned *)(line + 1);
    while (!go) SwitchToThread();
    for (unsigned i = 0; i < ADDS; i++) {
        unsigned one = 1;
        __asm__ volatile("lock xaddl %0, %1" : "+r"(one), "+m"(*at) : : "memory");
    }
    a_done = 1;
    return 0;
}

static DWORD WINAPI thread_b(LPVOID param) {
    (void)param;
    volatile unsigned *at = (volatile unsigned *)(line + 4);
    unsigned count = 0;
    while (!go) SwitchToThread();
    while (!a_done) {
        unsigned one = 1;
        __asm__ volatile("lock xaddl %0, %1" : "+r"(one), "+m"(*at) : : "memory");
        count++;
    }
    b_count = count;
    return 0;
}

void start(void) {
    row("operands"); field("a_offset", (ULONG_PTR)(line + 1) & 63); field("b_offset", (ULONG_PTR)(line + 4) & 63);
    end();
    HANDLE h[2];
    DWORD tid;
    h[0] = CreateThread(NULL, 0, thread_a, NULL, 0, &tid);
    h[1] = CreateThread(NULL, 0, thread_b, NULL, 0, &tid);
    go = 1;
    DWORD w = WaitForMultipleObjects(2, h, TRUE, INFINITE);
    row("wait"); field("ret", w); end();
    unsigned a = *(volatile unsigned *)(line + 1) & 0xFFFFFFu;
    unsigned b = *(volatile unsigned *)(line + 4);
    row("a"); field("sum", a); field("expected", ADDS); end();
    row("b"); field("lost", b_count - b); field("counted_some", b_count > 0); end();
    ExitProcess(0);
}
