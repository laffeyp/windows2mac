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
    u64 vcpu;                        /* the vCPU that has it loaded, its index + 1; 0: none (Sprint 118) */
    u8 monitor[0x130];               /* the per-thread monitor words (scheduler.py PER_THREAD_MONITOR, E_MONITOR_AT) */
    u64 woken;                       /* the woken mark (Sprint 354; threads.rs E_WOKEN): see w2m_block */
    u8 reserved1[0x8];
    u8 state[W2M_STATE_SIZE];        /* the state block (struct w2m_state) */
};
_Static_assert(__builtin_offsetof(struct w2m_run_entry, x) == W2M_RUN_X_OFF, "run entry x");
_Static_assert(__builtin_offsetof(struct w2m_run_entry, monitor) == W2M_RUN_MONITOR_OFF, "run entry monitor");
_Static_assert(__builtin_offsetof(struct w2m_run_entry, woken) == W2M_RUN_MONITOR_OFF + 0x130, "run entry woken");
_Static_assert(__builtin_offsetof(struct w2m_run_entry, state) == W2M_RUN_STATE_OFF, "run entry state");
_Static_assert(sizeof(struct w2m_run_entry) == W2M_RUN_ENTRY, "run entry size");
struct w2m_run_table {
    u64 count, unused_current, unused_current_entry, in_vm, blocked, waiting, unused_slice_pauses, switches;
    /* waiting: vCPUs waiting in the host with no thread to run (vm_backend.py _wait_for_change; Sprint 121) */
    /* the current thread, its entry and the slice's start: on each vCPU's monitor data page since Sprint 107 */
};
#define RUN ((struct w2m_run_table *)W2M_RUN_TABLE_VA)
enum { READY, BLOCKED, ENDED };

static struct w2m_run_entry *run_entry(u64 i) {
    return (struct w2m_run_entry *)(W2M_RUN_TABLE_VA + W2M_RUN_ENTRIES_OFF + i * W2M_RUN_ENTRY);
}

static u64 *monitor_word(u64 off) { return (u64 *)(W2M_MONITOR_DATA_VA + off); }

/* This vCPU's index + 1 (its LOCK_OWNER word, on its private monitor data page). */
static u64 this_vcpu(void) { return *monitor_word(W2M_LOCK_OWNER_OFF); }

/* Whether another vCPU has the thread loaded (Sprint 118): it runs there, and no other vCPU may take it. */
static int held_elsewhere(const struct w2m_run_entry *e) { return e->vcpu && e->vcpu != this_vcpu(); }

static struct w2m_thread_scratch *scratch_of(const struct w2m_run_entry *e) {
    return (struct w2m_thread_scratch *)(e->teb + W2M_THREAD_SCRATCH_OFF);
}

static void set_status(struct w2m_run_entry *e, u64 status) {
    if ((e->status == BLOCKED) != (status == BLOCKED)) RUN->blocked += status == BLOCKED ? 1 : (u64)-1;
    e->status = status;
}

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

static u64 host(u64 sub, u64 a, u64 b, u64 c) { return w2m_host(W2M_HOST_THREAD, sub, a, b, c, 0); }

/* The wake census (Sprint 095): threads a signal made ready, waits that blocked again unsatisfied before their
 * deadline (a retried wait or sleep: its deadline set by an earlier block), and up to 16 call sites of those re-blocks
 * by return address (vm_backend.py VM.wake_census names them). */
struct w2m_wake_census { u64 readied, rewaits, sites[16][2]; } w2m_wake_census;

/* The woken mark's values (Sprint 356): WOKEN, a readying came, the host's mark (threads.rs `mark_woken`) and this
 * file's; ARMED, the thread has checked a wait under the sync objects' lock and not yet parked. 0: neither. */
enum { WOKEN = 1, ARMED = 2 };

/* Recentering step 5: a blocked thread names what it waits on, and a signal names what it signals, so the host wakes
 * only the threads that wait on it (before, every blocked thread woke on any signal, ran to find its wait unmet and
 * blocked again). Key 0 on either side wakes every blocked thread: a wait on several objects, a pipe. */
