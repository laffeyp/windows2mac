// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* wait_closed_kinds.c — WaitForMultipleObjects of a closed handle, and waits of other kinds whose handle is closed
 * while they wait (Sprint 352, card 2 of docs/design-handles-and-sync-objects-2026-11-26.md).
 *
 * No C runtime. Rows, in order:
 *   - "any_closed", "all_closed" (defined): an auto-reset event made and set, a second event made and closed, then
 *     WaitForMultipleObjects with a zero timeout of {closed, set} for any and of {set, closed} for all; its return and
 *     last error, and a zero-timeout wait on the set event after it ("after"). Microsoft Learn, WaitForMultipleObjects:
 *     WAIT_FAILED, "The function has failed. To get extended error information, call GetLastError." No handle is made
 *     between the close and the waits, so the closed value names nothing.
 *   - "main" (defined): the main thread's calls in the two waits below, each on a handle no other thread closes.
 *   - "manual_worker" (recorded, not compared): a worker waits 2000 ms on a manual-reset event; the main thread waits
 *     for it to begin, sleeps 200 ms, duplicates the event's handle, closes the original and sets the event through the
 *     duplicate. Microsoft Learn, WaitForSingleObject: "If this handle is closed while the wait is still pending, the
 *     function's behavior is undefined."
 *   - "any_worker" (recorded, not compared): a worker waits 2000 ms for any of two manual-reset events; the main thread
 *     closes the first's only handle and sets the second. WaitForMultipleObjects: "If one of these handles is closed
 *     while the wait is still pending, the function's behavior is undefined."
 *   - "cycles" (defined): 600 rounds, each making an event, a semaphore and a mutex, signaling and waiting on them by
 *     WaitForSingleObject and WaitForMultipleObjects, duplicating the thread's and the process's pseudo-handles and
 *     naming them by GetThreadId and GetProcessId, and closing all five; the count of each call that answered as
 *     Windows defines it. More rounds than w2m's 256 objects: a reference a lookup kept would leave its object's entry
 *     taken, and a create would stop the run.
 */
#include <windows.h>

#define ROUNDS 600

static char out[256];
static unsigned n;

static void put(const char *s) { while (*s && n < sizeof out - 32) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}

static HANDLE ready, manual, first, second;
static DWORD manual_wait, manual_error, any_wait, any_error;

static DWORD WINAPI manual_worker(LPVOID param) {
    (void)param;
    SetEvent(ready);
    SetLastError(0);
    manual_wait = WaitForSingleObject(manual, 2000);
    manual_error = GetLastError();
    return 0;
}

static DWORD WINAPI any_worker(LPVOID param) {
    (void)param;
    HANDLE both[2];
    both[0] = first;
    both[1] = second;
    SetEvent(ready);
    SetLastError(0);
    any_wait = WaitForMultipleObjects(2, both, FALSE, 2000);
    any_error = GetLastError();
    return 0;
}

static unsigned created, set, released, all_taken, any_first, waited, reset, mutex_released, thread_named,
    process_named, closes;

static void cycle(void) {
    HANDLE e = CreateEventW(NULL, TRUE, FALSE, NULL);
    HANDLE s = CreateSemaphoreW(NULL, 0, 1, NULL);
    HANDLE m = CreateMutexA(NULL, FALSE, NULL);
    if (e && s && m) created++;
    if (SetEvent(e)) set++;
    if (ReleaseSemaphore(s, 1, NULL)) released++;
    HANDLE hs[3];
    hs[0] = e;
    hs[1] = s;
    hs[2] = m;
    if (WaitForMultipleObjects(3, hs, TRUE, 0) == WAIT_OBJECT_0) all_taken++;    /* the unit and the mutex taken */
    if (WaitForMultipleObjects(2, hs, FALSE, 0) == WAIT_OBJECT_0) any_first++;   /* the event, manual, still set */
    if (WaitForSingleObject(e, 0) == WAIT_OBJECT_0) waited++;
    if (ResetEvent(e)) reset++;
    if (ReleaseMutex(m)) mutex_released++;
    HANDLE t = NULL, p = NULL;
    DuplicateHandle(GetCurrentProcess(), GetCurrentThread(), GetCurrentProcess(), &t, 0, FALSE, DUPLICATE_SAME_ACCESS);
    DuplicateHandle(GetCurrentProcess(), GetCurrentProcess(), GetCurrentProcess(), &p, 0, FALSE, DUPLICATE_SAME_ACCESS);
    if (t && GetThreadId(t) == GetCurrentThreadId()) thread_named++;
    if (p && GetProcessId(p) == GetCurrentProcessId()) process_named++;
    closes += (CloseHandle(e) != 0) + (CloseHandle(s) != 0) + (CloseHandle(m) != 0) + (CloseHandle(t) != 0) +
              (CloseHandle(p) != 0);
}

