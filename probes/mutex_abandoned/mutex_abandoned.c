// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* mutex_abandoned.c — waits on a mutex whose owner thread ended holding it (Sprint 356's amendment; reviewer-4's
 * twenty-sixth review, Medium 1). Each mutex below is unnamed and made unowned; a worker takes it by
 * WaitForSingleObject and returns from its thread function holding it, so the mutex is abandoned when the worker ends.
 * Microsoft Learn, WaitForSingleObject, WAIT_ABANDONED 0x00000080L: "The specified object is a mutex object that was
 * not released by the thread that owned the mutex object before the owning thread terminated. Ownership of the mutex
 * object is granted to the calling thread and the mutex state is set to nonsignaled."
 *
 * No C runtime. Rows, in order, each field a return value or a last error read right after the call (each call made
 * with the last error set to 0 first):
 *   - "single": the main thread waits with a zero timeout on the abandoned mutex ("wait", "error"), waits on it again,
 *     owning it ("again"), releases it twice ("released", "released_again", "release_error" after the second), then
 *     waits once more ("after", the abandoned state cleared by the release) and releases ("after_released").
 *   - "any": WaitForMultipleObjects for any, zero timeout, of {an unsignaled event, an abandoned mutex} ("wait",
 *     "error"), and the mutex released by the main thread after ("released").
 *   - "all": WaitForMultipleObjects for all, zero timeout, of {a set manual-reset event, an abandoned mutex} ("wait",
 *     "error"), the mutex released after ("released"), and a second wait for all of the same two after the release
 *     ("after").
 *   - "blocked": a worker waits with no timeout on a mutex another worker holds; the holder ends holding it. The
 *     waiter's answer and last error ("wait", "error"), whether the waiter ended inside 5000 ms of the holder's end
 *     ("ended": 0, else 258), and its ReleaseMutex before it ends ("released").
 *   - "not_owner": the main thread's ReleaseMutex of a mutex a running worker holds ("released", "error"), then the
 *     worker's own ReleaseMutex ("owner_released") and the main thread's zero-timeout wait after it ("after").
 */
#include <windows.h>

static char out[512];
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

static HANDLE held, ready, go;
static DWORD blocked_wait, blocked_error, blocked_released, owner_released;

/* Takes `held` and returns holding it. */
static DWORD WINAPI abandon(LPVOID param) {
    (void)param;
    WaitForSingleObject(held, INFINITE);
    return 0;
}

/* Takes `held`, says so, waits for `go`, and returns holding it. */
static DWORD WINAPI abandon_later(LPVOID param) {
    (void)param;
    WaitForSingleObject(held, INFINITE);
    SetEvent(ready);
    WaitForSingleObject(go, INFINITE);
    return 0;
}

/* Says it is about to wait, waits on `held` with no timeout, and releases it. */
static DWORD WINAPI waiter(LPVOID param) {
    (void)param;
    SetEvent(ready);
    SetLastError(0);
    blocked_wait = WaitForSingleObject(held, INFINITE);
    blocked_error = GetLastError();
    blocked_released = ReleaseMutex(held) != 0;
    return 0;
}

/* Takes `held`, says so, waits for `go`, then releases it. */
static DWORD WINAPI owner(LPVOID param) {
    (void)param;
    WaitForSingleObject(held, INFINITE);
    SetEvent(ready);
    WaitForSingleObject(go, INFINITE);
    owner_released = ReleaseMutex(held) != 0;
    return 0;
}

/* A new mutex, abandoned: a worker takes it and ends holding it. */
static HANDLE abandoned(void) {
    held = CreateMutexA(NULL, FALSE, NULL);
    HANDLE t = CreateThread(NULL, 0, abandon, NULL, 0, NULL);
    WaitForSingleObject(t, INFINITE);
    CloseHandle(t);
    return held;
}

void start(void) {
    ready = CreateEventW(NULL, FALSE, FALSE, NULL);
    go = CreateEventW(NULL, TRUE, FALSE, NULL);

    HANDLE m = abandoned();
    SetLastError(0);
    DWORD single_wait = WaitForSingleObject(m, 0);
    DWORD single_error = GetLastError();
    DWORD single_again = WaitForSingleObject(m, 0);
    BOOL single_released = ReleaseMutex(m);
    SetLastError(0);
    BOOL single_released_again = ReleaseMutex(m);
    DWORD single_release_error = GetLastError();
    DWORD single_after = WaitForSingleObject(m, 0);
    BOOL single_after_released = ReleaseMutex(m);
    CloseHandle(m);

    HANDLE quiet = CreateEventW(NULL, TRUE, FALSE, NULL);
    m = abandoned();
    HANDLE hs[2];
    hs[0] = quiet;
    hs[1] = m;
    SetLastError(0);
    DWORD any_wait = WaitForMultipleObjects(2, hs, FALSE, 0);
    DWORD any_error = GetLastError();
    BOOL any_released = ReleaseMutex(m);
    CloseHandle(m);
    CloseHandle(quiet);

    HANDLE set = CreateEventW(NULL, TRUE, TRUE, NULL);
    m = abandoned();
    hs[0] = set;
    hs[1] = m;
    SetLastError(0);
    DWORD all_wait = WaitForMultipleObjects(2, hs, TRUE, 0);
    DWORD all_error = GetLastError();
    BOOL all_released = ReleaseMutex(m);
    DWORD all_after = WaitForMultipleObjects(2, hs, TRUE, 0);
    ReleaseMutex(m);
    CloseHandle(m);
    CloseHandle(set);

    held = CreateMutexA(NULL, FALSE, NULL);
    HANDLE holder = CreateThread(NULL, 0, abandon_later, NULL, 0, NULL);
    WaitForSingleObject(ready, INFINITE);
    HANDLE w = CreateThread(NULL, 0, waiter, NULL, 0, NULL);
    WaitForSingleObject(ready, INFINITE);
    Sleep(200);                                     /* the waiter is in its wait */
    SetEvent(go);
    WaitForSingleObject(holder, INFINITE);
    DWORD blocked_ended = WaitForSingleObject(w, 5000);
    CloseHandle(holder);
    CloseHandle(w);
    CloseHandle(held);
    ResetEvent(go);

    held = CreateMutexA(NULL, FALSE, NULL);
    HANDLE o = CreateThread(NULL, 0, owner, NULL, 0, NULL);
    WaitForSingleObject(ready, INFINITE);
    SetLastError(0);
    BOOL not_owner_released = ReleaseMutex(held);
    DWORD not_owner_error = GetLastError();
    SetEvent(go);
    WaitForSingleObject(o, INFINITE);
    DWORD not_owner_after = WaitForSingleObject(held, 0);
    ReleaseMutex(held);
    CloseHandle(o);
    CloseHandle(held);

    row("single");
    field("wait", single_wait);
    field("error", single_error);
    field("again", single_again);
    field("released", single_released != 0);
    field("released_again", single_released_again != 0);
    field("release_error", single_release_error);
    field("after", single_after);
    field("after_released", single_after_released != 0);
    end();
    row("any");
    field("wait", any_wait);
    field("error", any_error);
    field("released", any_released != 0);
    end();
    row("all");
    field("wait", all_wait);
    field("error", all_error);
    field("released", all_released != 0);
    field("after", all_after);
    end();
    row("blocked");
    field("wait", blocked_wait);
    field("error", blocked_error);
    field("ended", blocked_ended);
    field("released", blocked_released);
    end();
    row("not_owner");
    field("released", not_owner_released != 0);
    field("error", not_owner_error);
    field("owner_released", owner_released);
    field("after", not_owner_after);
    end();
    ExitProcess(0);
}