void w2m_block(u64 deadline, u64 key) {
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    if (sc->wait_deadline) {                       /* woken, and blocking again */
        u64 site = (u64)__builtin_return_address(0);
        w2m_wake_census.rewaits++;
        for (int i = 0; i < 16; i++) {
            if (w2m_wake_census.sites[i][0] == site || !w2m_wake_census.sites[i][0]) {
                w2m_wake_census.sites[i][0] = site;
                w2m_wake_census.sites[i][1]++;
                break;
            }
        }
    }
    if (!sc->wait_deadline) sc->wait_deadline = deadline;
    sc->wait_key = key;
    w2m_state()->schedule |= W2M_SCHEDULE_RETRY;
    /* Blocked now, under the runtime lock (Sprint 118): the bridge frees the lock before the switch stub or the host
     * takes the thread off its vCPU, and a signal from another vCPU in between wakes only BLOCKED threads, so a
     * thread still READY there would miss it and wait forever (the first wait_handoff runs on two vCPUs: 1 in 3
     * stopped as a deadlock). A wake before the switch leaves it READY, and the switch then does not block it.
     * The woken mark (Sprint 354, card 3 of docs/design-handles-and-sync-objects-2026-11-26.md): a readying that found
     * this thread running on its vCPU and not yet BLOCKED, between its wait's check and this park, set it; the thread
     * stays READY, the mark cleared, and the switch retries the call, which checks again (`retry` needs BLOCKED). While
     * the runtime word covers a whole call that window does not exist; card 5's waits, checked under the sync objects'
     * lock and parked under the word, open it (Sprint 356). The mark's word holds WOKEN (the host writes it too) or
     * ARMED (`w2m_wait_armed`); either is cleared here by one atomic exchange, every writer but the thread's own wait
     * holding the word this park holds; ARMED parks the thread. */
    if (RUN->count) {
        struct w2m_run_entry *c = run_entry(*monitor_word(W2M_CURRENT_OFF));
        if (__atomic_exchange_n(&c->woken, 0, __ATOMIC_RELAXED) != WOKEN) set_status(c, BLOCKED);
        c->deadline = sc->wait_deadline;
    }
}

/* The running thread's mark armed (Sprint 356; Sprint 354's Deferred (a)), by its wait under `sync_lock`, in the
 * holding that checked its objects and made its entries: a release that comes after that holding finds the entries,
 * and its readying, under the runtime word, finds the thread BLOCKED, or not yet parked and ARMED, and marks it WOKEN.
 * A store, relaxed: `sync_lock`'s release orders it before any release that finds the entries, and the readying's word
 * after. */
void w2m_wait_armed(void) {
    if (RUN->count) __atomic_store_n(&run_entry(*monitor_word(W2M_CURRENT_OFF))->woken, ARMED, __ATOMIC_RELAXED);
}

/* The running thread marked WOKEN (Sprint 356): its park leaves it ready and the switch retries its call at once; a
 * wait for all naming one of the host's objects, the word held, whose check found an object signaled after it read it
 * unset (sync_objects.c `wait_all_held`). */
void w2m_wait_again(void) {
    if (RUN->count) __atomic_store_n(&run_entry(*monitor_word(W2M_CURRENT_OFF))->woken, WOKEN, __ATOMIC_RELAXED);
}

/* The woken mark set on a thread a readying finds not BLOCKED and ARMED (w2m_block): one between its wait's check and
 * its park, on its own vCPU. Before Sprint 356 every thread another vCPU held was marked (`held_elsewhere`), one
 * running guest code included, which spent the mark on one spare retry of its next park (Sprint 354's Deferred (a));
 * the waits that check outside the runtime word (sync_objects.c) are the only ones that arm, every other wait checking
 * and parking within one holding of the word. A compare-and-swap of ARMED for WOKEN, under the word: the park's
 * exchange holds the word too, and a wait arms with no word held, never while a readying meant for it can see its
 * entries. */
static void mark_woken(struct w2m_run_entry *e) {
    u64 armed = ARMED;
    if (e->status != BLOCKED)
        __atomic_compare_exchange_n(&e->woken, &armed, WOKEN, 0, __ATOMIC_RELAXED, __ATOMIC_RELAXED);
}