void start(void) {
    DWORD tid;

    HANDLE valid = CreateEventW(NULL, FALSE, TRUE, NULL);
    HANDLE closed = CreateEventW(NULL, FALSE, FALSE, NULL);
    CloseHandle(closed);
    HANDLE hs[2];
    hs[0] = closed;
    hs[1] = valid;
    SetLastError(0);
    DWORD any = WaitForMultipleObjects(2, hs, FALSE, 0);
    DWORD any_err = GetLastError();
    DWORD any_after = WaitForSingleObject(valid, 0);
    SetEvent(valid);
    hs[0] = valid;
    hs[1] = closed;
    SetLastError(0);
    DWORD all = WaitForMultipleObjects(2, hs, TRUE, 0);
    DWORD all_err = GetLastError();
    DWORD all_after = WaitForSingleObject(valid, 0);
    CloseHandle(valid);

    ready = CreateEventW(NULL, FALSE, FALSE, NULL);
    manual = CreateEventW(NULL, TRUE, FALSE, NULL);
    HANDLE thread = CreateThread(NULL, 0, manual_worker, NULL, 0, &tid);
    WaitForSingleObject(ready, INFINITE);
    Sleep(200);
    HANDLE copy = NULL;
    BOOL duplicated = DuplicateHandle(GetCurrentProcess(), manual, GetCurrentProcess(), &copy, 0, FALSE,
                                      DUPLICATE_SAME_ACCESS);
    BOOL closed1 = CloseHandle(manual);
    BOOL set1 = SetEvent(copy);
    DWORD ended1 = WaitForSingleObject(thread, INFINITE);
    CloseHandle(thread);
    CloseHandle(copy);

    first = CreateEventW(NULL, TRUE, FALSE, NULL);
    second = CreateEventW(NULL, TRUE, FALSE, NULL);
    thread = CreateThread(NULL, 0, any_worker, NULL, 0, &tid);
    WaitForSingleObject(ready, INFINITE);
    Sleep(200);
    BOOL closed2 = CloseHandle(first);
    BOOL set2 = SetEvent(second);
    DWORD ended2 = WaitForSingleObject(thread, INFINITE);
    CloseHandle(thread);
    CloseHandle(second);

    for (int k = 0; k < ROUNDS; k++) cycle();

    row("any_closed");
    field("wait", any);
    field("error", any_err);
    field("after", any_after);
    end();
    row("all_closed");
    field("wait", all);
    field("error", all_err);
    field("after", all_after);
    end();
    row("main");
    field("duplicated", duplicated != 0);
    field("closed", closed1 != 0);
    field("set", set1 != 0);
    field("ended", ended1);
    field("closed_first", closed2 != 0);
    field("set_second", set2 != 0);
    field("ended_any", ended2);
    end();
    row("manual_worker");
    field("wait", manual_wait);
    field("error", manual_error);
    end();
    row("any_worker");
    field("wait", any_wait);
    field("error", any_error);
    end();
    row("cycles");
    field("created", created);
    field("set", set);
    field("released", released);
    field("all_taken", all_taken);
    field("any_first", any_first);
    field("waited", waited);
    field("reset", reset);
    field("mutex_released", mutex_released);
    field("thread_named", thread_named);
    field("process_named", process_named);
    field("closes", closes);
    end();
    ExitProcess(0);
}
