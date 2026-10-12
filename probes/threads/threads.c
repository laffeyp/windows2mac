// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* threads.c — threads in the game's forms (Sprint 051, family "threads"; specs/work_list.json).
 *
 * No C runtime. The capture (store/game-runs/38113342549, 38113343972) holds the forms: CreateThread(NULL, 0, start,
 * param, 0, &tid) (28 calls, mono and UnityPlayer); SetThreadPriority 0 and -2; SetThreadDescription; mono's collector
 * calling SuspendThread, GetThreadContext and ResumeThread (111 each) and GetExitCodeThread; SleepEx(n, TRUE);
 * SwitchToThread; AppPolicyGetThreadInitializationType through GetProcAddress. Threads are sequenced by events, so
 * every row is the same on every run; thread ids and handles print as relations, never as values. threads_dll.dll
 * logs each TLS callback and DllMain call with its reason and thread; the rows name the threads main, worker and
 * so on. Rows: each call's return and last error (after SetLastError(0xDEAD)).
 */
#include <windows.h>

#define SENTINEL 0xDEADu

__declspec(dllimport) LONG threads_dll_count(void);
__declspec(dllimport) int threads_dll_event(LONG i, DWORD *reason, DWORD *tid);
__declspec(dllimport) int *threads_dll_counter(void);
__declspec(dllimport) void threads_dll_watch_fls(DWORD index);
__declspec(dllimport) void threads_dll_fls_seen(ULONG_PTR *value, DWORD *err);

static char out[16384];
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
static void sfield(const char *k, long long v) {
    put(",\""); put(k); put("\":");
    if (v < 0) { put("-"); v = -v; }
    putu((unsigned long long)v);
}
/* Each row is written when it ends, so a fault leaves the rows before it. */
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}

static ULONG_PTR teb(void) { ULONG_PTR t; __asm__ volatile("movq %%gs:0x30, %0" : "=r"(t)); return t; }

/* Thread names for the event log. */
static DWORD tids[8];
static const char *names[8] = {"main", "worker", "spinner", "suspended", "exiter", "other", "other", "other"};
static const char *name_of(DWORD tid) {
    for (int i = 0; i < 8; i++) if (tids[i] == tid) return names[i];
    return "unknown";
}
/* The DLL's log from event `from` on: "<kind>:<reason>:<thread>". */
static LONG events(const char *name, LONG from) {
    LONG count = threads_dll_count();
    row(name); put(",\"events\":[");
    for (LONG i = from; i < count; i++) {
        DWORD reason, tid;
        int kind = threads_dll_event(i, &reason, &tid);
        if (i > from) put(",");
        put("\""); put(kind == 1 ? "tls" : "dllmain"); put(":"); putu(reason); put(":"); put(name_of(tid)); put("\"");
    }
    put("]"); end();
    return count;
}

static HANDLE event_started, event_go;
static DWORD tls_slot, fls_slot;

/* worker: what a new thread starts with, then a wait, then a return value */
static volatile DWORD w_tid, w_last_error, w_counter;
static volatile ULONG_PTR w_tls, w_fls;
static volatile DWORD w_fls_err;
static DWORD WINAPI worker(LPVOID param) {
    w_tid = GetCurrentThreadId();
    w_last_error = GetLastError();
    w_tls = (ULONG_PTR)TlsGetValue(tls_slot);
    /* FLS is the thread's own (Sprint 061): the main thread's value is not this thread's, and FlsGetValue on a
     * thread that has set none answers NULL with ERROR_INVALID_PARAMETER, as the game's new threads saw */
    SetLastError(SENTINEL);
    w_fls = (ULONG_PTR)FlsGetValue(fls_slot);
    w_fls_err = GetLastError();
    FlsSetValue(fls_slot, (LPVOID)0x88);
    w_counter = (DWORD)*threads_dll_counter();
    *threads_dll_counter() = 7;
    SetLastError(0x77);
    TlsSetValue(tls_slot, (LPVOID)0x99);
    SetEvent(event_started);
    WaitForSingleObject(event_go, INFINITE);
    return (DWORD)(ULONG_PTR)param;
}

/* spinner: counts until told to stop; its stack bounds from its TEB */
static volatile LONG spin_stop, spin_count;
static volatile ULONG_PTR spin_stack_base, spin_stack_limit;
static DWORD WINAPI spinner(LPVOID param) {
    (void)param;
    spin_stack_base = *(ULONG_PTR *)(teb() + 0x08);
    spin_stack_limit = *(ULONG_PTR *)(teb() + 0x10);
    while (!spin_stop) spin_count++;
    return 3;
}

