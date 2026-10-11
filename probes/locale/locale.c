// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* locale.c — SystemTimeToTzSpecificLocalTime, GetThreadLocale, GetLocaleInfoA and GetUserDefaultLocaleName (Sprint 049,
 * family "locale"; specs/work_list.json).
 *
 * No C runtime. The capture (store/game-runs/38113342549, 38113343972) holds the game's forms: mono's
 * SystemTimeToTzSpecificLocalTime(NULL, &utc, &local) after each FileTimeToSystemTime (18 calls), GetThreadLocale
 * (0x409), GetLocaleInfoA(0x409, LOCALE_SISO639LANGNAME or LOCALE_SISO3166CTRYNAME, buf, 9) (3: "en", "US"), and
 * UnityPlayer's GetUserDefaultLocaleName(buf, 85) (6: "en-US"). The runners' zone is UTC (the capture's
 * GetTimeZoneInformation answered TIME_ZONE_ID_UNKNOWN), as is w2m's. Rows: return, last error (after
 * SetLastError(0xDEAD)), and what each call wrote, every buffer filled with 0xA5 first.
 */
#include <windows.h>

#define SENTINEL 0xDEADu

static char out[16384];
static unsigned n;
static char abuf[64];
static WCHAR wbuf[128];

static void put(const char *s) { while (*s && n < sizeof out - 64) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void hex(const char *k, const unsigned char *p, unsigned len) {
    put(",\""); put(k); put("\":\"");
    for (unsigned i = 0; i < len; i++) {
        out[n++] = "0123456789abcdef"[p[i] >> 4];
        out[n++] = "0123456789abcdef"[p[i] & 15];
    }
    put("\"");
}
/* Each row is written when it ends, so a fault leaves the rows before it. */
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}

static void st_field(const char *k, const SYSTEMTIME *t) {
    put(",\""); put(k); put("\":[");
    const WORD *w = (const WORD *)t;
    for (int i = 0; i < 8; i++) { if (i) put(","); putu(w[i]); }
    put("]");
}

/* SystemTimeToTzSpecificLocalTime(NULL, in, out): the SYSTEMTIME words in, the words out (0xA5A5 where unwritten). */
static void to_local(const char *name, WORD y, WORD m, WORD dow, WORD d, WORD h, WORD mi, WORD s, WORD ms) {
    SYSTEMTIME in = {y, m, dow, d, h, mi, s, ms}, o;
    for (int i = 0; i < 8; i++) ((WORD *)&o)[i] = 0xA5A5;
    SetLastError(SENTINEL);
    BOOL r = SystemTimeToTzSpecificLocalTime(NULL, &in, &o);
    DWORD err = GetLastError();
    row(name); field("ret", r != 0); field("err", err); st_field("in", &in); st_field("out", &o); end();
}

static void info_a(const char *name, LCID lcid, LCTYPE type, int size, int null_buffer) {
    for (unsigned i = 0; i < sizeof abuf; i++) abuf[i] = (char)0xA5;
    SetLastError(SENTINEL);
    int r = GetLocaleInfoA(lcid, type, null_buffer ? NULL : abuf, size);
    DWORD err = GetLastError();
    row(name); field("ret", (unsigned)r); field("err", err);
    if (!null_buffer) hex("buf", (const unsigned char *)abuf, 10);
    end();
}

static void user_name(const char *name, int size, int null_buffer) {
    for (unsigned i = 0; i < 128; i++) wbuf[i] = 0xA5A5;
    SetLastError(SENTINEL);
    int r = GetUserDefaultLocaleName(null_buffer ? NULL : wbuf, size);
    DWORD err = GetLastError();
    row(name); field("ret", (unsigned)r); field("err", err);
    if (!null_buffer) hex("buf", (const unsigned char *)wbuf, 16);
    end();
}

void start(void) {
    to_local("local.game_form", 2026, 10, 0, 11, 6, 51, 41, 378);
    to_local("local.day_of_week_ignored", 2026, 10, 5, 11, 6, 51, 41, 378);   /* 2026-10-11 is a Sunday (0) */
    to_local("local.first_day", 1601, 1, 1, 1, 0, 0, 0, 0);
    to_local("local.last_day", 30827, 12, 0, 31, 23, 59, 59, 999);
    to_local("local.leap_day", 2024, 2, 4, 29, 12, 0, 0, 0);
    to_local("local.not_leap_day", 2025, 2, 0, 29, 12, 0, 0, 0);
    to_local("local.month_13", 2026, 13, 0, 1, 0, 0, 0, 0);
    to_local("local.month_0", 2026, 0, 0, 1, 0, 0, 0, 0);
    to_local("local.day_0", 2026, 10, 0, 0, 0, 0, 0, 0);
    to_local("local.hour_24", 2026, 10, 0, 11, 24, 0, 0, 0);
    to_local("local.ms_1000", 2026, 10, 0, 11, 0, 0, 0, 1000);
    to_local("local.year_1600", 1600, 12, 0, 31, 0, 0, 0, 0);
    to_local("local.year_30828", 30828, 1, 0, 1, 0, 0, 0, 0);

    SetLastError(SENTINEL);
    LCID t = GetThreadLocale();
    DWORD err = GetLastError();
    row("thread_locale"); field("ret", t); field("err", err); end();

    info_a("info.language", 0x409, LOCALE_SISO639LANGNAME, 9, 0);
    info_a("info.country", 0x409, LOCALE_SISO3166CTRYNAME, 9, 0);
    info_a("info.language_exact", 0x409, LOCALE_SISO639LANGNAME, 3, 0);
    info_a("info.language_short", 0x409, LOCALE_SISO639LANGNAME, 2, 0);
    info_a("info.language_size_zero", 0x409, LOCALE_SISO639LANGNAME, 0, 1);
    info_a("info.country_size_zero", 0x409, LOCALE_SISO3166CTRYNAME, 0, 1);
    info_a("info.user_default", LOCALE_USER_DEFAULT, LOCALE_SISO639LANGNAME, 9, 0);
    info_a("info.system_default", LOCALE_SYSTEM_DEFAULT, LOCALE_SISO3166CTRYNAME, 9, 0);
    info_a("info.thread_locale", t, LOCALE_SISO3166CTRYNAME, 9, 0);
    info_a("info.negative_size", 0x409, LOCALE_SISO639LANGNAME, -1, 0);

    user_name("user_name.game_form", 85, 0);
    user_name("user_name.exact", 6, 0);
    user_name("user_name.short", 5, 0);
    user_name("user_name.size_zero", 0, 1);
    ExitProcess(0);
}
