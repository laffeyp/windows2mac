/* time_and_error_mode.c — local time, time conversions, the error mode (Sprint 046, family "time and error
 * mode").
 *
 * contracts/windows.time.local_and_error_mode records Windows on four runners (8 identical outputs):
 *   - the runners keep UTC: GetLocalTime equals GetSystemTime; GetTimeZoneInformation answers
 *     TIME_ZONE_ID_UNKNOWN (0) with every bias and transition zero, and sets the last error to 0
 *   - SystemTimeToFileTime and FileTimeToSystemTime convert exactly (2026-10-10 13:45:30.125 is
 *     134361135301250000; FILETIME 0 is 1601-01-01, a Monday); a month of 13, February 31, or a FILETIME of
 *     2^63 or more: FALSE, ERROR_INVALID_PARAMETER; success leaves the last error
 *   - the error mode a process starts with is its parent's: 0x8003 under the runner's cmd, 0 for a child
 *     whose parent set 0 (the Steam-like start); SetErrorMode returns the old mode and sets the new
 * w2m keeps UTC; the time zone's names are not in the rows (w2m writes them empty). The start mode decides
 * the first error mode: 0x8003 for the console start, 0 for the GUI start.
 */
#include "runtime.h"

typedef unsigned short u16;

#define ERROR_INVALID_PARAMETER 87u
#define TICKS_PER_DAY 864000000000ull           /* 100 ns units */

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

static int leap(u32 y) { return (y % 4 == 0 && y % 100 != 0) || y % 400 == 0; }

static u32 month_days(u32 y, u32 m) {
    static const u8 days[12] = {31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31};
    return days[m - 1] + (m == 2 && leap(y));
}

/* Days from 1601-01-01 to y-m-d. */
static u64 days_since_1601(u32 y, u32 m, u32 d) {
    u64 days = 0;
    u32 years = y - 1601;
    days += (u64)years * 365 + years / 4 - years / 100 + years / 400;
    for (u32 i = 1; i < m; i++) days += month_days(y, i);
    return days + d - 1;
}

static void to_systemtime(u64 t, u16 *st) {
    u64 days = t / TICKS_PER_DAY, rest = t % TICKS_PER_DAY;
    u32 y = 1601;
    for (;;) {
        u32 n = 365 + leap(y);
        if (days < n) break;
        days -= n;
        y++;
    }
    u32 m = 1;
    while (days >= month_days(y, m)) days -= month_days(y, m++);
    st[0] = (u16)y;
    st[1] = (u16)m;
    st[2] = (u16)((days_since_1601(y, m, (u32)days + 1) + 1) % 7);   /* 1601-01-01 was a Monday */
    st[3] = (u16)(days + 1);
    st[4] = (u16)(rest / 36000000000ull);
    st[5] = (u16)(rest / 600000000ull % 60);
    st[6] = (u16)(rest / 10000000ull % 60);
    st[7] = (u16)(rest / 10000ull % 1000);
}

static void now_systemtime(u64 out) { to_systemtime(w2m_filetime_now(), (u16 *)out); }

u64 k32_GetSystemTime(u64 out) { now_systemtime(out); return w2m_logged(W2M_API_GetSystemTime, out, 0); }
u64 k32_GetLocalTime(u64 out) { now_systemtime(out); return w2m_logged(W2M_API_GetLocalTime, out, 0); }   /* UTC */

u64 k32_GetTimeZoneInformation(u64 tz) {
    u8 *p = (u8 *)tz;
    for (int i = 0; i < 172; i++) p[i] = 0;                    /* TIME_ZONE_INFORMATION: 172 bytes */
    set_error(0);
    return w2m_logged(W2M_API_GetTimeZoneInformation, tz, 0);  /* TIME_ZONE_ID_UNKNOWN */
}

u64 k32_SystemTimeToFileTime(u64 st, u64 ft) {
    const u16 *t = (const u16 *)st;
    u32 y = t[0], m = t[1], d = t[3];
    if (y < 1601 || y > 30827 || m < 1 || m > 12 || d < 1 || d > month_days(y, m) || t[4] > 23 || t[5] > 59 ||
        t[6] > 59 || t[7] > 999) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_SystemTimeToFileTime, st, 0);
    }
    u64 v = days_since_1601(y, m, d) * TICKS_PER_DAY + (u64)t[4] * 36000000000ull + (u64)t[5] * 600000000ull +
            (u64)t[6] * 10000000ull + (u64)t[7] * 10000ull;
    *(u32 *)ft = (u32)v;
    *(u32 *)(ft + 4) = (u32)(v >> 32);
    return w2m_logged(W2M_API_SystemTimeToFileTime, st, 1);
}

u64 k32_FileTimeToSystemTime(u64 ft, u64 st) {
    u64 v = (u64)*(u32 *)ft | (u64)*(u32 *)(ft + 4) << 32;
    if (v >> 63) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_FileTimeToSystemTime, ft, 0);
    }
    to_systemtime(v, (u16 *)st);
    return w2m_logged(W2M_API_FileTimeToSystemTime, ft, 1);
}

static u32 error_mode;
static int error_mode_ready;

static u32 current_error_mode(void) {
    if (!error_mode_ready) {
        error_mode_ready = 1;
        error_mode = *(const volatile u64 *)(W2M_MONITOR_DATA_VA + W2M_START_MODE_OFF) == 1 ? 0 : 0x8003;
    }
    return error_mode;
}

u64 k32_GetErrorMode(void) { return w2m_logged(W2M_API_GetErrorMode, 0, current_error_mode()); }

u64 k32_SetErrorMode(u64 mode) {
    u32 old = current_error_mode();
    error_mode = (u32)mode;
    return w2m_logged(W2M_API_SetErrorMode, (u32)mode, old);
}