/* A thread made ready inside the VM (Sprint 121). A vCPU waiting in the host for a thread learns of it only from the
 * host, so while one waits (RUN->waiting) the host is asked to wake it, and on resuming this vCPU the host ends its
 * slice only for threads readied beyond the waiting vCPUs (vm_backend.py _slice_until). With none waiting, the running
 * thread's slice ends at its next stop (SLICE_UNTIL), and this vCPU switches to the readied thread. Before, the vCPU
 * that readied a thread always switched to it, and a waiting vCPU learned of it at the next host serve: a worker
 * readied 20 times ran its rounds on the waiting vCPU in 3 runs of 13 (the readied guest, tests/data/timed_waits). */
static void readied(void) {
    if (RUN->waiting) host(W2M_THREAD_WAKE, 0, 0, 0);
    else *monitor_word(W2M_SLICE_UNTIL_OFF) = 0;
}

/* The waiters on `key` ready: those whose wait takes it, any signal (their key 0), or every key (`key` 0). A thread
 * between its wait's check and its park has written no key yet (w2m_block writes it), so every ARMED thread takes the
 * woken mark (Sprint 354; Sprint 356 the armed alone), whatever it will wait on: one whose wait is on another key
 * retries its call once and parks again. With no thread blocked, the mark is all a signal does. */
void w2m_signal(u64 key) {
    int woke = 0;
    for (u64 i = 0; i < RUN->count; i++) {
        struct w2m_run_entry *e = run_entry(i);
        if (e->status != BLOCKED) {
            mark_woken(e);
            continue;
        }
        u64 waits_on = scratch_of(e)->wait_key;
        if (!key || !waits_on || waits_on == key) {
            set_status(e, READY);
            w2m_wake_census.readied++;
            woke = 1;
        }
    }
    if (woke) readied();
}

/* Sprint 095: a release that hands its object to one waiter (sync_objects.c grant) readies that thread alone; the
 * other waiters on the object stay blocked, as Windows releases "a single waiting thread" (SetEvent, auto-reset) and
 * one waiter per unit (ReleaseSemaphore). Before, every waiter on the key woke and all but one blocked again (the
 * wake census: 11,086 of 186,272 readies on the batch run, 99.5% in wait_one). A waiter not yet BLOCKED takes the woken
 * mark (Sprint 354), when it is ARMED (Sprint 356): its grant is made, and its park must not lose the readying. */
void w2m_ready_thread(u32 tid) {
    for (u64 i = 0; i < RUN->count; i++) {
        struct w2m_run_entry *e = run_entry(i);
        if (e->tid != tid) continue;
        if (e->status != BLOCKED) {
            mark_woken(e);
            return;
        }
        set_status(e, READY);
        w2m_wake_census.readied++;
        readied();
        return;
    }
}

/* vm_backend.py _wake_expired: a blocked thread whose deadline has passed is ready, on the wall clock as on the
 * instruction clock (Sprint 120: on the wall clock a 20 ms Sleep beside a spinning thread never ended). The host's
 * clock (mode 1) is read by a host call, which the switch does not make: the host decides there (lockstep runs do not
 * switch in the VM). */
static void wake_expired(void) {
    if (*monitor_word(W2M_CLOCK_MODE_OFF) == 1) return;
    u64 now = w2m_counter_now();
    for (u64 i = 0; i < RUN->count; i++) {
        struct w2m_run_entry *e = run_entry(i);
        if (e->status == BLOCKED && e->deadline != ~0ull && e->deadline <= now) set_status(e, READY);
    }
}

/* vm_backend.py _slice_until, for a run the VM may switch in: 0 when another unsuspended thread is ready; the count
 * at which the earliest deadline of another blocked thread falls on the instruction clock (0 on another clock); ~0
 * when no other thread waits with one. And 0 while another vCPU holds a thread (Sprint 353): that vCPU readies and
 * puts down threads in the VM, which writes only its own SLICE_UNTIL, so this vCPU's slice ends at each TIME_SLICE
 * and its switch looks again; before, three_counters' second thread ran alone on vCPU 0 for the whole run, its
 * SLICE_UNTIL ~0, while vCPU 1 put the first down ready at each of its own slices' ends. On one vCPU no thread is
 * held elsewhere. The host's `readied` (threads.rs) ends another vCPU's slice only when more free threads are ready
 * than vCPUs wait, since the host sees each readying it makes; this rule ends it whenever another vCPU holds a thread,
 * ready or not, since a readying in the VM writes no other vCPU's word and this vCPU cannot tell it came. Two busy
 * threads on two vCPUs so switch in the VM at each TIME_SLICE with nothing else ready: two_counters, a scratch guest
 * of two threads counting to 300,000,000, took 155,665,372 to 165,960,372 ticks of 100 ns in 5 runs against
 * 149,638,326 to 153,964,835 before. */
