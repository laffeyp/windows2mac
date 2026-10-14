// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* threads.c — guest threads (Sprint 051, family "threads"; specs/work_list.json).
 *
 * contracts/windows.thread.threads records Windows (Server 2025 and 2022, runs 38124576301, 38124578332; equal):
 *   - CreateThread gives a handle (a multiple of 4) and a new thread id, and leaves the last error; the new thread
 *     runs each module's TLS callback and DllMain(DLL_THREAD_ATTACH), then its start routine with the last error
 *     187 (ERROR_SEM_NOT_FOUND), empty TLS slots and fresh static TLS; at its end, DLL_THREAD_DETACH the same way;
 *     a thread created suspended runs none of it until resumed
 *   - GetExitCodeThread: STILL_ACTIVE (259) while it runs, then its return or ExitThread's code
 *   - SuspendThread and ResumeThread answer the previous count; on an ended thread SuspendThread fails with
 *     ERROR_ACCESS_DENIED and ResumeThread answers 0 and leaves the last error
 *   - GetThreadContext of a suspended thread: TRUE, its registers, ContextFlags as asked, CS 0x33 and SS 0x2B
 *   - SetThreadPriority takes -15, -2 to 2 and 15 (ERROR_INVALID_PARAMETER otherwise); GetThreadPriority reads
 *     it back; SetThreadDescription and GetThreadDescription answer 0x10000000
 *   - SleepEx and Sleep answer 0; AppPolicyGetThreadInitializationType gives policy 0 and answers 0
 * The host's scheduler (src/w2m/scheduler.py) runs the threads on one vCPU, switching at instruction boundaries:
 * a service that must wait sets state.schedule (runtime.h) and the bridge hands the vCPU over.
 */
#include "runtime.h"

#define STILL_ACTIVE 259u
#define CREATE_SUSPENDED 0x4u
#define STACK_SIZE_PARAM_IS_A_RESERVATION 0x10000u
#define ERROR_ACCESS_DENIED 5u
#define ERROR_INVALID_PARAMETER 87u
#define INFINITE 0xFFFFFFFFu
#define DESCRIPTION_HR 0x10000000u            /* what both description calls answer on the runners */
#define CURRENT_THREAD (~0ull - 1)

int w2m_object_thread_of(u64 h, u32 *tid);   /* sync_objects.c: the thread a handle names */
u64 w2m_object_thread(u64 api, u32 tid);      /* sync_objects.c: a new handle on thread tid */

/* The host keeps the number of blocked threads here (scheduler.py), so a signal with no waiter costs no exit; and the
 * keys they wait on, so a signal of a key no thread waits on costs none either: w2m_blocked_any counts the blocked
 * threads whose wait takes any signal (key 0, or more keys than the table holds), w2m_blocked_keys the others' keys. */
volatile u64 w2m_threads_blocked;
#define W2M_BLOCKED_KEYS 32
volatile u64 w2m_blocked_any;
volatile u64 w2m_blocked_keys[W2M_BLOCKED_KEYS];

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

static u64 host(u64 sub, u64 a, u64 b, u64 c) { return w2m_host(W2M_HOST_THREAD, sub, a, b, c, 0); }

/* Recentering step 5: a blocked thread names what it waits on, and a signal names what it signals, so the host wakes
 * only the threads that wait on it (before, every blocked thread woke on any signal, ran to find its wait unmet and
 * blocked again). Key 0 on either side wakes every blocked thread: a wait on several objects, a pipe. */
void w2m_block(u64 deadline, u64 key) {
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    if (!sc->wait_deadline) sc->wait_deadline = deadline;
    sc->wait_key = key;
    w2m_state()->schedule |= W2M_SCHEDULE_RETRY;
}

void w2m_signal(u64 key) {
    if (!w2m_threads_blocked) return;
    if (key && !w2m_blocked_any) {
        int i = 0;
        while (i < W2M_BLOCKED_KEYS && w2m_blocked_keys[i] != key) i++;
        if (i == W2M_BLOCKED_KEYS) return;                   /* no thread waits on it */
    }
    host(W2M_THREAD_SIGNAL, key, 0, 0);
}

static u32 thread_of(u64 api, u64 h) {
    u32 tid = 0;
    if (h == CURRENT_THREAD) return (u32)w2m_teb()->client_thread;
    if (!w2m_object_thread_of(h, &tid)) w2m_unimplemented(api, h);      /* not a thread handle: not in the rows */
    return tid;
}

