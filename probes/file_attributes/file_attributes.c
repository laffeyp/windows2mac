// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* file_attributes.c — GetFileAttributesExW and GetFileAttributesW (Sprint 047, family "file attributes").
 *
 * No C runtime; names relative to the working directory, as probes/create_file does. UnityPlayer.dll calls
 * GetFileAttributesExW(path, GetFileExInfoStandard, &data) 36 times looking for its data folder (look-ahead
 * 2026-10-10). The probe makes one 5-byte file and asks about it under several spellings, the working
 * directory ("."), names that do not exist, and an invalid information level. Rows: return, last error
 * (after SetLastError(0xDEAD)), attributes, size, and the times as relations: each nonzero, creation no later
 * than the last write, the last write inside the window the system clock read before and after the write.
 */
#include <windows.h>

#define SENTINEL 0xDEADu

static char out[16384];
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
static unsigned long long ft(FILETIME f) { return (unsigned long long)f.dwHighDateTime << 32 | f.dwLowDateTime; }

static unsigned long long before, after;

static void ask(const char *name, const WCHAR *path, GET_FILEEX_INFO_LEVELS level) {
    WIN32_FILE_ATTRIBUTE_DATA d;
    unsigned char *b = (unsigned char *)&d;
    for (unsigned i = 0; i < sizeof d; i++) b[i] = 0xA5;
    SetLastError(SENTINEL);
    BOOL ok = GetFileAttributesExW(path, level, &d);
    DWORD err = GetLastError();
    row(name); field("ret", ok != 0); field("err", err);
    if (ok) {
        unsigned long long c = ft(d.ftCreationTime), a = ft(d.ftLastAccessTime), w = ft(d.ftLastWriteTime);
        field("attributes", d.dwFileAttributes); field("size_high", d.nFileSizeHigh); field("size_low", d.nFileSizeLow);
        flag("times_nonzero", c && a && w); flag("creation_le_write", c <= w); flag("access_ge_creation", a >= c);
        flag("write_in_window", w >= before && w <= after);
    } else {
        flag("data_untouched", b[0] == 0xA5 && b[sizeof d - 1] == 0xA5);
    }
    end();
}

static void plain(const char *name, const WCHAR *path) {
    SetLastError(SENTINEL);
    DWORD attributes = GetFileAttributesW(path);
    DWORD err = GetLastError();
    row(name); field("attributes", attributes); field("err", err); end();
}

void start(void) {
    DeleteFileW(L"attr_a.txt");
    FILETIME t;
    GetSystemTimeAsFileTime(&t);
    before = ft(t) - 20000000ull;                     /* two seconds of slack: file times are not the clock */
    HANDLE h = CreateFileW(L"attr_a.txt", GENERIC_READ | GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    DWORD written = 0;
    WriteFile(h, "hello", 5, &written, NULL);
    CloseHandle(h);
    GetSystemTimeAsFileTime(&t);
    after = ft(t) + 20000000ull;
    row("setup"); flag("created", h != INVALID_HANDLE_VALUE); field("written", written); end();

    static const struct { const char *name; const WCHAR *path; } NAMES[] = {
        {"file", L"attr_a.txt"}, {"file.upper", L"ATTR_A.TXT"}, {"file.trailing_dot", L"attr_a.txt."},
        {"file.trailing_space", L"attr_a.txt "}, {"file.trailing_backslash", L"attr_a.txt\\"},
        {"file.as_directory", L"attr_a.txt\\x"}, {"dot", L"."}, {"dot.backslash", L".\\"},
        {"dot.file", L".\\attr_a.txt"}, {"empty", L""}, {"missing", L"missing_w2m.txt"},
        {"missing_directory", L"missing_w2m\\x.txt"}, {"wildcard", L"attr_*.txt"},
    };
    for (unsigned i = 0; i < sizeof NAMES / sizeof NAMES[0]; i++) {
        static char name[48];
        unsigned k = 0;
        for (const char *s = "ex."; *s;) name[k++] = *s++;
        for (const char *s = NAMES[i].name; *s;) name[k++] = *s++;
        name[k] = 0;
        ask(name, NAMES[i].path, GetFileExInfoStandard);
        k = 0;
        for (const char *s = "plain."; *s;) name[k++] = *s++;
        for (const char *s = NAMES[i].name; *s;) name[k++] = *s++;
        name[k] = 0;
        plain(name, NAMES[i].path);
    }
    ask("ex.level_1", L"attr_a.txt", (GET_FILEEX_INFO_LEVELS)1);
    DeleteFileW(L"attr_a.txt");
    ask("ex.after_delete", L"attr_a.txt", GetFileExInfoStandard);
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
