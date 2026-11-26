// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* tls_free_reach.c — what TlsFree and TlsAlloc do to another thread's value at one index (Sprint 347; reviewer-4's
 * fifteenth review, Low 1).
 *
 * No C runtime. The main thread allocates a TLS index; a worker sets its own value there and so does the main thread.
 * The main thread frees the index, and the worker reads its value; the main thread allocates again, notes whether it
 * got the same index, and the worker reads again. The same is done at an index past the 64 TEB slots, in the
 * expansion array. Microsoft documents TlsAlloc as "When a TLS index is allocated, its storage slots are initialized to
 * NULL" and says nothing of what TlsFree does to other threads' values; the rows record both on Windows. Each step
 * waits on an event, so the rows do not depend on scheduling.
 */
#include <windows.h>

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

#define WORKER_VALUE 0xB0Bu
#define MAIN_VALUE 0xA11u

static DWORD index_under_test;
static HANDLE worker_set, index_freed, worker_read, index_again, worker_done;
static ULONG_PTR after_free, after_free_error, after_again;

static DWORD WINAPI worker(LPVOID param) {
    (void)param;
    TlsSetValue(index_under_test, (LPVOID)(ULONG_PTR)WORKER_VALUE);
    SetEvent(worker_set);
    WaitForSingleObject(index_freed, INFINITE);
    SetLastError(0);
    after_free = (ULONG_PTR)TlsGetValue(index_under_test);
    after_free_error = GetLastError();
    SetEvent(worker_read);
    WaitForSingleObject(index_again, INFINITE);
    after_again = (ULONG_PTR)TlsGetValue(index_under_test);
    SetEvent(worker_done);
    return 0;
}

/* One pass at `index`: the worker's and the main thread's values after the free and after the index is taken again. */
static void pass(const char *name, DWORD index) {
    index_under_test = index;
    HANDLE *events[] = {&worker_set, &index_freed, &worker_read, &index_again, &worker_done};
    for (int k = 0; k < 5; k++) *events[k] = CreateEventW(NULL, FALSE, FALSE, NULL);
    DWORD tid;
    HANDLE h = CreateThread(NULL, 0, worker, NULL, 0, &tid);
    TlsSetValue(index, (LPVOID)(ULONG_PTR)MAIN_VALUE);
    WaitForSingleObject(worker_set, INFINITE);
    BOOL freed = TlsFree(index);
    SetEvent(index_freed);
    WaitForSingleObject(worker_read, INFINITE);
    DWORD again = TlsAlloc();
    ULONG_PTR main_after_again = (ULONG_PTR)TlsGetValue(again);
    SetEvent(index_again);
    WaitForSingleObject(worker_done, INFINITE);
    DWORD w = WaitForSingleObject(h, INFINITE);
    row(name);
    field("index_at_least_64", index >= 64);
    field("freed", freed != 0);
    field("worker_after_free", after_free);
    field("worker_after_free_error", after_free_error);
    field("same_index_again", again == index);
    field("worker_after_again", after_again);
    field("main_after_again", main_after_again);
    field("wait", w);
    end();
}

void start(void) {
    DWORD low = TlsAlloc();
    pass("teb_slot", low);
    /* Take indices until one lies past the 64 TEB slots; that one is tested, the ones below it freed at the end. */
    DWORD held[80];
    unsigned count = 0;
    DWORD high = TLS_OUT_OF_INDEXES;
    while (count < 80) {
        DWORD x = TlsAlloc();
        if (x == TLS_OUT_OF_INDEXES) break;
        if (x >= 64) { high = x; break; }
        held[count++] = x;
    }
    row("expansion"); field("found", high != TLS_OUT_OF_INDEXES); end();
    if (high != TLS_OUT_OF_INDEXES) pass("expansion_slot", high);
    while (count) TlsFree(held[--count]);
    ExitProcess(0);
}
