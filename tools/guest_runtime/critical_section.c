// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* critical_section.c — CRITICAL_SECTION on one thread (Sprint 044, family "critical sections").
 *
 * InitializeCriticalSectionEx, InitializeCriticalSectionAndSpinCount, InitializeCriticalSection,
 * EnterCriticalSection, TryEnterCriticalSection, LeaveCriticalSection, DeleteCriticalSection, as
 * contracts/windows.critical_section.single_thread/0.1.0.json records Windows leaving the structure
 * on six runners (Server 2025 and 2022; AMD EPYC 7763, 9V45; Intel Xeon 8573C; store/probe-runs/380397*):
 *   - initialized: DebugInfo -1, LockCount -1, RecursionCount 0, OwningThread 0, LockSemaphore 0
 *   - entered: LockCount -2 and RecursionCount 1 by the owner; again by the owner: RecursionCount + 1
 *   - left to RecursionCount 0: LockCount -1, no owner; deleted: every field 0
 *   - SpinCount: the count with the flags in the high byte; a zero count without flags is 2000 with
 *     the dynamic-spin flag (0x20007D0), as InitializeCriticalSection gives
 * Another thread holding the section is Sprint 047's (threads and waits): it stops the run.
 */
#include "runtime.h"

#define NO_DEBUG_INFO 0x01000000u
#define DYNAMIC_SPIN 0x02000000u                    /* RTL_CRITICAL_SECTION_FLAG_DYNAMIC_SPIN */
#define STATIC_INIT 0x04000000u                     /* RTL_CRITICAL_SECTION_FLAG_STATIC_INIT */
#define DEFAULT_SPIN 0x020007D0ull                 /* observed: count 2000, flag 0x02000000 */

struct critical_section {
    u64 debug_info;                                 /* 0  RTL_CRITICAL_SECTION.DebugInfo */
    int lock_count;                                 /* 8 */
    int recursion_count;                            /* 12 */
    u64 owning_thread;                              /* 16 */
    u64 lock_semaphore;                             /* 24 */
    u64 spin_count;                                 /* 32 */
};
_Static_assert(sizeof(struct critical_section) == 40, "CRITICAL_SECTION is 40 bytes (row size)");

static u64 thread_id(void) { return (u32)w2m_teb()->client_thread; }

static void initialize(struct critical_section *cs, u64 spin) {
    cs->debug_info = ~0ull;
    cs->lock_count = -1;
    cs->recursion_count = 0;
    cs->owning_thread = 0;
    cs->lock_semaphore = 0;
    cs->spin_count = spin;
}

/* InitializeCriticalSectionEx(lpCriticalSection, dwSpinCount, Flags): nonzero. Flags, as
 * contracts/windows.critical_section.flags records (Sprint 049): with spin 0 the count is 2000 with the dynamic-spin
 * flag, as InitializeCriticalSection gives, and CRITICAL_SECTION_NO_DEBUG_INFO adds its bit (0x030007D0); a
 * nonzero spin keeps the flags it is given; RTL_CRITICAL_SECTION_FLAG_STATIC_INIT alone answers TRUE and leaves
 * the structure as it was. Other combinations are not in the rows. */
u64 k32_InitializeCriticalSectionEx(u64 cs, u64 spin, u64 flags) {
    spin = (u32)spin;
    flags = (u32)flags;
    if (flags == STATIC_INIT) return w2m_logged(W2M_API_InitializeCriticalSectionEx, cs, 1);
    if ((flags & ~(u64)(NO_DEBUG_INFO | DYNAMIC_SPIN)) || (spin != 0 && (flags & DYNAMIC_SPIN)))
        w2m_unimplemented(W2M_API_InitializeCriticalSectionEx, flags | spin << 32);
    initialize((struct critical_section *)cs, spin ? spin | flags : DEFAULT_SPIN | (flags & NO_DEBUG_INFO));
    return w2m_logged(W2M_API_InitializeCriticalSectionEx, cs, 1);
}

/* InitializeCriticalSectionAndSpinCount(lpCriticalSection, dwSpinCount): nonzero. */
u64 k32_InitializeCriticalSectionAndSpinCount(u64 cs, u64 spin) {
    if ((u32)spin == 0) w2m_unimplemented(W2M_API_InitializeCriticalSectionAndSpinCount, 0);
    initialize((struct critical_section *)cs, (u32)spin);
    return w2m_logged(W2M_API_InitializeCriticalSectionAndSpinCount, cs, 1);
}

u64 k32_InitializeCriticalSection(u64 cs) {
    initialize((struct critical_section *)cs, DEFAULT_SPIN);
    return w2m_logged(W2M_API_InitializeCriticalSection, cs, 0);
}

/* Enter or try: 1 when this thread now owns the section. */
static int acquire(struct critical_section *cs) {
    if (cs->recursion_count > 0 && cs->owning_thread == thread_id()) {
        cs->recursion_count++;
        return 1;
    }
    if (cs->lock_count == -1 && cs->recursion_count == 0) {
        cs->lock_count = -2;
        cs->recursion_count = 1;
        cs->owning_thread = thread_id();
        return 1;
    }
    return 0;
}

u64 k32_EnterCriticalSection(u64 cs) {
    if (!acquire((struct critical_section *)cs)) w2m_unimplemented(W2M_API_EnterCriticalSection, cs);
    return w2m_logged(W2M_API_EnterCriticalSection, cs, 0);
}

/* TryEnterCriticalSection: nonzero when entered; another owner (Sprint 047) stops the run. */
u64 k32_TryEnterCriticalSection(u64 cs) {
    if (!acquire((struct critical_section *)cs)) w2m_unimplemented(W2M_API_TryEnterCriticalSection, cs);
    return w2m_logged(W2M_API_TryEnterCriticalSection, cs, 1);
}

u64 k32_LeaveCriticalSection(u64 cs) {
    struct critical_section *c = (struct critical_section *)cs;
    if (c->recursion_count <= 0 || c->owning_thread != thread_id())
        w2m_unimplemented(W2M_API_LeaveCriticalSection, cs);          /* not owned: undefined */
    if (--c->recursion_count == 0) {
        c->owning_thread = 0;
        c->lock_count = -1;
    }
    return w2m_logged(W2M_API_LeaveCriticalSection, cs, 0);
}

u64 k32_DeleteCriticalSection(u64 cs) {
    u8 *b = (u8 *)cs;
    for (int i = 0; i < 40; i++) b[i] = 0;
    return w2m_logged(W2M_API_DeleteCriticalSection, cs, 0);
}
