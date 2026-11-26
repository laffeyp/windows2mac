// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* thread_slots_race.c — the process's TLS and FLS slots changed by threads at once (Sprint 345; the host design's
 * Change 1, "the TLS and FLS slots' lock").
 *
 * No C runtime. Part 1: threads A and B each run ROUNDS rounds at once. In a round a thread takes HOLD TLS indices by
 * TlsAlloc, marks each as its own by an atomic compare-and-swap on a shared owner word per index, then clears each
 * mark and frees the index by TlsFree. Windows hands out a TLS index to one caller until it is freed, so no mark is
 * ever found held by the other thread: `duplicate` is 0 on every run. Each round a thread also counts the rounds in
 * which the other's round count moved, which shows how far the loops overlapped.
 *
 * Part 2: FLS_THREADS threads start at once; each makes its FLS data by its first FlsSetValue of one index, then waits.
 * The main thread frees the index, and each thread reads it back. FlsFree clears the index on every thread, so each
 * read is NULL: `stale` is 0. w2m lists each thread's FLS data on its first set (`thread_storage.c`, `fls_blocks`); a
 * link lost to two first sets at once leaves that thread's value after the free.
 *
 * Rows: part 1's indices, its duplicates and its overlap; part 2's sets, its reads after the free and the waits.
 */
#include <windows.h>

#define ROUNDS 20000u
#define HOLD 8u
#define TLS_INDICES 1088u
#define FLS_THREADS 16u

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
    volatile unsigned round;        /* this thread's rounds done, read by the other */
    unsigned out_of_indexes, out_of_range, duplicate, free_failed, other_moved;
};

static volatile LONG owner[TLS_INDICES];
static volatile LONG go;
static struct side sides[2];

static DWORD WINAPI tls_side(LPVOID param) {
    LONG me = (LONG)(ULONG_PTR)param + 1;
    struct side *s = &sides[me - 1];
    struct side *other = &sides[2 - me];
    unsigned seen = other->round;
    DWORD held[HOLD];
    while (!go) SwitchToThread();
    for (unsigned i = 0; i < ROUNDS; i++) {
        unsigned k = 0;
        for (; k < HOLD; k++) {
            DWORD x = TlsAlloc();
            if (x == TLS_OUT_OF_INDEXES) { s->out_of_indexes++; break; }
            if (x >= TLS_INDICES) { s->out_of_range++; TlsFree(x); break; }
            if (__sync_val_compare_and_swap(&owner[x], 0, me) != 0) s->duplicate++;
            held[k] = x;
        }
        while (k) {
            DWORD x = held[--k];
            __sync_val_compare_and_swap(&owner[x], me, 0);
            if (!TlsFree(x)) s->free_failed++;
        }
        unsigned now = other->round;
        if (now != seen) { s->other_moved++; seen = now; }
        s->round = i + 1;
    }
    return 0;
}

static DWORD fls_index;
static volatile LONG fls_go, fls_set;
static HANDLE fls_freed;
static ULONG_PTR fls_after[FLS_THREADS];
static BOOL fls_set_ok[FLS_THREADS];

static DWORD WINAPI fls_side(LPVOID param) {
    unsigned t = (unsigned)(ULONG_PTR)param;
    while (!fls_go) {}
    fls_set_ok[t] = FlsSetValue(fls_index, (PVOID)(ULONG_PTR)(0x1000 + t));
    __sync_fetch_and_add(&fls_set, 1);
    WaitForSingleObject(fls_freed, INFINITE);
    fls_after[t] = (ULONG_PTR)FlsGetValue(fls_index);
    return 0;
}

void start(void) {
    HANDLE h[FLS_THREADS];
    DWORD tid;
    h[0] = CreateThread(NULL, 0, tls_side, (LPVOID)0, 0, &tid);
    h[1] = CreateThread(NULL, 0, tls_side, (LPVOID)1, 0, &tid);
    go = 1;
    DWORD w = WaitForMultipleObjects(2, h, TRUE, INFINITE);
    row("tls.wait"); field("ret", w); end();
    static const char *const names[2] = {"tls.a", "tls.b"};
    for (int k = 0; k < 2; k++) {
        row(names[k]);
        field("rounds", sides[k].round);
        field("out_of_indexes", sides[k].out_of_indexes);
        field("out_of_range", sides[k].out_of_range);
        field("duplicate", sides[k].duplicate);
        field("free_failed", sides[k].free_failed);
        end();
    }
    /* How far the loops overlapped: not compared with Windows' rows, its count read by the test. */
    row("tls.overlap"); field("a_saw_b_move", sides[0].other_moved); field("b_saw_a_move", sides[1].other_moved);
    end();

    fls_index = FlsAlloc(NULL);
    fls_freed = CreateEventW(NULL, TRUE, FALSE, NULL);
    row("fls.alloc"); field("allocated", fls_index != FLS_OUT_OF_INDEXES); field("event", fls_freed != NULL); end();
    for (unsigned t = 0; t < FLS_THREADS; t++) h[t] = CreateThread(NULL, 0, fls_side, (LPVOID)(ULONG_PTR)t, 0, &tid);
    fls_go = 1;
    while (fls_set < (LONG)FLS_THREADS) SwitchToThread();
    BOOL freed = FlsFree(fls_index);
    SetEvent(fls_freed);
    w = WaitForMultipleObjects(FLS_THREADS, h, TRUE, INFINITE);
    unsigned set_ok = 0, stale = 0;
    for (unsigned t = 0; t < FLS_THREADS; t++) {
        set_ok += fls_set_ok[t] != 0;
        stale += fls_after[t] != 0;
    }
    row("fls"); field("threads", FLS_THREADS); field("set_ok", set_ok); field("freed", freed != 0);
    field("stale", stale); field("wait", w); end();
    ExitProcess(0);
}
