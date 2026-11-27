// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* apcs_and_waitable_timers.c — the asynchronous procedure calls and the waitable timer the game declares and the Windows
 * batch run never called (Sprint 370; docs/unserved-declared-imports-2026-11-27.md): mono-2.0-bdwgc.dll's QueueUserAPC,
 * UnityPlayer.dll's CreateWaitableTimerA and SetWaitableTimer, and the alertable flag of SleepEx, WaitForSingleObjectEx,
 * WaitForMultipleObjectsEx and SignalObjectAndWait, which both images import. In the form of probes/named_sync_objects:
 * no C runtime, one JSON row a line on standard output, each field a return value, a last error read right after the
 * call (the last error set to 0 first), or a count the APC functions keep.
 *
 * Microsoft Learn, as each row expects it (probes/apcs_and_waitable_timers/expected.jsonl, a cell null where the pages
 * are silent, for the run to settle; 258 is WaitForSingleObject's WAIT_TIMEOUT, 0 its WAIT_OBJECT_0):
 *   - QueueUserAPC (https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-queueuserapc):
 *     "If the function succeeds, the return value is nonzero"; "When a user-mode APC is queued, the thread is not
 *     directed to call the APC function unless it is in an alertable state. After the thread is in an alertable state,
 *     the thread handles all pending APCs in first in, first out (FIFO) order, and the wait operation returns
 *     WAIT_IO_COMPLETION. A thread enters an alertable state by using SleepEx function, SignalObjectAndWait function,
 *     WaitForSingleObjectEx function, WaitForMultipleObjectsEx function, or MsgWaitForMultipleObjectsEx function"; "If an
 *     application queues an APC before the thread begins running, the thread begins by calling the APC function" (so
 *     each worker below signals that it runs before an APC is queued to it); "When the thread is in the process of being
 *     terminated, calling QueueUserAPC to add to the thread's APC queue will fail with (31) ERROR_GEN_FAILURE" (an ended
 *     thread's and a closed handle's answers are not named: the run settles them).
 *   - Asynchronous Procedure Calls (https://learn.microsoft.com/en-us/windows/win32/sync/asynchronous-procedure-calls):
 *     "If the wait is satisfied before the APC is queued, the thread is no longer in an alertable wait state so the APC
 *     function will not be executed. However, the APC is still queued, so the APC function will be executed when the
 *     thread calls another alertable wait function."
 *   - Using a Waitable Timer with an Asynchronous Procedure Call
 *     (https://learn.microsoft.com/en-us/windows/win32/sync/using-a-waitable-timer-with-an-asynchronous-procedure-call):
 *     "An alertable function will return only after all APC entries have been processed" (row apc.self: an APC queued by
 *     an APC runs in the same wait).
 *   - SleepEx (https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-sleepex): bAlertable FALSE: "If
 *     an APC is queued to the thread, the function does not immediately return and the APC function is not executed";
 *     "The return value is zero if the specified time interval expired"; WAIT_IO_COMPLETION "0x000000C0L" (192;
 *     WaitForSingleObjectEx's table).
 *   - WaitForSingleObjectEx and WaitForMultipleObjectsEx
 *     (https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobjectex,
 *     .../nf-synchapi-waitformultipleobjectsex): "If this parameter is TRUE and the thread is in the waiting state, the
 *     function returns when the system queues an I/O completion routine or APC, and the thread runs the routine or
 *     function. Otherwise, the function does not return, and the completion routine or APC function is not executed";
 *     WAIT_IO_COMPLETION "The wait was ended by one or more user-mode asynchronous procedure calls (APC) queued to the
 *     thread." Neither page says whether a zero timeout runs a queued APC, nor which answer a signaled object and a
 *     queued APC give together: the run settles them.
 *   - SignalObjectAndWait (https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-signalobjectandwait):
 *     "If dwMilliseconds is zero, the function tests the object's state, checks for queued completion routines or APCs,
 *     and returns immediately"; bAlertable "If FALSE, the function does not return, and the thread does not call the
 *     completion routine or APC function."
 *   - CreateWaitableTimerW, which documents CreateWaitableTimerA too (Learn has no page of the A form:
 *     .../api/synchapi/nf-synchapi-createwaitabletimera and .../api/winbase/nf-winbase-createwaitabletimera answer 404;
 *     https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-createwaitabletimerw): bManualReset "If this
 *     parameter is TRUE, the timer is a manual-reset notification timer. Otherwise, the timer is a synchronization
 *     timer"; "If the named timer object exists before the function call, the function returns a handle to the existing
 *     object and GetLastError returns ERROR_ALREADY_EXISTS"; "If lpTimerName matches the name of an existing event,
 *     semaphore, mutex, job, or file-mapping object, the function fails and GetLastError returns ERROR_INVALID_HANDLE";
 *     "The timer object is destroyed when its last handle has been closed." CreateEventW and CreateMutexW name a waitable
 *     timer among the kinds whose name answers ERROR_INVALID_HANDLE (.../nf-synchapi-createeventw,
 *     .../nf-synchapi-createmutexw).
 *   - SetWaitableTimer (https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-setwaitabletimer):
 *     lpDueTime "in 100 nanosecond intervals ... Positive values indicate absolute time ... Negative values indicate
 *     relative time"; lPeriod "If lPeriod is zero, the timer is signaled once. If lPeriod is greater than zero, the timer
 *     is periodic ... If lPeriod is less than zero, the function fails" (its error not named); "If the timer is already
 *     active when you call SetWaitableTimer, the timer is stopped, then it is reactivated ... it does cancel any pending
 *     completion routines"; "When the specified due time arrives, the timer becomes inactive, and the optional APC is
 *     queued to the thread that set the timer if there is no outstanding APC already queued"; "When a manual-reset timer
 *     is set to the signaled state, it remains in this state until SetWaitableTimer is called to reset the timer ... When
 *     a synchronization timer is set to the signaled state, it remains in this state until a thread completes a wait
 *     operation on the timer object."
 *   - Waitable Timer Objects (https://learn.microsoft.com/en-us/windows/win32/sync/waitable-timer-objects): "When a timer
 *     is set, it is canceled if it was already active, the state of the timer is nonsignaled, and the timer is placed in
 *     the kernel timer queue"; "periodic timer: A timer that is reactivated each time the specified period expires".
 *   - TimerAPCProc (https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nc-synchapi-ptimerapcroutine):
 *     lpArgToCompletionRoutine "The value passed to the function using the lpArgToCompletionRoutine parameter of the
 *     SetWaitableTimer function"; dwTimerLowValue and dwTimerHighValue "the UTC-based time at which the timer was
 *     signaled"; "The completion routine is executed by the thread that activates the timer using SetWaitableTimer.
 *     However, the thread must be in an alertable state."
 *
 * Rows, in order (APCs queued to the main thread through GetCurrentThread's pseudo-handle unless a row says a worker):
 *   - "apc.self": four APCs, data 1 to 3 queued and 4 queued by the first as it runs ("queued", 1 when every
 *     QueueUserAPC answered nonzero); SleepEx(0, TRUE) ("sleep"), the order they ran in as digits ("order"), and
 *     SleepEx(0, TRUE) again with none left ("again").
 *   - "apc.unalertable": one APC queued; SleepEx(20, FALSE) ("sleep") and the calls made then ("ran_sleep");
 *     WaitForSingleObjectEx on an unset event for 20 ms, not alertable ("wait", "ran_wait"); the same alertable with a
 *     timeout of 0 ("zero", "ran_zero"); SleepEx(0, TRUE) after ("drain", "ran_drain").
 *   - "apc.signaled": one APC queued; WaitForSingleObjectEx on a set manual-reset event, timeout 0, alertable ("wait",
 *     "ran"); SleepEx(0, TRUE) after ("drain", "ran_drain").
 *   - "apc.multiple": one APC queued; WaitForMultipleObjectsEx for any of two unset events, 5000 ms, alertable ("wait",
 *     "ran").
 *   - "apc.signal_and_wait": one APC queued; SignalObjectAndWait of an auto-reset event, waiting 5000 ms alertable on an
 *     unset event ("wait", "ran"), the first event's state after ("signaled"); another APC, the same call with a timeout
 *     of 0 not alertable ("unalertable", "ran_unalertable"); SleepEx(0, TRUE) after ("drain", "ran_drain").
 *   - "apc.other": a worker in SleepEx(INFINITE, TRUE) and one APC queued to it with data 11 ("queued"): the worker's
 *     answer ("wait"), the data its APC saw ("data"), whether the APC ran on the worker ("on_target"), the worker's end
 *     ("ended").
 *   - "apc.other_waiting": a worker in WaitForSingleObjectEx(an unset event, INFINITE, TRUE): the same fields.
 *   - "apc.other_unalertable": a worker in WaitForSingleObject(an unset event, INFINITE); one APC queued to it, 100 ms
 *     slept, the calls made then ("ran_before_set"); the event set: the worker's answer ("wait"), its SleepEx(0, TRUE)
 *     after ("drain"), the calls made then ("ran_after"), the worker's end ("ended").
 *   - "apc.invalid": QueueUserAPC with a closed handle ("closed", "closed_error") and with an ended worker's handle
 *     ("ended", "ended_error").
 *   - "timer.manual": CreateWaitableTimerA(NULL, TRUE, NULL) ("created", "create_error"); a wait of 0 before any set
 *     ("inactive"); SetWaitableTimer 100 ms ahead ("set", "set_error"), a wait of 0 at once ("before"), a wait of 5000
 *     ("wait"), another of 0 ("still"); set again 100 s ahead ("reset") and a wait of 0 ("after_reset").
 *   - "timer.sync": a synchronization timer set 50 ms ahead; a wait of 5000 ("wait"), then one of 0 ("again").
 *   - "timer.periodic": a synchronization timer set 20 ms ahead with a period of 20 ms; three waits of 5000 ("first",
 *     "second", "third"); set 100 s ahead with no period ("cancel") and a wait of 0 ("after").
 *   - "timer.absolute": a manual-reset timer set by absolute time, 50 ms past GetSystemTimeAsFileTime; a wait of 5000
 *     ("wait"); a synchronization timer set 10 s in the past, a wait of 0 ("past").
 *   - "timer.multiple": a synchronization timer set 30 ms ahead and an unset event; WaitForMultipleObjects for any of
 *     {the event, the timer}, 5000 ms ("wait").
 *   - "timer.apc": a synchronization timer set 20 ms ahead with a completion routine and its argument ("set");
 *     SleepEx(5000, TRUE) ("sleep"); the routine's calls ("calls"), whether its argument was the one given ("arg_ok"),
 *     whether its time words lie within 1 s of the due time ("time_ok"); waits of 0 after ("signaled", "again").
 *   - "timer.apc_unalertable": a routine 20 ms ahead; Sleep(100), the calls then ("calls_before"); SleepEx(0, TRUE)
 *     ("sleep"), the calls then ("calls").
 *   - "timer.apc_periodic": a routine 20 ms ahead with a period of 50 ms; Sleep(300); SleepEx(0, TRUE) ("sleep"), the
 *     calls then ("calls"); set again 100 s ahead with no routine ("cancel"), SleepEx(100, TRUE) ("after"), the calls
 *     then ("calls_after").
 *   - "timer.apc_cancel": a routine 20 ms ahead, then set again 50 ms ahead with none; SleepEx(200, TRUE) ("sleep"), the
 *     calls ("calls"), a wait of 5000 ("wait").
 *   - "timer.invalid": SetWaitableTimer of an event's handle ("event", "event_error") and with a period of -1
 *     ("period", "period_error").
 *   - "timer.named": CreateWaitableTimerA(NULL, TRUE, a name) ("created", "create_error"), again with FALSE ("again",
 *     "again_error"), "distinct"; set 20 ms ahead through the first, a wait of 5000 through the second ("wait_second")
 *     and of 0 through the first ("wait_first"); CreateEventW and CreateMutexW of the timer's name, and
 *     CreateWaitableTimerA of an event's name (each a handle's presence and its last error); both timer handles closed
 *     and the name made again ("made_again", "made_again_error").
 */
#include <windows.h>

#define N_TIMER "w2m.apcs_and_waitable_timers.timer"
#define N_TIMER_W L"w2m.apcs_and_waitable_timers.timer"
#define N_EVENT_W L"w2m.apcs_and_waitable_timers.event"
#define N_EVENT "w2m.apcs_and_waitable_timers.event"
#define MS(n) (-(LONGLONG)(n) * 10000)          /* a relative due time of n ms, in 100 ns units */

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

/* The APC functions: each call counted, its data appended to `order` as a digit, the thread it ran on kept. */
static volatile LONG calls;
static volatile ULONGLONG order;
static volatile DWORD ran_on, data_seen;
static void CALLBACK note(ULONG_PTR data) {
    calls++;
    order = order * 10 + data;
    data_seen = (DWORD)data;
    ran_on = GetCurrentThreadId();
}
static int queue_ok;
static void CALLBACK note_and_queue(ULONG_PTR data) {
    note(data);
    queue_ok &= QueueUserAPC(note, GetCurrentThread(), 4) != 0;
}

/* The timer's completion routine: its calls, its argument and its time words. */
static volatile LONG timer_calls;
static volatile ULONGLONG timer_time;
static void *volatile timer_arg;
static void CALLBACK on_timer(LPVOID arg, DWORD low, DWORD high) {
    timer_calls++;
    timer_arg = arg;
    timer_time = (ULONGLONG)high << 32 | low;
}

static ULONGLONG now_filetime(void) {
    FILETIME f;
    GetSystemTimeAsFileTime(&f);
    return (ULONGLONG)f.dwHighDateTime << 32 | f.dwLowDateTime;
}

static BOOL set_timer(HANDLE t, LONGLONG due, LONG period, PTIMERAPCROUTINE routine, void *arg) {
    LARGE_INTEGER d;
    d.QuadPart = due;
    return SetWaitableTimer(t, &d, period, routine, arg, FALSE);
}

/* The workers: each sets `started`, then waits as its kind says; its answer and, for the third, a drain after. */
static HANDLE started, worker_event;
static DWORD worker_ret, worker_drain, worker_ran_after;
static DWORD WINAPI sleeper(LPVOID param) {
    (void)param;
    SetEvent(started);
    worker_ret = SleepEx(INFINITE, TRUE);
    return 0;
}
static DWORD WINAPI alert_waiter(LPVOID param) {
    (void)param;
    SetEvent(started);
    worker_ret = WaitForSingleObjectEx(worker_event, INFINITE, TRUE);
    return 0;
}
static DWORD WINAPI plain_waiter(LPVOID param) {
    (void)param;
    SetEvent(started);
    worker_ret = WaitForSingleObject(worker_event, INFINITE);
    worker_drain = SleepEx(0, TRUE);
    worker_ran_after = (DWORD)calls;
    return 0;
}
static DWORD WINAPI nothing(LPVOID param) {
    (void)param;
    return 0;
}

static HANDLE start_worker(LPTHREAD_START_ROUTINE fn, DWORD *tid) {
    ResetEvent(started);
    HANDLE w = CreateThread(NULL, 0, fn, NULL, 0, tid);
    WaitForSingleObject(started, 5000);
    return w;
}

static void reset_counts(void) {
    calls = 0;
    order = 0;
    ran_on = 0;
    data_seen = 0;
}

void start(void) {
    HANDLE self = GetCurrentThread();
    started = CreateEventW(NULL, TRUE, FALSE, NULL);
    worker_event = CreateEventW(NULL, TRUE, FALSE, NULL);
    HANDLE unset = CreateEventW(NULL, TRUE, FALSE, NULL);
    HANDLE unset2 = CreateEventW(NULL, TRUE, FALSE, NULL);
    HANDLE set = CreateEventW(NULL, TRUE, TRUE, NULL);

    /* apc.self */
    reset_counts();
    queue_ok = 1;
    queue_ok &= QueueUserAPC(note_and_queue, self, 1) != 0;
    queue_ok &= QueueUserAPC(note, self, 2) != 0;
    queue_ok &= QueueUserAPC(note, self, 3) != 0;
    DWORD self_sleep = SleepEx(0, TRUE);
    ULONGLONG self_order = order;
    DWORD self_again = SleepEx(0, TRUE);

    /* apc.unalertable */
    reset_counts();
    int un_queued = QueueUserAPC(note, self, 5) != 0;
    DWORD un_sleep = SleepEx(20, FALSE);
    LONG un_ran_sleep = calls;
    DWORD un_wait = WaitForSingleObjectEx(unset, 20, FALSE);
    LONG un_ran_wait = calls;
    DWORD un_zero = WaitForSingleObjectEx(unset, 0, TRUE);
    LONG un_ran_zero = calls;
    DWORD un_drain = SleepEx(0, TRUE);
    LONG un_ran_drain = calls;

    /* apc.signaled */
    reset_counts();
    int sg_queued = QueueUserAPC(note, self, 6) != 0;
    DWORD sg_wait = WaitForSingleObjectEx(set, 0, TRUE);
    LONG sg_ran = calls;
    DWORD sg_drain = SleepEx(0, TRUE);
    LONG sg_ran_drain = calls;

    /* apc.multiple */
    reset_counts();
    int mu_queued = QueueUserAPC(note, self, 7) != 0;
    HANDLE two[2] = {unset, unset2};
    DWORD mu_wait = WaitForMultipleObjectsEx(2, two, FALSE, 5000, TRUE);
    LONG mu_ran = calls;

    /* apc.signal_and_wait */
    reset_counts();
    HANDLE a = CreateEventW(NULL, FALSE, FALSE, NULL);
    int sw_queued = QueueUserAPC(note, self, 8) != 0;
    DWORD sw_wait = SignalObjectAndWait(a, unset, 5000, TRUE);
    LONG sw_ran = calls;
    DWORD sw_signaled = WaitForSingleObject(a, 0);
    sw_queued &= QueueUserAPC(note, self, 9) != 0;
    DWORD sw_unalertable = SignalObjectAndWait(a, unset, 0, FALSE);
    LONG sw_ran_unalertable = calls;
    DWORD sw_drain = SleepEx(0, TRUE);
    LONG sw_ran_drain = calls;
    WaitForSingleObject(a, 0);                         /* `a` left unset */

    /* apc.other */
    reset_counts();
    DWORD tid;
    HANDLE w = start_worker(sleeper, &tid);
    int ot_queued = QueueUserAPC(note, w, 11) != 0;
    DWORD ot_ended = WaitForSingleObject(w, 5000);
    DWORD ot_wait = worker_ret, ot_data = data_seen;
    int ot_on_target = ran_on == tid;
    CloseHandle(w);

    /* apc.other_waiting */
    reset_counts();
    w = start_worker(alert_waiter, &tid);
    int ow_queued = QueueUserAPC(note, w, 12) != 0;
    DWORD ow_ended = WaitForSingleObject(w, 5000);
    DWORD ow_wait = worker_ret, ow_data = data_seen;
    int ow_on_target = ran_on == tid;
    CloseHandle(w);

    /* apc.other_unalertable */
    reset_counts();
    w = start_worker(plain_waiter, &tid);
    int ou_queued = QueueUserAPC(note, w, 13) != 0;
    Sleep(100);
    LONG ou_ran_before = calls;
    SetEvent(worker_event);
    DWORD ou_ended = WaitForSingleObject(w, 5000);
    DWORD ou_wait = worker_ret, ou_drain = worker_drain, ou_ran_after = worker_ran_after;
    CloseHandle(w);
    ResetEvent(worker_event);

    /* apc.invalid */
    HANDLE gone = CreateEventW(NULL, FALSE, FALSE, NULL);
    CloseHandle(gone);
    SetLastError(0);
    BOOL iv_closed = QueueUserAPC(note, gone, 1);
    DWORD iv_closed_error = GetLastError();
    w = CreateThread(NULL, 0, nothing, NULL, 0, NULL);
    WaitForSingleObject(w, 5000);
    SetLastError(0);
    BOOL iv_ended = QueueUserAPC(note, w, 1);
    DWORD iv_ended_error = GetLastError();
    CloseHandle(w);

    /* timer.manual */
    SetLastError(0);
    HANDLE tm = CreateWaitableTimerA(NULL, TRUE, NULL);
    DWORD tm_create_error = GetLastError();
    DWORD tm_inactive = WaitForSingleObject(tm, 0);
    SetLastError(0);
    BOOL tm_set = set_timer(tm, MS(100), 0, NULL, NULL);
    DWORD tm_set_error = GetLastError();
    DWORD tm_before = WaitForSingleObject(tm, 0);
    DWORD tm_wait = WaitForSingleObject(tm, 5000);
    DWORD tm_still = WaitForSingleObject(tm, 0);
    BOOL tm_reset = set_timer(tm, MS(100000), 0, NULL, NULL);
    DWORD tm_after_reset = WaitForSingleObject(tm, 0);
    CloseHandle(tm);

    /* timer.sync */
    HANDLE ts = CreateWaitableTimerA(NULL, FALSE, NULL);
    set_timer(ts, MS(50), 0, NULL, NULL);
    DWORD ts_wait = WaitForSingleObject(ts, 5000);
    DWORD ts_again = WaitForSingleObject(ts, 0);
    CloseHandle(ts);

    /* timer.periodic */
    HANDLE tp = CreateWaitableTimerA(NULL, FALSE, NULL);
    set_timer(tp, MS(20), 20, NULL, NULL);
    DWORD tp_first = WaitForSingleObject(tp, 5000);
    DWORD tp_second = WaitForSingleObject(tp, 5000);
    DWORD tp_third = WaitForSingleObject(tp, 5000);
    BOOL tp_cancel = set_timer(tp, MS(100000), 0, NULL, NULL);
    DWORD tp_after = WaitForSingleObject(tp, 0);
    CloseHandle(tp);

    /* timer.absolute */
    HANDLE ta = CreateWaitableTimerA(NULL, TRUE, NULL);
    set_timer(ta, (LONGLONG)(now_filetime() + 500000), 0, NULL, NULL);
    DWORD ta_wait = WaitForSingleObject(ta, 5000);
    HANDLE tq = CreateWaitableTimerA(NULL, FALSE, NULL);
    set_timer(tq, (LONGLONG)(now_filetime() - 100000000), 0, NULL, NULL);
    DWORD ta_past = WaitForSingleObject(tq, 0);
    CloseHandle(ta);
    CloseHandle(tq);

    /* timer.multiple */
    HANDLE tw = CreateWaitableTimerA(NULL, FALSE, NULL);
    set_timer(tw, MS(30), 0, NULL, NULL);
    HANDLE pair[2] = {unset, tw};
    DWORD tw_wait = WaitForMultipleObjects(2, pair, FALSE, 5000);
    CloseHandle(tw);

    /* timer.apc */
    static int marker;
    timer_calls = 0;
    HANDLE tc = CreateWaitableTimerA(NULL, FALSE, NULL);
    ULONGLONG tc_before = now_filetime();
    BOOL tc_set = set_timer(tc, MS(20), 0, on_timer, &marker);
    DWORD tc_sleep = SleepEx(5000, TRUE);
    LONG tc_calls = timer_calls;
    int tc_arg_ok = timer_arg == &marker;
    ULONGLONG due = tc_before + 200000;
    int tc_time_ok = timer_time + 10000000 >= due && timer_time <= due + 10000000;
    DWORD tc_signaled = WaitForSingleObject(tc, 0);
    DWORD tc_again = WaitForSingleObject(tc, 0);

    /* timer.apc_unalertable */
    timer_calls = 0;
    set_timer(tc, MS(20), 0, on_timer, &marker);
    Sleep(100);
    LONG tu_calls_before = timer_calls;
    DWORD tu_sleep = SleepEx(0, TRUE);
    LONG tu_calls = timer_calls;

    /* timer.apc_periodic */
    timer_calls = 0;
    set_timer(tc, MS(20), 50, on_timer, &marker);
    Sleep(300);
    DWORD tr_sleep = SleepEx(0, TRUE);
    LONG tr_calls = timer_calls;
    BOOL tr_cancel = set_timer(tc, MS(100000), 0, NULL, NULL);
    DWORD tr_after = SleepEx(100, TRUE);
    LONG tr_calls_after = timer_calls;

    /* timer.apc_cancel */
    timer_calls = 0;
    set_timer(tc, MS(20), 0, on_timer, &marker);
    set_timer(tc, MS(50), 0, NULL, NULL);
    DWORD tx_sleep = SleepEx(200, TRUE);
    LONG tx_calls = timer_calls;
    DWORD tx_wait = WaitForSingleObject(tc, 5000);
    CloseHandle(tc);

    /* timer.invalid */
    HANDLE ti = CreateWaitableTimerA(NULL, FALSE, NULL);
    SetLastError(0);
    BOOL ti_event = set_timer(unset, MS(20), 0, NULL, NULL);
    DWORD ti_event_error = GetLastError();
    SetLastError(0);
    BOOL ti_period = set_timer(ti, MS(20), -1, NULL, NULL);
    DWORD ti_period_error = GetLastError();
    CloseHandle(ti);

    /* timer.named */
    HANDLE nt[2];
    SetLastError(0);
    nt[0] = CreateWaitableTimerA(NULL, TRUE, N_TIMER);
    DWORD nt_create_error = GetLastError();
    SetLastError(0);
    nt[1] = CreateWaitableTimerA(NULL, FALSE, N_TIMER);
    DWORD nt_again_error = GetLastError();
    int nt_distinct = nt[0] && nt[1] && nt[0] != nt[1];
    set_timer(nt[0], MS(20), 0, NULL, NULL);
    DWORD nt_wait_second = WaitForSingleObject(nt[1], 5000);
    DWORD nt_wait_first = WaitForSingleObject(nt[0], 0);
    SetLastError(0);
    HANDLE nt_event = CreateEventW(NULL, FALSE, FALSE, N_TIMER_W);
    DWORD nt_event_error = GetLastError();
    SetLastError(0);
    HANDLE nt_mutex = CreateMutexW(NULL, FALSE, N_TIMER_W);
    DWORD nt_mutex_error = GetLastError();
    HANDLE ev = CreateEventW(NULL, FALSE, FALSE, N_EVENT_W);
    SetLastError(0);
    HANDLE nt_on_event = CreateWaitableTimerA(NULL, FALSE, N_EVENT);
    DWORD nt_on_event_error = GetLastError();
    CloseHandle(ev);
    CloseHandle(nt[0]);
    CloseHandle(nt[1]);
    SetLastError(0);
    HANDLE nt_made_again = CreateWaitableTimerA(NULL, TRUE, N_TIMER);
    DWORD nt_made_again_error = GetLastError();
    CloseHandle(nt_made_again);

    row("apc.self");
    field("queued", queue_ok);
    field("sleep", self_sleep);
    field("order", self_order);
    field("again", self_again);
    end();
    row("apc.unalertable");
    field("queued", un_queued);
    field("sleep", un_sleep);
    field("ran_sleep", (DWORD)un_ran_sleep);
    field("wait", un_wait);
    field("ran_wait", (DWORD)un_ran_wait);
    field("zero", un_zero);
    field("ran_zero", (DWORD)un_ran_zero);
    field("drain", un_drain);
    field("ran_drain", (DWORD)un_ran_drain);
    end();
    row("apc.signaled");
    field("queued", sg_queued);
    field("wait", sg_wait);
    field("ran", (DWORD)sg_ran);
    field("drain", sg_drain);
    field("ran_drain", (DWORD)sg_ran_drain);
    end();
    row("apc.multiple");
    field("queued", mu_queued);
    field("wait", mu_wait);
    field("ran", (DWORD)mu_ran);
    end();
    row("apc.signal_and_wait");
    field("queued", sw_queued);
    field("wait", sw_wait);
    field("ran", (DWORD)sw_ran);
    field("signaled", sw_signaled);
    field("unalertable", sw_unalertable);
    field("ran_unalertable", (DWORD)sw_ran_unalertable);
    field("drain", sw_drain);
    field("ran_drain", (DWORD)sw_ran_drain);
    end();
    row("apc.other");
    field("queued", ot_queued);
    field("wait", ot_wait);
    field("data", ot_data);
    field("on_target", ot_on_target);
    field("ended", ot_ended);
    end();
    row("apc.other_waiting");
    field("queued", ow_queued);
    field("wait", ow_wait);
    field("data", ow_data);
    field("on_target", ow_on_target);
    field("ended", ow_ended);
    end();
    row("apc.other_unalertable");
    field("queued", ou_queued);
    field("ran_before_set", (DWORD)ou_ran_before);
    field("wait", ou_wait);
    field("drain", ou_drain);
    field("ran_after", ou_ran_after);
    field("ended", ou_ended);
    end();
    row("apc.invalid");
    field("closed", iv_closed != 0);
    field("closed_error", iv_closed_error);
    field("ended", iv_ended != 0);
    field("ended_error", iv_ended_error);
    end();
    row("timer.manual");
    field("created", tm != NULL);
    field("create_error", tm_create_error);
    field("inactive", tm_inactive);
    field("set", tm_set != 0);
    field("set_error", tm_set_error);
    field("before", tm_before);
    field("wait", tm_wait);
    field("still", tm_still);
    field("reset", tm_reset != 0);
    field("after_reset", tm_after_reset);
    end();
    row("timer.sync");
    field("wait", ts_wait);
    field("again", ts_again);
    end();
    row("timer.periodic");
    field("first", tp_first);
    field("second", tp_second);
    field("third", tp_third);
    field("cancel", tp_cancel != 0);
    field("after", tp_after);
    end();
    row("timer.absolute");
    field("wait", ta_wait);
    field("past", ta_past);
    end();
    row("timer.multiple");
    field("wait", tw_wait);
    end();
    row("timer.apc");
    field("set", tc_set != 0);
    field("sleep", tc_sleep);
    field("calls", (DWORD)tc_calls);
    field("arg_ok", tc_arg_ok);
    field("time_ok", tc_time_ok);
    field("signaled", tc_signaled);
    field("again", tc_again);
    end();
    row("timer.apc_unalertable");
    field("calls_before", (DWORD)tu_calls_before);
    field("sleep", tu_sleep);
    field("calls", (DWORD)tu_calls);
    end();
    row("timer.apc_periodic");
    field("sleep", tr_sleep);
    field("calls", (DWORD)tr_calls);
    field("cancel", tr_cancel != 0);
    field("after", tr_after);
    field("calls_after", (DWORD)tr_calls_after);
    end();
    row("timer.apc_cancel");
    field("sleep", tx_sleep);
    field("calls", (DWORD)tx_calls);
    field("wait", tx_wait);
    end();
    row("timer.invalid");
    field("event", ti_event != 0);
    field("event_error", ti_event_error);
    field("period", ti_period != 0);
    field("period_error", ti_period_error);
    end();
    row("timer.named");
    field("created", nt[0] != NULL);
    field("create_error", nt_create_error);
    field("again", nt[1] != NULL);
    field("again_error", nt_again_error);
    field("distinct", nt_distinct);
    field("wait_second", nt_wait_second);
    field("wait_first", nt_wait_first);
    field("event_on_timer", nt_event != NULL);
    field("event_on_timer_error", nt_event_error);
    field("mutex_on_timer", nt_mutex != NULL);
    field("mutex_on_timer_error", nt_mutex_error);
    field("timer_on_event", nt_on_event != NULL);
    field("timer_on_event_error", nt_on_event_error);
    field("made_again", nt_made_again != NULL);
    field("made_again_error", nt_made_again_error);
    end();
    ExitProcess(0);
}
