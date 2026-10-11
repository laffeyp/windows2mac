// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* slim_locks.c — SRW locks and condition variables on one thread (Sprint 046, family "slim locks").
 *
 * contracts/windows.sync.slim_locks records Windows on four runners (8 identical outputs), one thread:
 *   - InitializeSRWLock and InitializeConditionVariable write a zero word
 *   - an exclusive holder makes the word 1; n shared holders make it (n << 4) | 1; the last release, 0
 *   - TryAcquireSRWLockExclusive: TRUE on a free lock (word 1), FALSE while held exclusive or shared;
 *     TryAcquireSRWLockShared: FALSE while held exclusive; neither touches the last error
 *   - SleepConditionVariableSRW on a held lock with a timeout of 0 or 1 ms: FALSE, ERROR_TIMEOUT, the
 *     lock held again after and both words as before; Wake(All)ConditionVariable with no waiter: nothing
 * Every other form waits for another thread (an acquire of a held lock, an INFINITE sleep) or is not in
 * the rows: it stops the run (waits between threads are Sprint 047's). The finite sleep returns at once:
 * no other thread can wake it, and w2m does not yet pass the time.
 */
#include "runtime.h"

#define EXCLUSIVE 1ull
#define SHARED_ONE 0x10ull
#define ERROR_TIMEOUT 1460u
#define INFINITE 0xFFFFFFFFu

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

u64 k32_InitializeSRWLock(u64 lock) {
    *(u64 *)lock = 0;
    return w2m_logged(W2M_API_InitializeSRWLock, lock, 0);
}

u64 k32_InitializeConditionVariable(u64 cv) {
    *(u64 *)cv = 0;
    return w2m_logged(W2M_API_InitializeConditionVariable, cv, 0);
}

/* Sprint 051: with threads, an acquire of a held lock blocks the thread until a release (runtime.h
 * W2M_SCHEDULE_RETRY), and a release wakes the blocked threads to try again. */
static void must_wait(void) { w2m_block(~0ull); }

u64 k32_AcquireSRWLockExclusive(u64 lock) {
    if (*(u64 *)lock) {
        must_wait();
        return 0;                                   /* retried: the bridge does not return */
    }
    w2m_thread_scratch()->wait_deadline = 0;
    *(u64 *)lock = EXCLUSIVE;
    return w2m_logged(W2M_API_AcquireSRWLockExclusive, lock, 0);
}

u64 k32_TryAcquireSRWLockExclusive(u64 lock) {
    u64 got = *(u64 *)lock == 0;
    if (got) *(u64 *)lock = EXCLUSIVE;
    return w2m_logged(W2M_API_TryAcquireSRWLockExclusive, lock, got);
}

u64 k32_ReleaseSRWLockExclusive(u64 lock) {
    if (*(u64 *)lock != EXCLUSIVE) w2m_unimplemented(W2M_API_ReleaseSRWLockExclusive, *(u64 *)lock);
    *(u64 *)lock = 0;
    w2m_signal();
    return w2m_logged(W2M_API_ReleaseSRWLockExclusive, lock, 0);
}

u64 k32_AcquireSRWLockShared(u64 lock) {
    u64 w = *(u64 *)lock;
    if (w == EXCLUSIVE) {
        must_wait();
        return 0;
    }
    w2m_thread_scratch()->wait_deadline = 0;
    *(u64 *)lock = (w ? w + SHARED_ONE : SHARED_ONE | EXCLUSIVE);
    return w2m_logged(W2M_API_AcquireSRWLockShared, lock, 0);
}

u64 k32_TryAcquireSRWLockShared(u64 lock) {
    if (*(u64 *)lock != EXCLUSIVE) w2m_unimplemented(W2M_API_TryAcquireSRWLockShared, *(u64 *)lock);   /* not in the rows */
    return w2m_logged(W2M_API_TryAcquireSRWLockShared, lock, 0);
}

u64 k32_ReleaseSRWLockShared(u64 lock) {
    u64 w = *(u64 *)lock;
    if (w == 0 || w == EXCLUSIVE) w2m_unimplemented(W2M_API_ReleaseSRWLockShared, w);
    *(u64 *)lock = w == (SHARED_ONE | EXCLUSIVE) ? 0 : w - SHARED_ONE;
    if (!*(u64 *)lock) w2m_signal();
    return w2m_logged(W2M_API_ReleaseSRWLockShared, lock, 0);
}

/* Condition variables (Sprint 051): each one's wake count and waiters, by address; the variable's own word stays 0
 * (the rows hold it only without a waiter). A wake moves the count on, so every waiter of it wakes (Windows allows
 * spurious wakeups: SleepConditionVariableSRW's page). */
#define VARIABLES 256
#define CONDITION_VARIABLE_LOCKMODE_SHARED 1u
static struct { u64 cv; u64 generation; u32 waiters; } variables[VARIABLES];

static int variable_of(u64 cv, int add) {
    for (int i = 0; i < VARIABLES; i++) if (variables[i].cv == cv) return i;
    if (!add) return -1;
    for (int i = 0; i < VARIABLES; i++)
        if (!variables[i].waiters) { variables[i].cv = cv; variables[i].generation = 0; return i; }
    w2m_unimplemented(W2M_API_SleepConditionVariableSRW, VARIABLES);
    return -1;
}

static int lock_held(u64 lock, int shared) {
    u64 w = *(u64 *)lock;
    return shared ? (w != 0 && w != EXCLUSIVE) : w == EXCLUSIVE;
}

static int lock_free(u64 lock, int shared) {
    u64 w = *(u64 *)lock;
    return shared ? w != EXCLUSIVE : w == 0;
}

/* SleepConditionVariableSRW(ConditionVariable, SRWLock, dwMilliseconds, Flags): the lock released, a wait for a
 * wake or the timeout, then the lock taken again (blocking until free); TRUE when woken, FALSE and ERROR_TIMEOUT
 * when the time ran out. Retried until done; its progress is in the TEB (runtime.h w2m_thread_scratch.phase:
 * 1 waiting, 2 woken, 3 timed out). */
u64 k32_SleepConditionVariableSRW(u64 cv, u64 lock, u64 ms, u64 flags) {
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    int shared = ((u32)flags & CONDITION_VARIABLE_LOCKMODE_SHARED) != 0;
    if ((u32)flags & ~CONDITION_VARIABLE_LOCKMODE_SHARED) w2m_unimplemented(W2M_API_SleepConditionVariableSRW, flags);
    if (sc->phase == 0) {
        if (!lock_held(lock, shared)) w2m_unimplemented(W2M_API_SleepConditionVariableSRW, *(u64 *)lock);
        *(u64 *)lock = shared ? (*(u64 *)lock == (SHARED_ONE | EXCLUSIVE) ? 0 : *(u64 *)lock - SHARED_ONE) : 0;
        w2m_signal();
        int v = variable_of(cv, 1);
        variables[v].waiters++;
        sc->wait_cv = cv;
        sc->wait_generation = variables[v].generation;
        sc->wait_deadline = 0;
        sc->phase = 1;
        if ((u32)ms == 0) {
            variables[v].waiters--;
            sc->phase = 3;
        }
    }
    if (sc->phase == 1) {
        int v = variable_of(cv, 0);
        if (variables[v].generation != sc->wait_generation) {
            variables[v].waiters--;
            sc->wait_deadline = 0;
            sc->phase = 2;
        } else if (!sc->wait_deadline) {
            w2m_block((u32)ms == INFINITE ? ~0ull : w2m_counter_now() + (u64)(u32)ms * 10000);
            return 0;
        } else if (sc->wait_deadline != ~0ull && w2m_counter_now() >= sc->wait_deadline) {
            variables[v].waiters--;
            sc->wait_deadline = 0;
            sc->phase = 3;
        } else {
            w2m_block(sc->wait_deadline);
            return 0;
        }
    }
    if (!lock_free(lock, shared)) {                       /* phases 2 and 3: the lock again */
        must_wait();
        return 0;
    }
    sc->wait_deadline = 0;
    *(u64 *)lock = shared ? (*(u64 *)lock ? *(u64 *)lock + SHARED_ONE : SHARED_ONE | EXCLUSIVE) : EXCLUSIVE;
    int woken = sc->phase == 2;
    sc->phase = 0;
    sc->wait_cv = 0;
    if (!woken) set_error(ERROR_TIMEOUT);
    return w2m_logged(W2M_API_SleepConditionVariableSRW, (u32)ms, woken);
}

static u64 wake(u64 api, u64 cv) {
    int v = variable_of(cv, 0);
    if (v >= 0 && variables[v].waiters) {
        variables[v].generation++;
        w2m_signal();
    }
    return w2m_logged(api, cv, 0);
}

u64 k32_WakeConditionVariable(u64 cv) { return wake(W2M_API_WakeConditionVariable, cv); }
u64 k32_WakeAllConditionVariable(u64 cv) { return wake(W2M_API_WakeAllConditionVariable, cv); }
