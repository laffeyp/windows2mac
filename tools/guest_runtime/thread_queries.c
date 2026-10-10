// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* thread_queries.c — GetCurrentThread, GetThreadTimes, GetThreadPriority (Sprint 046, family "thread
 * queries").
 *
 * contracts/windows.thread.queries records Windows on four runners (8 identical outputs), the times as
 * relations:
 *   - GetCurrentThread answers the pseudo-handle -2
 *   - GetThreadTimes on it succeeds: creation within a second after now or ten minutes before it (Server
 *     2022 can read it later than the system time), exit time zero, kernel and user time under ten
 *     seconds; on NULL it fails with ERROR_INVALID_HANDLE
 *   - GetThreadPriority on it: 0 (THREAD_PRIORITY_NORMAL)
 * w2m has one thread: its creation time is the process's first clock reading, its kernel time zero and
 * its user time the time since creation (an upper bound; w2m does not count the thread's own time).
 * Another thread's handle stops the run (Sprint 047).
 */
#include "runtime.h"

#define CURRENT_THREAD 0xFFFFFFFFFFFFFFFEull
#define ERROR_INVALID_HANDLE 6u

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }
static void filetime(u64 out, u64 v) { *(u32 *)out = (u32)v; *(u32 *)(out + 4) = (u32)(v >> 32); }

u64 k32_GetCurrentThread(void) { return w2m_logged(W2M_API_GetCurrentThread, 0, CURRENT_THREAD); }

u64 k32_GetThreadTimes(u64 thread, u64 creation, u64 exit, u64 kernel, u64 rsp) {
    u64 user = *(u64 *)(rsp + 0x28);
    if (thread == 0) {
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_GetThreadTimes, thread, 0);
    }
    if (thread != CURRENT_THREAD) w2m_unimplemented(W2M_API_GetThreadTimes, thread);
    u64 start = w2m_filetime_start(), now = w2m_filetime_now();
    filetime(creation, start);
    filetime(exit, 0);
    filetime(kernel, 0);
    filetime(user, now - start);
    return w2m_logged(W2M_API_GetThreadTimes, thread, 1);
}

u64 k32_GetThreadPriority(u64 thread) {
    if (thread != CURRENT_THREAD) w2m_unimplemented(W2M_API_GetThreadPriority, thread);
    return w2m_logged(W2M_API_GetThreadPriority, thread, 0);
}
