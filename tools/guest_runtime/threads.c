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
 * The threads run on one vCPU, switching at instruction boundaries: a service that must wait sets state.schedule
 * (runtime.h) and the bridge hands the vCPU over, and a time slice ends at a block's stop. w2m_switch below makes the
 * switch inside the VM by the host's rule (src/w2m/vm_backend.py _schedule, _switch; Sprint 069) and leaves to the
 * host what needs it: a thread's end, a wait with no thread ready, lockstep, a machine of several processes.
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

/* The run table (src/w2m/scheduler.py RT_*, E_*; Sprint 069): each thread's saved state, status, suspend count,
 * deadline and census counters, in thread id order, which the host reads and writes as this file does. */
struct w2m_run_entry {
    u64 tid, status, suspend, deadline, teb, vslot, instructions, pauses;   /* vslot: its vectors' offset in the file */
    u64 x[31];                       /* x0-x30 at an x86 instruction boundary; x17 the RIP, x23 the budget */
    u64 reserved0;
    u8 monitor[0x130];               /* the per-thread monitor words (scheduler.py PER_THREAD_MONITOR, E_MONITOR_AT) */
    u8 reserved1[0x10];
    u8 state[W2M_STATE_SIZE];        /* the state block (struct w2m_state) */
};
_Static_assert(__builtin_offsetof(struct w2m_run_entry, x) == W2M_RUN_X_OFF, "run entry x");
_Static_assert(__builtin_offsetof(struct w2m_run_entry, monitor) == W2M_RUN_MONITOR_OFF, "run entry monitor");
_Static_assert(__builtin_offsetof(struct w2m_run_entry, state) == W2M_RUN_STATE_OFF, "run entry state");
_Static_assert(sizeof(struct w2m_run_entry) == W2M_RUN_ENTRY, "run entry size");
struct w2m_run_table {
    u64 count, current, current_entry, in_vm, blocked, slice_count, slice_pauses, switches;
};
#define RUN ((struct w2m_run_table *)W2M_RUN_TABLE_VA)
enum { READY, BLOCKED, ENDED };

static struct w2m_run_entry *run_entry(u64 i) {
    return (struct w2m_run_entry *)(W2M_RUN_TABLE_VA + W2M_RUN_ENTRIES_OFF + i * W2M_RUN_ENTRY);
}

static u64 *monitor_word(u64 off) { return (u64 *)(W2M_MONITOR_DATA_VA + off); }

static struct w2m_thread_scratch *scratch_of(const struct w2m_run_entry *e) {
    return (struct w2m_thread_scratch *)(e->teb + W2M_THREAD_SCRATCH_OFF);
}

static void set_status(struct w2m_run_entry *e, u64 status) {
    if ((e->status == BLOCKED) != (status == BLOCKED)) RUN->blocked += status == BLOCKED ? 1 : (u64)-1;
    e->status = status;
}

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

/* The waiters on `key` ready: those whose wait takes it, any signal (their key 0), or every key (`key` 0). A thread
 * ready besides the running one ends the running one's slice at its next stop (SLICE_UNTIL). */
void w2m_signal(u64 key) {
    if (!RUN->blocked) return;
    int woke = 0;
    for (u64 i = 0; i < RUN->count; i++) {
        struct w2m_run_entry *e = run_entry(i);
        if (e->status != BLOCKED) continue;
        u64 waits_on = scratch_of(e)->wait_key;
        if (!key || !waits_on || waits_on == key) {
            set_status(e, READY);
            woke = 1;
        }
    }
    if (woke) *monitor_word(W2M_SLICE_UNTIL_OFF) = 0;
}

/* vm_backend.py _wake_expired: on the instruction clock, a blocked thread whose deadline has passed is ready. */
static void wake_expired(void) {
    if (*monitor_word(W2M_CLOCK_MODE_OFF) != 2) return;
    u64 now = w2m_counter_now();
    for (u64 i = 0; i < RUN->count; i++) {
        struct w2m_run_entry *e = run_entry(i);
        if (e->status == BLOCKED && e->deadline != ~0ull && e->deadline <= now) set_status(e, READY);
    }
}

/* vm_backend.py _slice_until, for a run the VM may switch in: 0 when another unsuspended thread is ready; the count
 * at which the earliest deadline of another blocked thread falls on the instruction clock (0 on another clock); ~0
 * when no other thread waits with one. */
static u64 slice_until(u64 cur) {
    u64 nearest = ~0ull;
    int timed = 0;
    for (u64 i = 0; i < RUN->count; i++) {
        const struct w2m_run_entry *e = run_entry(i);
        if (i == cur || e->suspend) continue;
        if (e->status == READY) return 0;
        if (e->status == BLOCKED && e->deadline != ~0ull) {
            timed = 1;
            if (e->deadline < nearest) nearest = e->deadline;
        }
    }
    if (!timed) return ~0ull;
    if (*monitor_word(W2M_CLOCK_MODE_OFF) != 2) return 0;
    u64 rate = *monitor_word(W2M_CLOCK_RATE_OFF), warp = *monitor_word(W2M_CLOCK_WARP_OFF);
    if (nearest <= warp) return 0;
    u64 ticks = nearest - warp;
    return ticks / 10000000 * rate + ticks % 10000000 * rate / 10000000;
}

/* The instructions of the block whose stop called the slice stub (`stop` its HVC #4): the immediate of its entry's
 * SUBS x23, x23, #n, two words before the body the stop's next word branches to (vm_backend.py _stop_block_length). */