/* CreateThread(lpThreadAttributes, dwStackSize, lpStartAddress, lpParameter, dwCreationFlags, lpThreadId) */
u64 k32_CreateThread(u64 attributes, u64 stack_size, u64 start, u64 parameter, u64 rsp) {
    u32 flags = *(u32 *)(rsp + 0x28);
    u64 tid_out = *(u64 *)(rsp + 0x30);
    if (attributes || (flags & ~(CREATE_SUSPENDED | STACK_SIZE_PARAM_IS_A_RESERVATION)))
        w2m_unimplemented(W2M_API_CreateThread, attributes ? attributes : flags);       /* not in the rows */
    u32 tid = (u32)w2m_host(W2M_HOST_THREAD, W2M_THREAD_CREATE, start, parameter,
                            (u64)flags | stack_size << 32, 0);
    w2m_tlb_flush();                              /* the new stack's guard and reserved pages are unmapped */
    u64 h = w2m_object_thread(W2M_API_CreateThread, tid);
    if (tid_out) *(u32 *)tid_out = tid;
    return w2m_logged(W2M_API_CreateThread, start, h);
}

/* ExitThread(dwExitCode): the thread runs its modules' DLL_THREAD_DETACH (a stub the host writes) and then ends in
 * W2mThreadEnd; the start routine's return comes here too (the attach stub calls ExitThread with it). */
W2M_VOID u64 k32_ExitThread(u64 code) {
    w2m_thread_scratch()->exit_code = (u32)code;
    w2m_state()->redirect_rip = host(W2M_THREAD_EXIT_STUB, (u32)code, 0, 0);
    return w2m_logged(W2M_API_ExitThread, (u32)code, 0);
}

/* The last call of every thread but the first: the exit code is final, waiters on its handle wake. */
W2M_VOID u64 k32_W2mThreadEnd(void) {
    host(W2M_THREAD_END, w2m_thread_scratch()->exit_code, 0, 0);
    w2m_state()->schedule |= W2M_SCHEDULE_END;
    return w2m_logged(W2M_API_W2mThreadEnd, 0, 0);
}

u64 k32_GetExitCodeThread(u64 h, u64 code) {
    u32 tid = thread_of(W2M_API_GetExitCodeThread, h);
    *(u32 *)code = (u32)host(W2M_THREAD_EXIT_CODE, tid, 0, 0);
    return w2m_logged(W2M_API_GetExitCodeThread, h, 1);
}

/* For sync_objects.c: a thread's handle signals once the thread has ended. */
int w2m_thread_ended(u32 tid) { return host(W2M_THREAD_EXIT_CODE, tid, 0, 0) != STILL_ACTIVE; }

u64 k32_SuspendThread(u64 h) {
    u32 tid = thread_of(W2M_API_SuspendThread, h);
    if (tid == (u32)w2m_teb()->client_thread) w2m_unimplemented(W2M_API_SuspendThread, h);   /* itself: not in the rows */
    u64 previous = host(W2M_THREAD_SUSPEND, tid, 0, 0);
    if (previous == ~0ull) {                       /* ended */
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_SuspendThread, h, 0xFFFFFFFFu);
    }
    return w2m_logged(W2M_API_SuspendThread, h, previous);
}

u64 k32_ResumeThread(u64 h) {
    u32 tid = thread_of(W2M_API_ResumeThread, h);
    return w2m_logged(W2M_API_ResumeThread, h, host(W2M_THREAD_RESUME, tid, 0, 0));
}

/* GetThreadContext(hThread, lpContext): the host fills the CONTEXT from the thread's saved state, as its
 * ContextFlags ask; the thread must be another. On one vCPU a thread suspended while it runs was stopped by its
 * time slice at an x86 instruction boundary, so the saved state is its registers there (rows threads.context:
 * a spinner's RIP and RSP; Sprint 066 probes/running_thread_context awaits Windows' rows for every register). */
u64 k32_GetThreadContext(u64 h, u64 context) {
    u32 tid = thread_of(W2M_API_GetThreadContext, h);
    if (tid == (u32)w2m_teb()->client_thread) w2m_unimplemented(W2M_API_GetThreadContext, h);   /* itself */
    host(W2M_THREAD_CONTEXT, tid, context, 0);
    return w2m_logged(W2M_API_GetThreadContext, h, 1);
}

