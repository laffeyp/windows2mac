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

u64 k32_AcquireSRWLockExclusive(u64 lock) {
    if (*(u64 *)lock) w2m_unimplemented(W2M_API_AcquireSRWLockExclusive, *(u64 *)lock);   /* would wait */
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
    return w2m_logged(W2M_API_ReleaseSRWLockExclusive, lock, 0);
}

u64 k32_AcquireSRWLockShared(u64 lock) {
    u64 w = *(u64 *)lock;
    if (w == EXCLUSIVE) w2m_unimplemented(W2M_API_AcquireSRWLockShared, w);              /* would wait */
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
    return w2m_logged(W2M_API_ReleaseSRWLockShared, lock, 0);
}

u64 k32_SleepConditionVariableSRW(u64 cv, u64 lock, u64 ms, u64 flags) {
    if ((u32)ms == INFINITE || (u32)flags != 0 || *(u64 *)lock != EXCLUSIVE)
        w2m_unimplemented(W2M_API_SleepConditionVariableSRW, (u32)ms);
    (void)cv;
    set_error(ERROR_TIMEOUT);
    return w2m_logged(W2M_API_SleepConditionVariableSRW, (u32)ms, 0);
}

u64 k32_WakeConditionVariable(u64 cv) {
    if (*(u64 *)cv) w2m_unimplemented(W2M_API_WakeConditionVariable, *(u64 *)cv);   /* a waiter: Sprint 047 */
    return w2m_logged(W2M_API_WakeConditionVariable, cv, 0);
}

u64 k32_WakeAllConditionVariable(u64 cv) {
    if (*(u64 *)cv) w2m_unimplemented(W2M_API_WakeAllConditionVariable, *(u64 *)cv);
    return w2m_logged(W2M_API_WakeAllConditionVariable, cv, 0);
}
