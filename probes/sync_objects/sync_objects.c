/* sync_objects.c — semaphores, events and mutexes on one thread (Sprint 046, family "sync objects").
 *
 * No C runtime; one thread. UnityPlayer.dll's DllMain creates a semaphore (CreateSemaphoreExW; frontier
 * 2026-10-10, 278,336 instructions); it imports CreateEventW, CreateEventExW, CreateMutexA, SetEvent,
 * ResetEvent, ReleaseSemaphore, WaitForSingleObject(Ex), WaitForMultipleObjects. Rows: each call's return
 * and last error (after SetLastError(0xDEAD)); handle values as relations (non-NULL, a multiple of 4,
 * distinct). Waits use a zero timeout: a wait that would block is another thread's (Sprint 047).
 */
#include <windows.h>

#define SENTINEL 0xDEADu

static char out[8192];
static unsigned n;

static void put(const char *s) { while (*s) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void end(void) { put("}\n"); }

static DWORD err;

static void handle_row(const char *name, HANDLE h) {
    row(name); flag("non_null", h != 0); flag("multiple_of_4", ((ULONG_PTR)h & 3) == 0); field("err", err); end();
}

static void wait_row(const char *name, HANDLE h) {
    SetLastError(SENTINEL);
    DWORD r = WaitForSingleObject(h, 0);
    err = GetLastError();
    row(name); field("ret", r); field("err", err); end();
}

static void bool_row(const char *name, BOOL ok) { row(name); field("ret", ok != 0); field("err", err); end(); }

void start(void) {
    SetLastError(SENTINEL);
    HANDLE sem = CreateSemaphoreExW(NULL, 1, 2, NULL, 0, SEMAPHORE_ALL_ACCESS);
    err = GetLastError();
    handle_row("semaphore.create", sem);
    wait_row("semaphore.wait_available", sem);
    wait_row("semaphore.wait_empty", sem);
    LONG prev = -7;
    SetLastError(SENTINEL);
    BOOL ok = ReleaseSemaphore(sem, 1, &prev);
    err = GetLastError();
    row("semaphore.release"); field("ret", ok != 0); field("err", err); field("previous", (unsigned long long)prev); end();
    prev = -7;
    SetLastError(SENTINEL);
    ok = ReleaseSemaphore(sem, 5, &prev);
    err = GetLastError();
    row("semaphore.release_past_max"); field("ret", ok != 0); field("err", err); flag("previous_untouched", prev == -7); end();
    wait_row("semaphore.wait_after_release", sem);

    SetLastError(SENTINEL);
    HANDLE manual = CreateEventW(NULL, TRUE, FALSE, NULL);
    err = GetLastError();
    handle_row("event.manual.create", manual);
    wait_row("event.manual.wait_unset", manual);
    SetLastError(SENTINEL); ok = SetEvent(manual); err = GetLastError(); bool_row("event.manual.set", ok);
    wait_row("event.manual.wait_set", manual);
    wait_row("event.manual.wait_set_again", manual);
    SetLastError(SENTINEL); ok = ResetEvent(manual); err = GetLastError(); bool_row("event.manual.reset", ok);
    wait_row("event.manual.wait_reset", manual);

    SetLastError(SENTINEL);
    HANDLE automatic = CreateEventExW(NULL, NULL, CREATE_EVENT_INITIAL_SET, EVENT_ALL_ACCESS);
    err = GetLastError();
    handle_row("event.auto.create_set", automatic);
    wait_row("event.auto.wait_set", automatic);
    wait_row("event.auto.wait_consumed", automatic);

    SetLastError(SENTINEL);
    HANDLE mutex = CreateMutexA(NULL, FALSE, NULL);
    err = GetLastError();
    handle_row("mutex.create", mutex);
    wait_row("mutex.wait_free", mutex);
    wait_row("mutex.wait_owned_recursive", mutex);
    SetLastError(SENTINEL); ok = ReleaseMutex(mutex); err = GetLastError(); bool_row("mutex.release_once", ok);
    SetLastError(SENTINEL); ok = ReleaseMutex(mutex); err = GetLastError(); bool_row("mutex.release_twice", ok);
    SetLastError(SENTINEL); ok = ReleaseMutex(mutex); err = GetLastError(); bool_row("mutex.release_unowned", ok);

    row("handles.distinct"); flag("value", sem != manual && sem != automatic && sem != mutex && manual != automatic &&
                                           manual != mutex && automatic != mutex); end();

    HANDLE both[2];
    SetEvent(manual);
    both[0] = manual;
    both[1] = sem;
    ReleaseSemaphore(sem, 0x7FFF, NULL);   /* past max: no change; the semaphore stays empty */
    SetLastError(SENTINEL);
    DWORD r = WaitForMultipleObjects(2, both, FALSE, 0);
    err = GetLastError();
    row("wait_multiple.any"); field("ret", r); field("err", err); end();
    SetLastError(SENTINEL);
    r = WaitForMultipleObjects(2, both, TRUE, 0);
    err = GetLastError();
    row("wait_multiple.all"); field("ret", r); field("err", err); end();
    SetLastError(SENTINEL);
    r = WaitForSingleObjectEx(manual, 0, FALSE);
    err = GetLastError();
    row("wait_ex.set"); field("ret", r); field("err", err); end();

    SetLastError(SENTINEL); ok = CloseHandle(mutex); err = GetLastError(); bool_row("close.mutex", ok);
    SetLastError(SENTINEL); ok = CloseHandle(mutex); err = GetLastError(); bool_row("close.again", ok);
    wait_row("wait.closed", mutex);
    CloseHandle(sem); CloseHandle(manual); CloseHandle(automatic);

    SetLastError(SENTINEL);
    Sleep(0);
    err = GetLastError();
    row("sleep.zero"); field("err", err); end();

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
