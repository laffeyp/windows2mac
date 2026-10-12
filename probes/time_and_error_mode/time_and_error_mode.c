// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* time_and_error_mode.c — local time, time conversions, the error mode (Sprint 046, family "time and error
 * mode").
 *
 * No C runtime. Under w2m UnityPlayer.dll calls GetLocalTime and SystemTimeToFileTime, and SetErrorMode
 * once UnityMain runs (frontier and scratch runs, 2026-10-10: 364,956 and 401,390 instructions). Rows: local
 * time against system time (the runners keep UTC; a relation, since the clock moves), the time zone the
 * process sees, SystemTimeToFileTime and FileTimeToSystemTime on fixed values and on invalid ones, and the
 * error mode: the one this process inherited from the runner's cmd, and the one a child started with
 * SetErrorMode(0) in its parent and no inheritance flag sees (the Steam-like start).
 */
#include <windows.h>

#define SENTINEL 0xDEADu

static char out[8192];
static unsigned n;

static void put(const char *s) { while (*s) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void end(void) { put("}\n"); }
/* A UTF-16 text field, ASCII as is and every other unit as \uXXXX (Sprint 061: the time zone's names). */
static void text(const char *k, const WCHAR *w, unsigned max) {
    static const char hex[] = "0123456789abcdef";
    put(",\""); put(k); put("\":\"");
    for (unsigned i = 0; i < max && w[i]; i++) {
        WCHAR c = w[i];
        if (c >= 0x20 && c < 0x7F && c != '"' && c != '\\') { out[n++] = (char)c; continue; }
        put("\\u"); for (int sh = 12; sh >= 0; sh -= 4) out[n++] = hex[(c >> sh) & 15];
    }
    put("\"");
}

static DWORD err;
static const WCHAR ROWS[] = L"w2m-error-mode-child.jsonl";

static unsigned long long ft(FILETIME f) { return (unsigned long long)f.dwHighDateTime << 32 | f.dwLowDateTime; }

static void systemtime_fields(SYSTEMTIME *t) {
    field("year", t->wYear); field("month", t->wMonth); field("day_of_week", t->wDayOfWeek); field("day", t->wDay);
    field("hour", t->wHour); field("minute", t->wMinute); field("second", t->wSecond); field("ms", t->wMilliseconds);
}

static int is_child(void) {
    const WCHAR *c = GetCommandLineW();
    DWORD len = 0;
    while (c[len]) len++;
    return len >= 6 && c[len - 5] == 'c' && c[len - 4] == 'h' && c[len - 3] == 'i' && c[len - 2] == 'l' && c[len - 1] == 'd';
}

static void child(void) {
    SetLastError(SENTINEL);
    UINT inherited = GetErrorMode();
    err = GetLastError();
    row("error_mode.child_inherited"); field("value", inherited); field("err", err); end();
    HANDLE f = CreateFileW(ROWS, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    DWORD w;
    WriteFile(f, out, n, &w, NULL);
    CloseHandle(f);
    ExitProcess(0);
}

void start(void) {
    if (is_child()) child();

    SYSTEMTIME local, system;
    FILETIME before, after;
    GetSystemTimeAsFileTime(&before);
    SetLastError(SENTINEL);
    GetLocalTime(&local);
    err = GetLastError();
    GetSystemTime(&system);
    GetSystemTimeAsFileTime(&after);
    FILETIME lf, sf;
    SystemTimeToFileTime(&local, &lf);
    SystemTimeToFileTime(&system, &sf);
    unsigned long long l = ft(lf), s = ft(sf);
    row("local_time"); field("err", err); flag("equals_system_within_a_second", (l > s ? l - s : s - l) < 10000000ull);
    flag("within_the_reads", l + 10000 >= ft(before) && l <= ft(after) + 10000); end();

    TIME_ZONE_INFORMATION tz;
    unsigned char *b = (unsigned char *)&tz;
    for (unsigned i = 0; i < sizeof tz; i++) b[i] = 0xA5;
    SetLastError(SENTINEL);
    DWORD id = GetTimeZoneInformation(&tz);
    err = GetLastError();
    row("time_zone"); field("ret", id); field("err", err); field("bias", (unsigned long long)(long long)tz.Bias);
    field("standard_bias", (unsigned long long)(long long)tz.StandardBias); field("daylight_bias", (unsigned long long)(long long)tz.DaylightBias);
    field("standard_month", tz.StandardDate.wMonth); field("daylight_month", tz.DaylightDate.wMonth); end();
    /* the names, which the game converts with WideCharToMultiByte (game run 38203364352 seq 921-924) */
    row("time_zone.names"); text("standard_name", tz.StandardName, 32); text("daylight_name", tz.DaylightName, 32); end();

    SYSTEMTIME fixed = {2026, 10, 6, 10, 13, 45, 30, 125};    /* a Saturday; wDayOfWeek ignored on input */
    FILETIME out_ft;
    SetLastError(SENTINEL);
    BOOL ok = SystemTimeToFileTime(&fixed, &out_ft);
    err = GetLastError();
    row("system_to_file.fixed"); field("ret", ok != 0); field("err", err); field("filetime", ft(out_ft)); end();
    SYSTEMTIME back;
    SetLastError(SENTINEL);
    ok = FileTimeToSystemTime(&out_ft, &back);
    err = GetLastError();
    row("file_to_system.fixed"); field("ret", ok != 0); field("err", err); systemtime_fields(&back); end();
    SYSTEMTIME bad = fixed;
    bad.wMonth = 13;
    SetLastError(SENTINEL);
    ok = SystemTimeToFileTime(&bad, &out_ft);
    err = GetLastError();
    row("system_to_file.month_13"); field("ret", ok != 0); field("err", err); end();
    bad = fixed;
    bad.wDay = 31;
    bad.wMonth = 2;
    SetLastError(SENTINEL);
    ok = SystemTimeToFileTime(&bad, &out_ft);
    err = GetLastError();
    row("system_to_file.february_31"); field("ret", ok != 0); field("err", err); end();
    FILETIME huge = {0xFFFFFFFFu, 0xFFFFFFFFu};
    SetLastError(SENTINEL);
    ok = FileTimeToSystemTime(&huge, &back);
    err = GetLastError();
    row("file_to_system.huge"); field("ret", ok != 0); field("err", err); end();
    FILETIME epoch = {0, 0};
    SetLastError(SENTINEL);
    ok = FileTimeToSystemTime(&epoch, &back);
    err = GetLastError();
    row("file_to_system.zero"); field("ret", ok != 0); field("err", err); systemtime_fields(&back); end();

    SetLastError(SENTINEL);
    UINT inherited = GetErrorMode();
    err = GetLastError();
    row("error_mode.inherited_from_cmd"); field("value", inherited); field("err", err); end();
    SetLastError(SENTINEL);
    UINT previous = SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
    err = GetLastError();
    row("error_mode.set"); flag("returns_previous", previous == inherited); field("err", err); field("now", GetErrorMode()); end();
    SetErrorMode(SEM_NOOPENFILEERRORBOX);
    row("error_mode.set_open_file"); field("now", GetErrorMode()); end();
    SetErrorMode(0);

    /* the parent's rows go out before the child starts: w2m serves no CreateProcessW, and its run of the parent
       ends there with these rows written (the child's row is a family of its own) */
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;

    static WCHAR self[600], cmd[700];
    DWORD len = GetModuleFileNameW(NULL, self, 300), k = 0;
    cmd[k++] = '"';
    for (DWORD i = 0; i < len; i++) cmd[k++] = self[i];
    const WCHAR *tail = L"\" child";
    for (DWORD i = 0; tail[i]; i++) cmd[k++] = tail[i];
    cmd[k] = 0;
    STARTUPINFOW si;
    unsigned char *z = (unsigned char *)&si;
    for (unsigned i = 0; i < sizeof si; i++) z[i] = 0;
    si.cb = sizeof si;
    PROCESS_INFORMATION pi;
    DeleteFileW(ROWS);
    if (CreateProcessW(self, cmd, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi)) {
        WaitForSingleObject(pi.hProcess, 60000);
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);
        HANDLE f = CreateFileW(ROWS, GENERIC_READ, 0, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
        DWORD got = 0;
        if (f != INVALID_HANDLE_VALUE) { ReadFile(f, out + n, 512, &got, NULL); CloseHandle(f); }
        n += got;
        DeleteFileW(ROWS);
    }

    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