static u64 stop_block_length(u64 stop) {
    u32 b = *(const u32 *)(stop + 4);
    long imm26 = (long)(b & 0x3FFFFFF);
    if (imm26 >> 25) imm26 -= 1l << 26;
    u32 subs = *(const u32 *)(stop + 4 + 4 * imm26 - 8);
    return subs >> 10 & 0xFFF;
}

static void monitor_words(u8 *entry, int to_entry) {
    static const u64 ranges[4][3] = {
        {W2M_RUN_MONITOR_0, W2M_RUN_MONITOR_1, W2M_RUN_MONITOR_2}, {W2M_RUN_MONITOR_3, W2M_RUN_MONITOR_4, W2M_RUN_MONITOR_5},
        {W2M_RUN_MONITOR_6, W2M_RUN_MONITOR_7, W2M_RUN_MONITOR_8}, {W2M_RUN_MONITOR_9, W2M_RUN_MONITOR_10, W2M_RUN_MONITOR_11}};
    for (int r = 0; r < 4; r++) {
        u8 *monitor = (u8 *)(W2M_MONITOR_DATA_VA + ranges[r][0]);
        u64 n = ranges[r][1] - ranges[r][0];
        if (to_entry) __builtin_memcpy(entry + ranges[r][2], monitor, n);
        else __builtin_memcpy(monitor, entry + ranges[r][2], n);
    }
}

/* vm_backend.py warp_to_deadline, for a wait no ready thread follows: the unsuspended blocked thread with the earliest
 * finite deadline, the lower thread id first on a tie, counting the current thread (`cur`) as blocked until `deadline`;
 * the run table's count when none waits with one. */
static u64 nearest_deadline(u64 cur, u64 deadline) {
    u64 first = RUN->count, nearest = ~0ull;
    for (u64 i = 0; i < RUN->count; i++) {
        const struct w2m_run_entry *e = run_entry(i);
        u64 d = i == cur ? deadline : e->deadline;
        if ((i == cur || e->status == BLOCKED) && !e->suspend && d != ~0ull && d < nearest) {
            nearest = d;
            first = i;
        }
    }
    return first;
}

/* The switch stub's decision (vm_backend.py _switch_stub), with the running thread's registers in its run table entry
 * and the count written: `schedule` 0 at a slice's end (`stop` its block's HVC #4), else state.schedule's bits. The
 * host's rule (_schedule, _switch): at a slice's end the budget refilled and timed-out waits made ready first, the
 * thread's RIP its block's entry and the block's instructions given back; RETRY blocks the thread with its wait's
 * deadline; the next thread is the first ready, unsuspended one after it in thread id order, itself last unless it
 * blocks; when a wait leaves no thread ready, on the instruction clock the clock moves to the earliest deadline and
 * that thread runs (warp_to_deadline). 1: the current entry (another's after a switch) holds the registers to go on
 * with, x23 its budget. 0: the host decides, nothing changed: switching is off for this run, a thread's end, the run's
 * limit, one thread, no thread ready and none to warp to. */
u64 w2m_switch(u64 schedule, u64 stop) {
    struct w2m_run_table *rt = RUN;
    u64 n = rt->count, cur = rt->current;
    u64 *count = monitor_word(0), *limit = monitor_word(8);
    if (!rt->in_vm || (schedule & W2M_SCHEDULE_END) || n < 2) return 0;
    struct w2m_run_entry *c = run_entry(cur);
    int retry = (schedule & W2M_SCHEDULE_RETRY) != 0;
    u64 next = n;
    if (!schedule) {
        u64 end = *monitor_word(W2M_RUN_END_OFF);
        if (*count >= end) return 0;
        *limit = *count + W2M_TIME_SLICE < end ? *count + W2M_TIME_SLICE : end;
        wake_expired();
        c->x[17] = w2m_guest_rip_at(stop);
        *count -= stop_block_length(stop);
    }
    for (u64 k = 1; k <= n && next == n; k++) {
        u64 i = (cur + k) % n;
        const struct w2m_run_entry *e = run_entry(i);
        if (!(i == cur && retry) && e->status == READY && !e->suspend) next = i;
    }
    u64 warped = n;
    if (next == n) {
        if (!retry || *monitor_word(W2M_CLOCK_MODE_OFF) != 2) return 0;
        warped = next = nearest_deadline(cur, scratch_of(c)->wait_deadline);
        if (next == n) return 0;
    }
    if (retry) {
        set_status(c, BLOCKED);
        c->deadline = scratch_of(c)->wait_deadline;
    }
    if (warped != n) {
        struct w2m_run_entry *w = run_entry(warped);
        u64 now = w2m_counter_now();
        if (w->deadline > now) *monitor_word(W2M_CLOCK_WARP_OFF) += w->deadline - now;
        set_status(w, READY);
    }
    if (schedule & W2M_SCHEDULE_ANSWER) c->x[0] = next != cur;
    w2m_state()->schedule = 0;
    if (next != cur) {
        struct w2m_run_entry *e = run_entry(next);
        u64 pauses = *monitor_word(W2M_PAUSE_COUNT_OFF);
        c->instructions += *count - rt->slice_count;
        c->pauses += pauses - rt->slice_pauses;
        rt->slice_count = *count;
        rt->slice_pauses = pauses;
        monitor_words(c->monitor, 1);
        __builtin_memcpy(c->state, (const void *)W2M_STATE_VA, W2M_STATE_SIZE);
        monitor_words(e->monitor, 0);
        __builtin_memcpy((void *)W2M_STATE_VA, e->state, W2M_STATE_SIZE);
        rt->current = next;
        rt->current_entry = (u64)e;
        rt->switches++;
        c = e;
    }
    c->x[23] = *limit - *count;
    *monitor_word(W2M_SLICE_UNTIL_OFF) = slice_until(next);
    return 1;
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