/* Priorities and descriptions by thread id. */
#define THREADS 256
static struct { u32 tid; s32 priority; u64 description; } info[THREADS];

static int info_of(u32 tid) {
    for (int i = 0; i < THREADS; i++) if (info[i].tid == tid) return i;
    for (int i = 0; i < THREADS; i++) if (!info[i].tid) { info[i].tid = tid; return i; }
    w2m_unimplemented(W2M_API_SetThreadPriority, THREADS);
    return 0;
}

u64 k32_SetThreadPriority(u64 h, u64 priority) {
    u32 tid = thread_of(W2M_API_SetThreadPriority, h);
    s32 p = (s32)priority;
    if (!((p >= -2 && p <= 2) || p == -15 || p == 15)) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_SetThreadPriority, h, 0);
    }
    info[info_of(tid)].priority = p;
    return w2m_logged(W2M_API_SetThreadPriority, h, 1);
}

/* For thread_queries.c's GetThreadPriority. */
s32 w2m_thread_priority(u32 tid) {
    for (int i = 0; i < THREADS; i++) if (info[i].tid == tid) return info[i].priority;
    return 0;
}

static u64 copy_string(const unsigned short *s) {
    u64 n = 0;
    while (s && s[n]) n++;
    unsigned short *d = (unsigned short *)w2m_heap_take(2 * (n + 1));
    if (!d) w2m_unimplemented(W2M_API_SetThreadDescription, n);
    for (u64 i = 0; i < n; i++) d[i] = s[i];
    d[n] = 0;
    return (u64)d;
}

u64 k32_SetThreadDescription(u64 h, u64 description) {
    u32 tid = thread_of(W2M_API_SetThreadDescription, h);
    int i = info_of(tid);
    if (info[i].description) w2m_heap_release(W2M_API_SetThreadDescription, info[i].description);
    info[i].description = copy_string((const unsigned short *)description);
    return w2m_logged(W2M_API_SetThreadDescription, h, DESCRIPTION_HR);
}

/* GetThreadDescription(hThread, ppszThreadDescription): a copy the caller frees with LocalFree. */
u64 k32_GetThreadDescription(u64 h, u64 out) {
    u32 tid = thread_of(W2M_API_GetThreadDescription, h);
    int i = info_of(tid);
    *(u64 *)out = copy_string((const unsigned short *)info[i].description);
    return w2m_logged(W2M_API_GetThreadDescription, h, DESCRIPTION_HR);
}

/* AppPolicyGetThreadInitializationType(processToken, policy): a desktop program's policy is 0 (None). */
u64 k32_AppPolicyGetThreadInitializationType(u64 token, u64 policy) {
    *(s32 *)policy = 0;
    return w2m_logged(W2M_API_AppPolicyGetThreadInitializationType, token, 0);
}

/* A sleep of ms milliseconds on the 10 MHz counter: 0 yields; otherwise the thread blocks until the counter passes
 * its deadline, other threads running meanwhile. 1: the sleep is over. */
static int slept(u32 ms) {
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    if (ms == 0) {
        w2m_state()->schedule |= W2M_SCHEDULE_YIELD;
        return 1;
    }
    if (ms == INFINITE) w2m_unimplemented(W2M_API_SleepEx, ms);             /* no APC ever ends it: not in the rows */
    if (!sc->wait_deadline) {
        w2m_block(w2m_counter_now() + (u64)ms * 10000, W2M_KEY_NONE);
        return 0;
    }
    if (w2m_counter_now() < sc->wait_deadline) {
        w2m_block(sc->wait_deadline, W2M_KEY_NONE);
        return 0;
    }
    sc->wait_deadline = 0;
    return 1;
}

u64 k32_SleepEx(u64 ms, u64 alertable) {
    (void)alertable;                               /* no APC is ever queued */
    if (!slept((u32)ms)) return 0;                 /* retried: the bridge does not return */
    return w2m_logged(W2M_API_SleepEx, (u32)ms, 0);
}

W2M_VOID u64 k32_Sleep(u64 ms) {
    if (!slept((u32)ms)) return 0;
    return w2m_logged(W2M_API_Sleep, (u32)ms, 0);
}

/* SwitchToThread: yields; the host writes the answer, nonzero when another thread ran. */
u64 k32_SwitchToThread(void) {
    w2m_state()->schedule |= W2M_SCHEDULE_YIELD | W2M_SCHEDULE_ANSWER;
    return w2m_logged(W2M_API_SwitchToThread, 0, 0);
}
