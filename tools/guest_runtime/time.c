// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* time.c — the work list's time family (Sprint 054, family "time"; specs/work_list.json).
 *
 * contracts/windows.time.time records Windows (Server 2025 and 2022, equal):
 *   - GetSystemTimePreciseAsFileTime (KERNELBASE, through GetProcAddress) is within a second of
 *     GetSystemTimeAsFileTime, never goes back, and advances across Sleep(50) by 40 ms to 5 s
 *   - timeGetTime is within a second of GetTickCount, never goes back, and advances the same way
 *   - timeBeginPeriod(1) and timeEndPeriod(1) answer TIMERR_NOERROR, a period of 0 TIMERR_NOCANDO
 *   - GetTickCount64's low half is within a second of GetTickCount
 * The clocks are process.c's: the system time and the 10 MHz counter, read in milliseconds. A period ended that was
 * not begun is not in the rows and stops the run.
 */
#include "runtime.h"

#define TIMERR_NOERROR 0u
#define TIMERR_NOCANDO 97u
#define PERIODS 16

static u32 begun[PERIODS + 1];                 /* timeBeginPeriod calls not yet ended, by period in ms */

/* "Retrieves the current system date and time with the highest possible level of precision (<1us)" */
W2M_VOID u64 k32_GetSystemTimePreciseAsFileTime(u64 out) {
    u64 now = w2m_filetime_now();
    *(u32 *)out = (u32)now;
    *(u32 *)(out + 4) = (u32)(now >> 32);
    return w2m_logged(W2M_API_GetSystemTimePreciseAsFileTime, out, 0);
}

u64 k32_GetTickCount64(void) {
    return w2m_logged(W2M_API_GetTickCount64, 0, w2m_counter_now() / 10000);
}

/* "The system time is the time elapsed, in milliseconds, since Windows was started" */
u64 winmm_timeGetTime(void) {
    return w2m_logged(W2M_API_timeGetTime, 0, (u32)(w2m_counter_now() / 10000));
}

u64 winmm_timeBeginPeriod(u64 period) {
    if ((u32)period == 0) return w2m_logged(W2M_API_timeBeginPeriod, period, TIMERR_NOCANDO);
    if ((u32)period > PERIODS) w2m_unimplemented(W2M_API_timeBeginPeriod, period);      /* not in the rows */
    begun[(u32)period]++;
    return w2m_logged(W2M_API_timeBeginPeriod, period, TIMERR_NOERROR);
}

u64 winmm_timeEndPeriod(u64 period) {
    if ((u32)period == 0) return w2m_logged(W2M_API_timeEndPeriod, period, TIMERR_NOCANDO);
    if ((u32)period > PERIODS || !begun[(u32)period]) w2m_unimplemented(W2M_API_timeEndPeriod, period);
    begun[(u32)period]--;
    return w2m_logged(W2M_API_timeEndPeriod, period, TIMERR_NOERROR);
}
