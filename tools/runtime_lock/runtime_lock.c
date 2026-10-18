// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* runtime_lock.c — the host's side of the guest runtime lock (Sprint 107; docs/design-several-vcpus.md).
 *
 * The lock is a 32-bit word on the run table's page (vm_backend.py RUNTIME_LOCK): 0 free, else the holder's vCPU index
 * + 1. Guest code takes it with CASA and frees it with STLR (vm_backend.py lock_runtime, unlock_runtime). The host
 * takes it before it changes what the runtime's C reads under it (the run table, the reconstruction table), with the
 * owner its own value. Python calls these through ctypes.CDLL, which releases the GIL around each call (Python 3.12
 * ctypes documentation), so a host thread waiting here does not stop the host thread whose vCPU holds the lock. */
#include <stdint.h>

/* Take the lock for `owner`: compare-and-swap 0 for `owner` with acquire order, reading plainly between tries. Never
 * an exchange, which would write `owner` over another holder's value: the word names its holder (vm_backend.py
 * lock_runtime, RuntimeLock; Sprint 118). */
void w2m_runtime_lock_take(volatile uint32_t *word, uint32_t owner) {
    for (;;) {
        uint32_t expected = 0;
        if (__atomic_compare_exchange_n(word, &expected, owner, 0, __ATOMIC_ACQUIRE, __ATOMIC_RELAXED)) return;
        while (__atomic_load_n(word, __ATOMIC_RELAXED) != 0) __builtin_arm_yield();
    }
}

/* Free the lock with release order: every store made under it is seen before the word reads 0. */
void w2m_runtime_lock_give(volatile uint32_t *word) { __atomic_store_n(word, 0, __ATOMIC_RELEASE); }

/* The holder's value, 0 when free. */
uint32_t w2m_runtime_lock_holder(volatile uint32_t *word) { return __atomic_load_n(word, __ATOMIC_ACQUIRE); }
