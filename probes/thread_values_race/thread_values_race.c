// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* thread_values_race.c — two threads' own values, each set and read back while the other does the same (Sprint 344;
 * reviewer-4's thirteenth review, Medium 1).
 *
 * No C runtime. Threads A and B each run ROUNDS rounds at once. In round i, a thread sets its last error to its base
 * plus i and reads it back, sets one TLS slot, which both use, to its base plus i and reads it back, and reads its own
 * thread id. Windows keeps each value in the calling thread's own TEB, so no thread reads the other's value: each
 * mismatch count is 0 on every run. w2m serves these calls with no runtime word (lock class none, Sprint 342), so its
 * bridge and service must reach only the calling vCPU's own state block and its thread's TEB; a store through
 * another vCPU's page shows here as a mismatch. Each round a thread also reads the other's round count and counts the
 * rounds in which it moved, which shows how far the two loops overlapped. Rows: each thread's mismatches by value,
 * the wait, and the overlap.
 */
#include <windows.h>

#define ROUNDS 200000u
#define BASE_A 0x10000000u
#define BASE_B 0x20000000u

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

struct side {
    DWORD base;
    volatile unsigned round;        /* this thread's rounds done, read by the other */
    unsigned last_error_wrong, tls_wrong, tid_wrong, other_moved;
};

static DWORD slot;
static volatile LONG go;
static struct side sides[2] = {{BASE_A, 0, 0, 0, 0, 0}, {BASE_B, 0, 0, 0, 0, 0}};

static DWORD WINAPI run_side(LPVOID param) {
    struct side *me = &sides[(ULONG_PTR)param];
    struct side *other = &sides[1 - (ULONG_PTR)param];
    DWORD tid = GetCurrentThreadId();
    unsigned seen = other->round;
    while (!go) SwitchToThread();
    for (unsigned i = 0; i < ROUNDS; i++) {
        DWORD v = me->base + i;
        SetLastError(v);
        if (GetLastError() != v) me->last_error_wrong++;
        TlsSetValue(slot, (LPVOID)(ULONG_PTR)v);
        if ((ULONG_PTR)TlsGetValue(slot) != v) me->tls_wrong++;
        if (GetCurrentThreadId() != tid) me->tid_wrong++;
        unsigned now = other->round;
        if (now != seen) { me->other_moved++; seen = now; }
        me->round = i + 1;
    }
    return 0;
}

void start(void) {
    slot = TlsAlloc();
    row("slot"); field("allocated", slot != TLS_OUT_OF_INDEXES); end();
    HANDLE h[2];
    DWORD tid;
    h[0] = CreateThread(NULL, 0, run_side, (LPVOID)0, 0, &tid);
    h[1] = CreateThread(NULL, 0, run_side, (LPVOID)1, 0, &tid);
    go = 1;
    DWORD w = WaitForMultipleObjects(2, h, TRUE, INFINITE);
    row("wait"); field("ret", w); end();
    static const char *const names[2] = {"a", "b"};
    for (int k = 0; k < 2; k++) {
        row(names[k]);
        field("rounds", sides[k].round);
        field("last_error_wrong", sides[k].last_error_wrong);
        field("tls_wrong", sides[k].tls_wrong);
        field("tid_wrong", sides[k].tid_wrong);
        end();
    }
    /* How far the loops overlapped: not compared with Windows' rows, its count read by the test. */
    row("overlap"); field("a_saw_b_move", sides[0].other_moved); field("b_saw_a_move", sides[1].other_moved); end();
    ExitProcess(0);
}