static volatile LONG suspended_ran;
static DWORD WINAPI suspended(LPVOID param) { (void)param; suspended_ran = 1; return 4; }

static DWORD WINAPI exiter(LPVOID param) { (void)param; ExitThread(77); return 5; }

void start(void) {
    tids[0] = GetCurrentThreadId();
    events("events.start", 0);
    LONG seen = threads_dll_count();
    event_started = CreateEventW(NULL, FALSE, FALSE, NULL);
    event_go = CreateEventW(NULL, TRUE, FALSE, NULL);
    tls_slot = TlsAlloc();
    TlsSetValue(tls_slot, (LPVOID)0x55);
    fls_slot = FlsAlloc(NULL);
    FlsSetValue(fls_slot, (LPVOID)0x66);
    threads_dll_watch_fls(fls_slot);
    *threads_dll_counter() = 1000;

    /* create, run to a wait, exit code while running, then its return */
    DWORD tid = 0;
    SetLastError(SENTINEL);
    HANDLE h = CreateThread(NULL, 0, worker, (LPVOID)0x5A5A, 0, &tid);
    DWORD err = GetLastError();
    tids[1] = tid;
    row("create"); field("handle_nonzero", h != NULL); field("handle_mod_4", (ULONG_PTR)h % 4); field("err", err);
    field("tid_nonzero", tid != 0); field("tid_not_main", tid != tids[0]); end();
    WaitForSingleObject(event_started, INFINITE);
    row("worker.start"); field("tid_matches", w_tid == tid); field("last_error", w_last_error); field("tls", w_tls);
    field("static_tls", w_counter); field("fls", w_fls); field("fls_err", w_fls_err); end();
    ULONG_PTR attach_fls;
    DWORD attach_err;
    threads_dll_fls_seen(&attach_fls, &attach_err);
    row("worker.attach_fls"); field("fls", attach_fls); field("err", attach_err); end();
    DWORD code = 0xA5A5;
    SetLastError(SENTINEL);
    BOOL r = GetExitCodeThread(h, &code);
    err = GetLastError();
    row("exit_code.running"); field("ret", r != 0); field("code", code); field("err", err); end();
    row("main.after_worker"); field("last_error", GetLastError()); field("tls", (ULONG_PTR)TlsGetValue(tls_slot));
    field("fls", (ULONG_PTR)FlsGetValue(fls_slot));
    field("static_tls", (DWORD)*threads_dll_counter()); end();
    seen = events("events.worker_started", seen);
    SetEvent(event_go);
    SetLastError(SENTINEL);
    DWORD w = WaitForSingleObject(h, INFINITE);
    err = GetLastError();
    row("wait.worker"); field("ret", w); field("err", err); end();
    SetLastError(SENTINEL);
    r = GetExitCodeThread(h, &code);
    err = GetLastError();
    row("exit_code.returned"); field("ret", r != 0); field("code", code); field("err", err); end();
    seen = events("events.worker_ended", seen);
    row("wait.again"); field("ret", WaitForSingleObject(h, 0)); end();
    CloseHandle(h);

    /* suspend a spinning thread, read its context, resume it */
    h = CreateThread(NULL, 0, spinner, NULL, 0, &tid);
    tids[2] = tid;
    while (!spin_count) SwitchToThread();
    SetLastError(SENTINEL);
    DWORD s1 = SuspendThread(h);
    DWORD s2 = SuspendThread(h);
    err = GetLastError();
    row("suspend"); field("first", s1); field("second", s2); field("err", err); end();
    LONG frozen = spin_count;
    Sleep(20);
    row("suspend.frozen"); field("count_unchanged", spin_count == frozen); end();
    static CONTEXT ctx;
    ctx.ContextFlags = CONTEXT_CONTROL | CONTEXT_INTEGER;
    SetLastError(SENTINEL);
    r = GetThreadContext(h, &ctx);
    err = GetLastError();
    ULONG_PTR image = (ULONG_PTR)GetModuleHandleW(NULL);
    ULONG_PTR image_end = image + ((IMAGE_NT_HEADERS64 *)(image + ((IMAGE_DOS_HEADER *)image)->e_lfanew))->OptionalHeader.SizeOfImage;
    row("context"); field("ret", r != 0); field("err", err); field("flags", ctx.ContextFlags);
    field("rip_in_image", ctx.Rip >= image && ctx.Rip < image_end);
    field("rsp_in_stack", ctx.Rsp >= spin_stack_limit && ctx.Rsp < spin_stack_base); field("seg_cs", ctx.SegCs);
    field("seg_ss", ctx.SegSs); end();
    SetLastError(SENTINEL);
    DWORD r1 = ResumeThread(h), r2 = ResumeThread(h), r3 = ResumeThread(h);
    err = GetLastError();
    row("resume"); field("first", r1); field("second", r2); field("third", r3); field("err", err); end();
    spin_stop = 1;
    WaitForSingleObject(h, INFINITE);
    GetExitCodeThread(h, &code);
    SetLastError(SENTINEL);
    DWORD s3 = SuspendThread(h);
    err = GetLastError();
    row("suspend.exited"); field("ret", s3); field("err", err); field("code", code); end();
    SetLastError(SENTINEL);
    DWORD r4 = ResumeThread(h);
    err = GetLastError();
    row("resume.exited"); field("ret", r4); field("err", err); end();
    CloseHandle(h);
    seen = events("events.spinner", seen);

    /* created suspended: runs only after ResumeThread */
    h = CreateThread(NULL, 0, suspended, NULL, CREATE_SUSPENDED, &tid);
    tids[3] = tid;
    Sleep(20);
    row("create_suspended"); field("ran_before_resume", suspended_ran); end();
    seen = events("events.created_suspended", seen);

    /* priorities and the description, on the suspended thread */
    SetLastError(SENTINEL);
    BOOL p1 = SetThreadPriority(h, THREAD_PRIORITY_LOWEST);
    int g1 = GetThreadPriority(h);
    BOOL p2 = SetThreadPriority(h, THREAD_PRIORITY_NORMAL);
    int g2 = GetThreadPriority(h);
    err = GetLastError();
    row("priority"); field("set_lowest", p1 != 0); sfield("get_lowest", g1); field("set_normal", p2 != 0); sfield("get_normal", g2);
    field("err", err); end();
    SetLastError(SENTINEL);
    BOOL p3 = SetThreadPriority(h, 100);
    err = GetLastError();
    row("priority.invalid"); field("ret", p3 != 0); field("err", err); end();
    SetLastError(SENTINEL);
    HRESULT hr = SetThreadDescription(h, L"w2m worker");
    err = GetLastError();
    row("description.set"); field("hr", (DWORD)hr); field("err", err); end();
    PWSTR desc = NULL;
    hr = GetThreadDescription(h, &desc);
    row("description.get"); field("hr", (DWORD)hr); field("matches", desc && desc[0] == 'w' && desc[1] == '2' && desc[9] == 'r' && desc[10] == 0);
    end();
    if (desc) LocalFree(desc);
    SetLastError(SENTINEL);
    DWORD r5 = ResumeThread(h);
    err = GetLastError();
    WaitForSingleObject(h, INFINITE);
    GetExitCodeThread(h, &code);
    row("resume.created_suspended"); field("ret", r5); field("err", err); field("ran", suspended_ran); field("code", code); end();
    CloseHandle(h);
    seen = events("events.suspended_ran", seen);

    /* ExitThread from inside the start routine */
    h = CreateThread(NULL, 0, exiter, NULL, 0, &tid);
    tids[4] = tid;
    WaitForSingleObject(h, INFINITE);
    GetExitCodeThread(h, &code);
    row("exit_thread"); field("code", code); end();
    CloseHandle(h);
    seen = events("events.exiter", seen);

    /* yields */
    SetLastError(SENTINEL);
    DWORD z0 = SleepEx(0, TRUE), z1 = SleepEx(1, TRUE);
    err = GetLastError();
    row("sleep_ex"); field("zero", z0); field("one", z1); field("err", err); end();
    SetLastError(SENTINEL);
    BOOL sw = SwitchToThread();
    err = GetLastError();
    row("switch_to_thread"); field("ret_is_bool", sw == 0 || sw == 1); field("err", err); end();

    /* AppPolicyGetThreadInitializationType(GetCurrentThreadEffectiveToken(), &policy) through GetProcAddress */
    typedef LONG (WINAPI *policy_fn)(HANDLE, int *);
    policy_fn get = (policy_fn)(void *)GetProcAddress(GetModuleHandleW(L"kernelbase.dll"), "AppPolicyGetThreadInitializationType");
    int policy = 0xA5;
    SetLastError(SENTINEL);
    LONG pr = get ? get((HANDLE)(LONG_PTR)-6, &policy) : -1;
    err = GetLastError();
    row("app_policy"); field("found", get != NULL); sfield("ret", pr); sfield("policy", policy); field("err", err); end();

    /* GetCurrentThreadId, GetThreadId on a pseudo handle */
    row("current"); field("pseudo_handle", (ULONG_PTR)GetCurrentThread() == (ULONG_PTR)-2);
    field("thread_id_of_pseudo_is_main", GetThreadId(GetCurrentThread()) == tids[0]); end();
    ExitProcess(0);
}
