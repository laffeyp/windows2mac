#include <windows.h>
/* Sprint 368: SleepConditionVariableCS, which the game's batch run on two vCPUs called and w2m lacked (the run
 * stopped "import KERNELBASE.dll!SleepConditionVariableCS"). Each check holds a sentence of Microsoft Learn's
 * SleepConditionVariableCS page, read 2026-11-27:
 *   https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-sleepconditionvariablecs
 * The process exits 0 when each holds, else the number of the first that does not:
 *   1  "If dwMilliseconds is zero, the function tests the states of the specified objects and returns immediately";
 *      "If the time-out interval elapses, the function re-acquires the critical section and returns zero": a 0 ms
 *      sleep that no thread wakes answers zero
 *   2  "Possible error codes include ERROR_TIMEOUT, which indicates that the time-out interval has elapsed before
 *      another thread has attempted to wake the sleeping thread": its last error is ERROR_TIMEOUT (1460)
 *   3  "the function re-acquires the critical section": after it the caller owns the section, entered once as it was
 *      ("This critical section must be entered exactly once by the caller"), read as
 *      contracts/windows.critical_section.single_thread records an owner's section: RecursionCount 1, OwningThread
 *      the caller's id
 *   4  "Sleeps on the specified condition variable and releases the specified critical section as an atomic
 *      operation": a worker sets `waiting` inside the section and then sleeps, so the main thread, once it enters the
 *      section and finds `waiting`, knows the worker is in the call, sets `ready` and wakes it ("A thread that is
 *      sleeping on a condition variable can be woken ... using the WakeConditionVariable or WakeAllConditionVariable
 *      function"; "After the thread is woken, it re-acquires the critical section it released"). Its sleeps have no
 *      time-out ("If dwMilliseconds is INFINITE, the function's time-out interval never elapses"), so none answers
 *      zero ("If the function fails or the time-out interval elapses, the return value is zero"); it rechecks
 *      `ready` in a loop ("Condition variables are subject to spurious wakeups ... you should recheck a predicate
 *      (typically in a while loop)") and ends, which the main thread's wait sees
 *   5  "it re-acquires the critical section it released", which the worker had entered once, so its one
 *      LeaveCriticalSection frees it: RecursionCount 0 and no owner at the end, the contract's left section */
static CRITICAL_SECTION cs;
static CONDITION_VARIABLE cv;
static volatile LONG ready, waiting, failed;
static DWORD WINAPI worker(LPVOID p) {
    (void)p;
    EnterCriticalSection(&cs);
    while (!ready) {
        waiting = 1;
        if (!SleepConditionVariableCS(&cv, &cs, INFINITE)) failed = 1;
    }
    LeaveCriticalSection(&cs);
    return 0;
}
void start(void) {
    InitializeCriticalSection(&cs);
    InitializeConditionVariable(&cv);
    EnterCriticalSection(&cs);
    BOOL timed = SleepConditionVariableCS(&cv, &cs, 0);
    DWORD error = GetLastError();
    BOOL owned = cs.RecursionCount == 1 && (DWORD)(ULONG_PTR)cs.OwningThread == GetCurrentThreadId();
    LeaveCriticalSection(&cs);
    if (timed) ExitProcess(1);
    if (error != ERROR_TIMEOUT) ExitProcess(2);
    if (!owned) ExitProcess(3);
    HANDLE t = CreateThread(NULL, 0, worker, NULL, 0, NULL);
    for (;;) {
        EnterCriticalSection(&cs);
        if (waiting) break;
        LeaveCriticalSection(&cs);
        Sleep(1);
    }
    ready = 1;
    WakeConditionVariable(&cv);
    LeaveCriticalSection(&cs);
    WaitForSingleObject(t, INFINITE);
    if (failed) ExitProcess(4);
    if (cs.RecursionCount != 0 || cs.OwningThread != NULL) ExitProcess(5);
    ExitProcess(0);
}
