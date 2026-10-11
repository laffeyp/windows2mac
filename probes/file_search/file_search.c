// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* file_search.c — the work list's file_search family (Sprint 054, family "file_search"; specs/work_list.json).
 *
 * No C runtime. UnityPlayer calls FindFirstFileExW(path, FindExInfoBasic, &data, FindExSearchNameMatch, NULL, 0)
 * (batch trace seq 1673: a missing directory, ERROR_PATH_NOT_FOUND; the play run lists a folder with FindNextFileW,
 * 348 calls) and FindClose. The probe makes a folder of two files and a subfolder in its working directory and
 * lists it: each entry in order with its attributes, size, short name and times (against GetFileAttributesExW),
 * then patterns, an exact name, the folder itself, missing paths, and FindClose's answers.
 */
#include <windows.h>

#define SENTINEL 0xDEADu

static char out[16384];
static unsigned n;
static void put(const char *s) { while (*s && n < sizeof out - 64) out[n++] = *s++; }
static void putw(const WCHAR *s) { while (*s && n < sizeof out - 64) out[n++] = *s < 128 ? (char)*s : '?', s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void row2(const char *s, const char *t) { put("{\"s\":\""); put(s); put(t); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void text(const char *k, const WCHAR *v) { put(",\""); put(k); put("\":\""); putw(v); put("\""); }
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}
static int same_time(FILETIME a, FILETIME b) { return a.dwLowDateTime == b.dwLowDateTime && a.dwHighDateTime == b.dwHighDateTime; }

static void entry(const WIN32_FIND_DATAW *d, const WCHAR *folder) {
    text("name", d->cFileName); text("short", d->cAlternateFileName);
    field("attributes", d->dwFileAttributes); field("size_high", d->nFileSizeHigh); field("size", d->nFileSizeLow);
    WCHAR path[MAX_PATH];
    int k = 0;
    for (const WCHAR *p = folder; *p; p++) path[k++] = *p;
    path[k++] = L'\\';
    for (const WCHAR *p = d->cFileName; *p; p++) path[k++] = *p;
    path[k] = 0;
    WIN32_FILE_ATTRIBUTE_DATA a;
    if (d->cFileName[0] != L'.' && GetFileAttributesExW(path, GetFileExInfoStandard, &a)) {
        flag("write_time_matches", same_time(a.ftLastWriteTime, d->ftLastWriteTime));
        flag("creation_time_matches", same_time(a.ftCreationTime, d->ftCreationTime));
    }
    flag("write_time_set", d->ftLastWriteTime.dwHighDateTime != 0);
}

/* FindFirstFileExW in the game's form, then FindNextFileW to the end and FindClose */
static void search(const char *name, const WCHAR *pattern, const WCHAR *folder) {
    WIN32_FIND_DATAW d;
    for (unsigned i = 0; i < sizeof d; i++) ((char *)&d)[i] = (char)0xA5;
    SetLastError(SENTINEL);
    HANDLE h = FindFirstFileExW(pattern, FindExInfoBasic, &d, FindExSearchNameMatch, NULL, 0);
    row(name); flag("invalid", h == INVALID_HANDLE_VALUE); field("err", GetLastError());
    if (h != INVALID_HANDLE_VALUE) entry(&d, folder);
    end();
    if (h == INVALID_HANDLE_VALUE) return;
    for (int i = 0; i < 10; i++) {
        char suffix[] = ".next0";
        suffix[5] = (char)('0' + i);
        SetLastError(SENTINEL);
        BOOL ok = FindNextFileW(h, &d);
        row2(name, suffix);
        field("ret", ok != 0); field("err", GetLastError());
        if (ok) entry(&d, folder);
        end();
        if (!ok) break;
    }
    SetLastError(SENTINEL);
    BOOL c = FindClose(h);
    row2(name, ".close"); field("ret", c != 0); field("err", GetLastError()); end();
}

static void make_file(const WCHAR *path, DWORD size) {
    HANDLE h = CreateFileW(path, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    static const char bytes[16] = "w2m file search";
    DWORD written = 0;
    if (size) WriteFile(h, bytes, size, &written, NULL);
    CloseHandle(h);
}

void start(void) {
    CreateDirectoryW(L"w2m_find", NULL);
    make_file(L"w2m_find\\b.txt", 5);
    make_file(L"w2m_find\\a.dat", 0);
    CreateDirectoryW(L"w2m_find\\Sub", NULL);
    search("all", L"w2m_find\\*", L"w2m_find");
    search("pattern_txt", L"w2m_find\\*.txt", L"w2m_find");
    search("pattern_question", L"w2m_find\\?.dat", L"w2m_find");
    search("exact", L"w2m_find\\b.txt", L"w2m_find");
    search("exact_other_case", L"w2m_find\\B.TXT", L"w2m_find");
    search("folder_itself", L"w2m_find", L".");
    search("missing_folder", L"w2m_missing\\*", L"w2m_missing");
    /* Sprint 056 (review of 054): a long name under a deeper folder, found by a short pattern */
    static WCHAR long_name[260];
    int k = 0;
    for (const WCHAR *p = L"w2m_find\\Deep\\"; *p; p++) long_name[k++] = *p;
    int base = k;
    for (int i = 0; i < 200; i++) long_name[k++] = (WCHAR)(L'a' + i % 26);
    long_name[k] = 0;
    CreateDirectoryW(L"w2m_find\\Deep", NULL);
    make_file(long_name, 3);
    search("long_name", L"w2m_find\\Deep\\a*", L"w2m_find\\Deep");
    search("long_name_exact", long_name, L"w2m_find\\Deep");
    (void)base;
    DeleteFileW(long_name);
    RemoveDirectoryW(L"w2m_find\\Deep");
    search("missing_pattern", L"w2m_find\\nothing*", L"w2m_find");
    search("missing_file", L"w2m_find\\nothing.txt", L"w2m_find");
    SetLastError(SENTINEL);
    BOOL c = FindClose(INVALID_HANDLE_VALUE);
    row("close.invalid"); field("ret", c != 0); field("err", GetLastError()); end();
    DeleteFileW(L"w2m_find\\b.txt");
    DeleteFileW(L"w2m_find\\a.dat");
    RemoveDirectoryW(L"w2m_find\\Sub");
    RemoveDirectoryW(L"w2m_find");
    ExitProcess(0);
}