static u64 slice_until(u64 cur) {
    u64 nearest = ~0ull;
    int timed = 0, elsewhere = 0;
    for (u64 i = 0; i < RUN->count; i++) {
        const struct w2m_run_entry *e = run_entry(i);
        if (i == cur) continue;
        if (held_elsewhere(e)) {
            elsewhere = 1;
            continue;
        }
        if (e->suspend) continue;
        if (e->status == READY) return 0;
        if (e->status == BLOCKED && e->deadline != ~0ull) {
            timed = 1;
            if (e->deadline < nearest) nearest = e->deadline;
        }
    }
    if (elsewhere) return 0;
    if (!timed) return ~0ull;
    if (*monitor_word(W2M_CLOCK_MODE_OFF) != 2) return 0;
    u64 rate = *monitor_word(W2M_CLOCK_RATE_OFF), warp = *(volatile u64 *)W2M_CLOCK_WARP_VA;
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
 * limit, one thread, the thread suspended from another vCPU, no thread ready and none to warp to. */
u64 w2m_switch(u64 schedule, u64 stop) {
    struct w2m_run_table *rt = RUN;
    u64 n = rt->count, cur = *monitor_word(W2M_CURRENT_OFF);
    u64 *count = monitor_word(0), *limit = monitor_word(8);
    if (!rt->in_vm || (schedule & W2M_SCHEDULE_END) || n < 2) return 0;
    struct w2m_run_entry *c = run_entry(cur);
    if (c->suspend) return 0;          /* suspended from another vCPU: the host puts it down (Sprint 119) */
    int retry = (schedule & W2M_SCHEDULE_RETRY) && c->status == BLOCKED;   /* w2m_block's, unless woken since */
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
        if (!(i == cur && retry) && e->status == READY && !e->suspend && !held_elsewhere(e)) next = i;
    }
    u64 warped = n;
    if (next == n) {
        if (!retry || *monitor_word(W2M_CLOCK_MODE_OFF) != 2) return 0;
        warped = next = nearest_deadline(cur, scratch_of(c)->wait_deadline);
        if (next == n) return 0;
    }
    if (warped != n) {
        struct w2m_run_entry *w = run_entry(warped);
        u64 now = w2m_counter_now();
        if (w->deadline > now) *(volatile u64 *)W2M_CLOCK_WARP_VA += w->deadline - now;
        set_status(w, READY);
    }
    if (schedule & W2M_SCHEDULE_ANSWER) c->x[0] = next != cur;
    w2m_state()->schedule = 0;
    if (next != cur) {
        struct w2m_run_entry *e = run_entry(next);
        u64 pauses = *monitor_word(W2M_PAUSE_COUNT_OFF);
        c->instructions += *count - *monitor_word(W2M_SLICE_COUNT_OFF);
        c->pauses += pauses - *monitor_word(W2M_SLICE_PAUSES_OFF);
        *monitor_word(W2M_SLICE_COUNT_OFF) = *count;
        *monitor_word(W2M_SLICE_PAUSES_OFF) = pauses;
        monitor_words(c->monitor, 1);
        __builtin_memcpy(c->state, (const void *)W2M_STATE_VA, W2M_STATE_SIZE);
        monitor_words(e->monitor, 0);
        __builtin_memcpy((void *)W2M_STATE_VA, e->state, W2M_STATE_SIZE);
        *monitor_word(W2M_CURRENT_OFF) = next;
        *monitor_word(W2M_CURRENT_ENTRY_OFF) = (u64)e;
        c->vcpu = 0;
        e->vcpu = this_vcpu();
        e->woken = 0;                  /* loaded: in no wait's window, its call checked again (w2m_block, mark_woken);
                                        * a host mark taken while it ran guest code elsewhere goes here (Sprint 356) */
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

void w2m_mutexes_abandoned(u32 tid);          /* sync_objects.c (Sprint 356's amendment) */
void w2m_apcs_ended(u32 tid);                 /* sync_objects.c (Sprint 370) */

/* The last call of every thread but the first: the exit code is final, waiters on its handle wake. Every mutex it
 * still owns is abandoned first (Sprint 356's amendment), its first waiter granted it with WAIT_ABANDONED; its APCs are
 * lost and a timer it set with a completion routine is canceled (Sprint 370). */
W2M_VOID u64 k32_W2mThreadEnd(void) {
    w2m_apcs_ended((u32)w2m_teb()->client_thread);
    w2m_mutexes_abandoned((u32)w2m_teb()->client_thread);
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

/* A thread suspended from another vCPU while it was in a call (vm_backend.py _await_put_down, Sprint 120) runs no
 * instruction after it: the call returns to the switch, which declines a suspended thread, and the host puts it down. */
static void put_down_if_suspended(void) {
    if (RUN->count && run_entry(*monitor_word(W2M_CURRENT_OFF))->suspend) w2m_state()->schedule |= W2M_SCHEDULE_YIELD;
}

u64 k32_SuspendThread(u64 h) {
    u32 tid = thread_of(W2M_API_SuspendThread, h);
    if (tid == (u32)w2m_teb()->client_thread) w2m_unimplemented(W2M_API_SuspendThread, h);   /* itself: not in the rows */
    u64 previous = host(W2M_THREAD_SUSPEND, tid, 0, 0);
    if (previous == ~0ull) {                       /* ended */
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_SuspendThread, h, 0xFFFFFFFFu);
    }
    put_down_if_suspended();
    return w2m_logged(W2M_API_SuspendThread, h, previous);
}

u64 k32_ResumeThread(u64 h) {
    u32 tid = thread_of(W2M_API_ResumeThread, h);
    return w2m_logged(W2M_API_ResumeThread, h, host(W2M_THREAD_RESUME, tid, 0, 0));
}

/* GetThreadContext(hThread, lpContext): the host fills the CONTEXT from the thread's saved state, as its
 * ContextFlags ask; the thread must be another. On one vCPU a thread suspended while it runs was stopped by its
 * time slice at an x86 instruction boundary, so the saved state is its registers there (rows threads.context: a
 * spinner's RIP and RSP; probes/running_thread_context, every register). On several vCPUs SuspendThread returns once
 * the thread's vCPU has put it down (vm_backend.py _stop_elsewhere, Sprint 119), or sooner when the caller is
 * suspended meanwhile, and GetThreadContext then waits for it the same way (Sprint 120). */
u64 k32_GetThreadContext(u64 h, u64 context) {
    u32 tid = thread_of(W2M_API_GetThreadContext, h);
    if (tid == (u32)w2m_teb()->client_thread) w2m_unimplemented(W2M_API_GetThreadContext, h);   /* itself */
    /* This thread suspended while it waited for the other's put-down (Sprint 120): it is put down at the call and calls
     * again once resumed, so no host waits holding a suspended thread and every such wait ends. Windows instead
     * completes the call and stops the thread on its return to user mode, so the CONTEXT is written before the
     * resume there and after it here; no row reads it in between. */
    if (!host(W2M_THREAD_CONTEXT, tid, context, 0)) {
        w2m_state()->schedule |= W2M_SCHEDULE_RETRY;
        return 0;
    }
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
    if (ms == INFINITE) w2m_unimplemented(W2M_API_SleepEx, ms);   /* not alertable, nothing ends it: not in the rows */
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

int w2m_apcs_queued(void);                    /* sync_objects.c (Sprint 370) */
u64 w2m_sleep_alertable(u32 ms, u64 rsp);

/* SleepEx(dwMilliseconds, bAlertable), the stack pointer the bridge's fifth argument: not alertable, or alertable with
 * no APC queued to any thread and a finite time, the sleep above, as before Sprint 370; alertable with an APC queued, or
 * for INFINITE, sync_objects.c's, which runs the thread's due APCs and answers WAIT_IO_COMPLETION: "If an APC is queued
 * to the thread (QueueUserAPC), the function returns when either the time-out period has elapsed or when the APC
 * function is called" (Microsoft Learn, SleepEx). Not alertable, "If an APC is queued to the thread, the function does
 * not immediately return and the APC function is not executed." */
u64 k32_SleepEx(u64 ms, u64 alertable, u64 unused2, u64 unused3, u64 rsp) {
    (void)unused2, (void)unused3;
    if ((u32)alertable && (w2m_apcs_queued() || (u32)ms == INFINITE)) return w2m_sleep_alertable((u32)ms, rsp);
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
