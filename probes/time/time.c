// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* time.c — the work list's time family (Sprint 054, family "time"; specs/work_list.json).
 *
 * No C runtime. The captures hold the forms: KERNELBASE's GetSystemTimePreciseAsFileTime through GetProcAddress
 * (seq 1603), WINMM's timeGetTime (51,501 calls in the play run's imports table; first at seq 1707),
 * timeBeginPeriod(1) once, GetTickCount64 once (mono). Clock values differ from run to run, so the rows hold
 * relations: each clock against the one w2m already serves (GetSystemTimeAsFileTime, GetTickCount), never going
 * back over 1,000 reads, and advancing across Sleep(50).
 */
#include <windows.h>

typedef void (WINAPI *precise_fn)(FILETIME *);

static char out[8192];
static unsigned n;
static void put(const char *s) { while (*s && n < sizeof out - 64) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}
static ULONGLONG ft(FILETIME f) { return (ULONGLONG)f.dwHighDateTime << 32 | f.dwLowDateTime; }
static ULONGLONG distance(ULONGLONG a, ULONGLONG b) { return a > b ? a - b : b - a; }

void start(void) {
    precise_fn precise = (precise_fn)GetProcAddress(GetModuleHandleW(L"kernelbase.dll"), "GetSystemTimePreciseAsFileTime");
    row("precise.found"); flag("value", precise != NULL); end();
    FILETIME a, b;
    precise(&a);
    GetSystemTimeAsFileTime(&b);
    row("precise.against_system_time"); flag("within_one_second", distance(ft(a), ft(b)) < 10000000ull);
    flag("after_2020", ft(a) > 132223104000000000ull); end();
    int back = 0;
    ULONGLONG last = ft(a);
    for (int i = 0; i < 1000; i++) {
        precise(&a);
        back |= ft(a) < last;
        last = ft(a);
    }
    row("precise.never_back"); flag("value", !back); end();
    ULONGLONG before = last;
    Sleep(50);
    precise(&a);
    row("precise.across_sleep_50"); flag("at_least_40ms", ft(a) - before >= 400000ull);
    flag("under_5s", ft(a) - before < 50000000ull); end();

    DWORD t = timeGetTime(), tick = GetTickCount();
    row("time_get_time.against_tick_count"); flag("within_one_second", distance(t, tick) < 1000); end();
    back = 0;
    DWORD prev = t;
    for (int i = 0; i < 1000; i++) {
        DWORD x = timeGetTime();
        back |= x < prev;
        prev = x;
    }
    row("time_get_time.never_back"); flag("value", !back); end();
    Sleep(50);
    DWORD after = timeGetTime();
    row("time_get_time.across_sleep_50"); flag("at_least_40ms", after - prev >= 40); flag("under_5s", after - prev < 5000); end();

    row("period.begin_1"); field("ret", timeBeginPeriod(1)); end();
    row("period.begin_0"); field("ret", timeBeginPeriod(0)); end();
    row("period.end_1"); field("ret", timeEndPeriod(1)); end();
    row("period.end_0"); field("ret", timeEndPeriod(0)); end();

    ULONGLONG t64 = GetTickCount64();
    DWORD t32 = GetTickCount();
    row("tick_count_64.against_tick_count"); flag("low_within_one_second", distance((DWORD)t64, t32) < 1000);
    flag("high_zero", (t64 >> 32) == 0); end();
    ExitProcess(0);
}
