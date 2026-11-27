// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* named_sync_objects.c — the named sync objects the game declares and the Windows batch run never called (Sprint 369;
 * docs/unserved-declared-imports-2026-11-27.md): mono-2.0-bdwgc.dll's CreateMutexW, OpenMutexW, OpenEventW,
 * OpenSemaphoreW and SignalObjectAndWait, UnityPlayer.dll's OpenEventA and SignalObjectAndWait. In the form of
 * probes/mutex_abandoned: no C runtime, one JSON row a line on standard output, each field a return value or a last
 * error read right after the call, each call made with the last error set to 0 first.
 *
 * Microsoft Learn, as each row expects it (probes/named_sync_objects/expected.jsonl, a cell null where the pages are
 * silent, for the run to settle; ReleaseMutex's 288 and ReleaseSemaphore's 298 are contracts/windows.sync.objects'
 * Windows rows, and a wait's 0 and 258 WaitForSingleObject's WAIT_OBJECT_0 and WAIT_TIMEOUT):
 *   - CreateMutexW (https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-createmutexw): "If the
 *     mutex is a named mutex and the object existed before this function call, the return value is a handle to the
 *     existing object, and the GetLastError function returns ERROR_ALREADY_EXISTS"; "If lpName matches the name of an
 *     existing event, semaphore, waitable timer, job, or file-mapping object, the function fails and the GetLastError
 *     function returns ERROR_INVALID_HANDLE"; "If this value is TRUE and the caller created the mutex, the calling
 *     thread obtains initial ownership of the mutex object"; "The mutex object is destroyed when its last handle has
 *     been closed."
 *   - OpenMutexW (https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-openmutexw): "Only the
 *     SYNCHRONIZE access right is required to use a mutex"; "If a named mutex does not exist, the function fails and
 *     GetLastError returns ERROR_FILE_NOT_FOUND."
 *   - OpenEventW and OpenEventA (https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-openeventw,
 *     .../nf-synchapi-openeventa): "If the function fails, the return value is NULL"; "The function succeeds only if
 *     some process has already created the event by using the CreateEvent function." The pages name no error for an
 *     absent name nor for a name of another kind: the run settles them.
 *   - OpenSemaphoreW (https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-opensemaphorew): as
 *     OpenEvent, "The function succeeds only if some process has already created the semaphore by using the
 *     CreateSemaphore function"; no error named.
 *   - CreateEventW and CreateSemaphoreW (.../nf-synchapi-createeventw, .../nf-synchapi-createsemaphorew), a name of
 *     another kind: "the function fails and the GetLastError function returns ERROR_INVALID_HANDLE. This occurs because
 *     these objects share the same namespace."
 *   - SignalObjectAndWait (https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-signalobjectandwait):
 *     "This object can be a semaphore, a mutex, or an event"; "If the handle is a mutex and the caller does not own the
 *     mutex, the function fails with ERROR_NOT_OWNER"; "If dwMilliseconds is zero, the function tests the object's
 *     state, checks for queued completion routines or APCs, and returns immediately"; WAIT_FAILED "(DWORD)0xFFFFFFFF".
 *     Its example: "dwRet = SignalObjectAndWait(hEventWorkerDone, hEventMoreWorkToDo, INFINITE, FALSE);".
 *   - Synchronization Object Security and Access Rights
 *     (https://learn.microsoft.com/en-us/windows/win32/sync/synchronization-object-security-and-access-rights):
 *     SYNCHRONIZE (0x00100000L), EVENT_ALL_ACCESS (0x1F0003), EVENT_MODIFY_STATE (0x0002), MUTEX_ALL_ACCESS
 *     (0x1F0001), SEMAPHORE_ALL_ACCESS (0x1F0003), SEMAPHORE_MODIFY_STATE (0x0002), "required for the ReleaseSemaphore
 *     function".
 *
 * Rows, in order:
 *   - "mutex": CreateMutexW owned and named ("created", "create_error"); the same name again, not owned ("again",
 *     "again_error"); OpenMutexW by name with SYNCHRONIZE alone ("opened", "open_error") and with MUTEX_ALL_ACCESS
 *     ("opened_all", "open_all_error"); whether the four handles are distinct ("distinct"); another thread's zero-timeout
 *     wait while the creator owns it ("other_owned"); the owner's zero-timeout wait through the MUTEX_ALL_ACCESS handle
 *     ("owner_wait"); ReleaseMutex through the second, the SYNCHRONIZE and the first handle ("released",
 *     "released_open", "released_extra", "release_error"); another thread's wait after ("other_free").
 *   - "mutex.absent": OpenMutexW of a name nothing holds ("opened", "error").
 *   - "event": a manual-reset event made by name, unset ("created", "create_error"); OpenEventW with EVENT_ALL_ACCESS
 *     ("opened", "open_error"); OpenEventA with SYNCHRONIZE | EVENT_MODIFY_STATE ("opened_ansi", "open_ansi_error");
 *     "distinct"; a wait on the first before ("before"); SetEvent through the second ("set"); a wait through the third
 *     ("after_set"); ResetEvent through the third ("reset"); a wait on the first ("after_reset").
 *   - "event.absent": OpenEventW and OpenEventA of a name nothing holds.
 *   - "semaphore": CreateSemaphoreW named, count 1 of 2 ("created", "create_error"); the same name again, counts 0 of 5
 *     ("again", "again_error"); OpenSemaphoreW with SEMAPHORE_ALL_ACCESS and with SYNCHRONIZE | SEMAPHORE_MODIFY_STATE
 *     ("opened", "open_error", "opened_modify", "open_modify_error"); a wait through the first open ("take"), one through
 *     the second create ("empty"); ReleaseSemaphore of 2 through the second open ("released", "previous"); of 1 through
 *     the first create, past the maximum the first create set ("over", "over_error").
 *   - "semaphore.absent": OpenSemaphoreW of a name nothing holds.
 *   - "kind": a name an object of another kind holds: CreateMutexW of the event's, OpenMutexW of the semaphore's,
 *     OpenEventW of the mutex's, OpenEventA of the semaphore's, OpenSemaphoreW of the event's, CreateEventW of the
 *     mutex's, CreateSemaphoreW of the mutex's (each a handle's presence and its last error).
 *   - "closed": every handle of the three closed, each name opened again.
 *   - "saw.event": SignalObjectAndWait of an auto-reset event, waiting 0 ms on an unset manual-reset event ("wait"), the
 *     first event's state after ("signaled"); again with the second set ("wait_set").
 *   - "saw.semaphore": a semaphore of count 0 of 1 signaled and waited on 0 ms ("wait"), then a wait on it ("after");
 *     once at its maximum, signaled ("full", "full_error") and waited on ("after_full").
 *   - "saw.mutex": an unnamed mutex made owned, signaled while waiting on a set event ("wait"), another thread's wait
 *     after ("other"); signaled again, unowned ("not_owner", "not_owner_error"); waited on by the call ("wait_mutex"),
 *     another thread's wait after ("other_owned").
 *   - "saw.thread": a wait 0 ms on a thread that has ended ("wait").
 *   - "saw.closed": a closed handle to signal ("signal", "signal_error"), and a closed handle to wait on ("wait",
 *     "wait_error"), the event to signal then waited on 0 ms ("signaled").
 *   - "saw.handoff": the page's example: a worker signals `ready` and waits INFINITE on `go`; the main thread waits up
 *     to 5000 ms on `ready` ("ready"), sets `go`, and waits up to 5000 ms for the worker's end ("ended"); the worker's
 *     answer and last error ("worker_wait", "worker_error"); `go` after ("go_after").
 */
#include <windows.h>

#define N_MUTEX L"w2m.named_sync_objects.mutex"
#define N_EVENT L"w2m.named_sync_objects.event"
#define N_EVENT_ANSI "w2m.named_sync_objects.event"
#define N_SEMAPHORE L"w2m.named_sync_objects.semaphore"
#define N_SEMAPHORE_ANSI "w2m.named_sync_objects.semaphore"
#define N_ABSENT L"w2m.named_sync_objects.absent"
#define N_ABSENT_ANSI "w2m.named_sync_objects.absent"

static char out[1024];
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

/* Another thread's zero-timeout wait on a mutex, released when it took it. */
static HANDLE probed;
static DWORD probed_ret;
static DWORD WINAPI try_wait(LPVOID param) {
    (void)param;
    probed_ret = WaitForSingleObject(probed, 0);
    if (probed_ret == WAIT_OBJECT_0 || probed_ret == WAIT_ABANDONED) ReleaseMutex(probed);
    return 0;
}
static DWORD other_wait(HANDLE h) {
    probed = h;
    HANDLE t = CreateThread(NULL, 0, try_wait, NULL, 0, NULL);
    WaitForSingleObject(t, INFINITE);
    CloseHandle(t);
    return probed_ret;
}

static DWORD WINAPI nothing(LPVOID param) {
    (void)param;
    return 0;
}

/* The page's worker: signals `ready` and waits on `go`. */
static HANDLE ready, go;
static DWORD worker_wait, worker_error;
static DWORD WINAPI worker(LPVOID param) {
    (void)param;
    SetLastError(0);
    worker_wait = SignalObjectAndWait(ready, go, INFINITE, FALSE);
    worker_error = GetLastError();
    return 0;
}

static int distinct(HANDLE *hs, int k) {
    for (int i = 0; i < k; i++) {
        if (!hs[i]) return 0;
        for (int j = 0; j < i; j++)
            if (hs[i] == hs[j]) return 0;
    }
    return 1;
}

void start(void) {
    /* the mutex */
    HANDLE m[4];
    SetLastError(0);
    m[0] = CreateMutexW(NULL, TRUE, N_MUTEX);
    DWORD m_create_error = GetLastError();
    SetLastError(0);
    m[1] = CreateMutexW(NULL, FALSE, N_MUTEX);
    DWORD m_again_error = GetLastError();
    SetLastError(0);
    m[2] = OpenMutexW(SYNCHRONIZE, FALSE, N_MUTEX);
    DWORD m_open_error = GetLastError();
    SetLastError(0);
    m[3] = OpenMutexW(MUTEX_ALL_ACCESS, FALSE, N_MUTEX);
    DWORD m_open_all_error = GetLastError();
    int m_distinct = distinct(m, 4);
    DWORD m_other_owned = other_wait(m[2]);
    DWORD m_owner_wait = WaitForSingleObject(m[3], 0);
    BOOL m_released = ReleaseMutex(m[1]);
    BOOL m_released_open = ReleaseMutex(m[2]);
    SetLastError(0);
    BOOL m_released_extra = ReleaseMutex(m[0]);
    DWORD m_release_error = GetLastError();
    DWORD m_other_free = other_wait(m[0]);

    SetLastError(0);
    HANDLE m_absent = OpenMutexW(SYNCHRONIZE, FALSE, N_ABSENT);
    DWORD m_absent_error = GetLastError();

    /* the event */
    HANDLE e[3];
    SetLastError(0);
    e[0] = CreateEventW(NULL, TRUE, FALSE, N_EVENT);
    DWORD e_create_error = GetLastError();
    SetLastError(0);
    e[1] = OpenEventW(EVENT_ALL_ACCESS, FALSE, N_EVENT);
    DWORD e_open_error = GetLastError();
    SetLastError(0);
    e[2] = OpenEventA(SYNCHRONIZE | EVENT_MODIFY_STATE, FALSE, N_EVENT_ANSI);
    DWORD e_open_ansi_error = GetLastError();
    int e_distinct = distinct(e, 3);
    DWORD e_before = WaitForSingleObject(e[0], 0);
    BOOL e_set = SetEvent(e[1]);
    DWORD e_after_set = WaitForSingleObject(e[2], 0);
    BOOL e_reset = ResetEvent(e[2]);
    DWORD e_after_reset = WaitForSingleObject(e[0], 0);

    SetLastError(0);
    HANDLE e_absent = OpenEventW(EVENT_ALL_ACCESS, FALSE, N_ABSENT);
    DWORD e_absent_error = GetLastError();
    SetLastError(0);
    HANDLE e_absent_ansi = OpenEventA(EVENT_ALL_ACCESS, FALSE, N_ABSENT_ANSI);
    DWORD e_absent_ansi_error = GetLastError();

    /* the semaphore */
    HANDLE s[4];
    SetLastError(0);
    s[0] = CreateSemaphoreW(NULL, 1, 2, N_SEMAPHORE);
    DWORD s_create_error = GetLastError();
    SetLastError(0);
    s[1] = CreateSemaphoreW(NULL, 0, 5, N_SEMAPHORE);
    DWORD s_again_error = GetLastError();
    SetLastError(0);
    s[2] = OpenSemaphoreW(SEMAPHORE_ALL_ACCESS, FALSE, N_SEMAPHORE);
    DWORD s_open_error = GetLastError();
    SetLastError(0);
    s[3] = OpenSemaphoreW(SYNCHRONIZE | SEMAPHORE_MODIFY_STATE, FALSE, N_SEMAPHORE);
    DWORD s_open_modify_error = GetLastError();
    DWORD s_take = WaitForSingleObject(s[2], 0);
    DWORD s_empty = WaitForSingleObject(s[1], 0);
    LONG s_previous = -1;
    BOOL s_released = ReleaseSemaphore(s[3], 2, &s_previous);
    SetLastError(0);
    BOOL s_over = ReleaseSemaphore(s[0], 1, NULL);
    DWORD s_over_error = GetLastError();

    SetLastError(0);
    HANDLE s_absent = OpenSemaphoreW(SEMAPHORE_ALL_ACCESS, FALSE, N_ABSENT);
    DWORD s_absent_error = GetLastError();

    /* a name of another kind */
    SetLastError(0);
    HANDLE k_mutex_event = CreateMutexW(NULL, FALSE, N_EVENT);
    DWORD k_mutex_event_error = GetLastError();
    SetLastError(0);
    HANDLE k_open_mutex_semaphore = OpenMutexW(SYNCHRONIZE, FALSE, N_SEMAPHORE);
    DWORD k_open_mutex_semaphore_error = GetLastError();
    SetLastError(0);
    HANDLE k_open_event_mutex = OpenEventW(EVENT_ALL_ACCESS, FALSE, N_MUTEX);
    DWORD k_open_event_mutex_error = GetLastError();
    SetLastError(0);
    HANDLE k_open_event_ansi_semaphore = OpenEventA(EVENT_ALL_ACCESS, FALSE, N_SEMAPHORE_ANSI);
    DWORD k_open_event_ansi_semaphore_error = GetLastError();
    SetLastError(0);
    HANDLE k_open_semaphore_event = OpenSemaphoreW(SEMAPHORE_ALL_ACCESS, FALSE, N_EVENT);
    DWORD k_open_semaphore_event_error = GetLastError();
    SetLastError(0);
    HANDLE k_event_mutex = CreateEventW(NULL, FALSE, FALSE, N_MUTEX);
    DWORD k_event_mutex_error = GetLastError();
    SetLastError(0);
    HANDLE k_semaphore_mutex = CreateSemaphoreW(NULL, 0, 1, N_MUTEX);
    DWORD k_semaphore_mutex_error = GetLastError();

    /* every handle closed, each name opened again */
    for (int i = 0; i < 4; i++) CloseHandle(m[i]);
    for (int i = 0; i < 3; i++) CloseHandle(e[i]);
    for (int i = 0; i < 4; i++) CloseHandle(s[i]);
    SetLastError(0);
    HANDLE c_mutex = OpenMutexW(SYNCHRONIZE, FALSE, N_MUTEX);
    DWORD c_mutex_error = GetLastError();
    SetLastError(0);
    HANDLE c_event = OpenEventW(EVENT_ALL_ACCESS, FALSE, N_EVENT);
    DWORD c_event_error = GetLastError();
    SetLastError(0);
    HANDLE c_semaphore = OpenSemaphoreW(SEMAPHORE_ALL_ACCESS, FALSE, N_SEMAPHORE);
    DWORD c_semaphore_error = GetLastError();

    /* SignalObjectAndWait: an event signaled */
    HANDLE a = CreateEventW(NULL, FALSE, FALSE, NULL);
    HANDLE b = CreateEventW(NULL, TRUE, FALSE, NULL);
    DWORD ev_wait = SignalObjectAndWait(a, b, 0, FALSE);
    DWORD ev_signaled = WaitForSingleObject(a, 0);
    SetEvent(b);
    DWORD ev_wait_set = SignalObjectAndWait(a, b, 0, FALSE);
    WaitForSingleObject(a, 0);                          /* `a` left unset */

    /* a semaphore signaled */
    HANDLE sem = CreateSemaphoreW(NULL, 0, 1, NULL);
    DWORD sem_wait = SignalObjectAndWait(sem, sem, 0, FALSE);
    DWORD sem_after = WaitForSingleObject(sem, 0);
    ReleaseSemaphore(sem, 1, NULL);
    SetLastError(0);
    DWORD sem_full = SignalObjectAndWait(sem, b, 0, FALSE);
    DWORD sem_full_error = GetLastError();
    DWORD sem_after_full = WaitForSingleObject(sem, 0);

    /* a mutex signaled, and waited on */
    HANDLE mu = CreateMutexW(NULL, TRUE, NULL);
    DWORD mu_wait = SignalObjectAndWait(mu, b, 0, FALSE);
    DWORD mu_other = other_wait(mu);
    SetLastError(0);
    DWORD mu_not_owner = SignalObjectAndWait(mu, b, 0, FALSE);
    DWORD mu_not_owner_error = GetLastError();
    DWORD mu_wait_mutex = SignalObjectAndWait(a, mu, 0, FALSE);
    DWORD mu_other_owned = other_wait(mu);
    ReleaseMutex(mu);
    WaitForSingleObject(a, 0);                          /* `a` left unset */

    /* a thread waited on */
    HANDLE t = CreateThread(NULL, 0, nothing, NULL, 0, NULL);
    WaitForSingleObject(t, INFINITE);
    DWORD th_wait = SignalObjectAndWait(a, t, 0, FALSE);
    WaitForSingleObject(a, 0);                          /* `a` left unset */
    CloseHandle(t);

    /* a closed handle to signal, and one to wait on */
    HANDLE gone = CreateEventW(NULL, FALSE, FALSE, NULL);
    CloseHandle(gone);
    SetLastError(0);
    DWORD cl_signal = SignalObjectAndWait(gone, b, 0, FALSE);
    DWORD cl_signal_error = GetLastError();
    SetLastError(0);
    DWORD cl_wait = SignalObjectAndWait(a, gone, 0, FALSE);
    DWORD cl_wait_error = GetLastError();
    DWORD cl_signaled = WaitForSingleObject(a, 0);

    /* the page's example: a worker in its wait before the object it waits on is set */
    ready = CreateEventW(NULL, FALSE, FALSE, NULL);
    go = CreateEventW(NULL, FALSE, FALSE, NULL);
    HANDLE w = CreateThread(NULL, 0, worker, NULL, 0, NULL);
    DWORD ho_ready = WaitForSingleObject(ready, 5000);
    SetEvent(go);
    DWORD ho_ended = WaitForSingleObject(w, 5000);
    DWORD ho_go_after = WaitForSingleObject(go, 0);
    CloseHandle(w);

    row("mutex");
    field("created", m[0] != NULL);
    field("create_error", m_create_error);
    field("again", m[1] != NULL);
    field("again_error", m_again_error);
    field("opened", m[2] != NULL);
    field("open_error", m_open_error);
    field("opened_all", m[3] != NULL);
    field("open_all_error", m_open_all_error);
    field("distinct", m_distinct);
    field("other_owned", m_other_owned);
    field("owner_wait", m_owner_wait);
    field("released", m_released != 0);
    field("released_open", m_released_open != 0);
    field("released_extra", m_released_extra != 0);
    field("release_error", m_release_error);
    field("other_free", m_other_free);
    end();
    row("mutex.absent");
    field("opened", m_absent != NULL);
    field("error", m_absent_error);
    end();
    row("event");
    field("created", e[0] != NULL);
    field("create_error", e_create_error);
    field("opened", e[1] != NULL);
    field("open_error", e_open_error);
    field("opened_ansi", e[2] != NULL);
    field("open_ansi_error", e_open_ansi_error);
    field("distinct", e_distinct);
    field("before", e_before);
    field("set", e_set != 0);
    field("after_set", e_after_set);
    field("reset", e_reset != 0);
    field("after_reset", e_after_reset);
    end();
    row("event.absent");
    field("opened", e_absent != NULL);
    field("error", e_absent_error);
    field("opened_ansi", e_absent_ansi != NULL);
    field("ansi_error", e_absent_ansi_error);
    end();
    row("semaphore");
    field("created", s[0] != NULL);
    field("create_error", s_create_error);
    field("again", s[1] != NULL);
    field("again_error", s_again_error);
    field("opened", s[2] != NULL);
    field("open_error", s_open_error);
    field("opened_modify", s[3] != NULL);
    field("open_modify_error", s_open_modify_error);
    field("distinct", distinct(s, 4));
    field("take", s_take);
    field("empty", s_empty);
    field("released", s_released != 0);
    field("previous", (unsigned long long)(long long)s_previous);
    field("over", s_over != 0);
    field("over_error", s_over_error);
    end();
    row("semaphore.absent");
    field("opened", s_absent != NULL);
    field("error", s_absent_error);
    end();
    row("kind");
    field("create_mutex_on_event", k_mutex_event != NULL);
    field("create_mutex_on_event_error", k_mutex_event_error);
    field("open_mutex_on_semaphore", k_open_mutex_semaphore != NULL);
    field("open_mutex_on_semaphore_error", k_open_mutex_semaphore_error);
    field("open_event_on_mutex", k_open_event_mutex != NULL);
    field("open_event_on_mutex_error", k_open_event_mutex_error);
    field("open_event_ansi_on_semaphore", k_open_event_ansi_semaphore != NULL);
    field("open_event_ansi_on_semaphore_error", k_open_event_ansi_semaphore_error);
    field("open_semaphore_on_event", k_open_semaphore_event != NULL);
    field("open_semaphore_on_event_error", k_open_semaphore_event_error);
    field("create_event_on_mutex", k_event_mutex != NULL);
    field("create_event_on_mutex_error", k_event_mutex_error);
    field("create_semaphore_on_mutex", k_semaphore_mutex != NULL);
    field("create_semaphore_on_mutex_error", k_semaphore_mutex_error);
    end();
    row("closed");
    field("mutex", c_mutex != NULL);
    field("mutex_error", c_mutex_error);
    field("event", c_event != NULL);
    field("event_error", c_event_error);
    field("semaphore", c_semaphore != NULL);
    field("semaphore_error", c_semaphore_error);
    end();
    row("saw.event");
    field("wait", ev_wait);
    field("signaled", ev_signaled);
    field("wait_set", ev_wait_set);
    end();
    row("saw.semaphore");
    field("wait", sem_wait);
    field("after", sem_after);
    field("full", sem_full);
    field("full_error", sem_full_error);
    field("after_full", sem_after_full);
    end();
    row("saw.mutex");
    field("wait", mu_wait);
    field("other", mu_other);
    field("not_owner", mu_not_owner);
    field("not_owner_error", mu_not_owner_error);
    field("wait_mutex", mu_wait_mutex);
    field("other_owned", mu_other_owned);
    end();
    row("saw.thread");
    field("wait", th_wait);
    end();
    row("saw.closed");
    field("signal", cl_signal);
    field("signal_error", cl_signal_error);
    field("wait", cl_wait);
    field("wait_error", cl_wait_error);
    field("signaled", cl_signaled);
    end();
    row("saw.handoff");
    field("ready", ho_ready);
    field("ended", ho_ended);
    field("worker_wait", worker_wait);
    field("worker_error", worker_error);
    field("go_after", ho_go_after);
    end();
    ExitProcess(0);
}
