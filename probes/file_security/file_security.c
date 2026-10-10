// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* file_security.c — CreateFileW with a SECURITY_ATTRIBUTES and no descriptor (Sprint 048, family "file security").
 *
 * No C runtime. UnityPlayer.dll opens "C:/w2m/app/<game>_Data/boot.config" with SECURITY_ATTRIBUTES {nLength 24,
 * lpSecurityDescriptor NULL, bInheritHandle TRUE} (look-ahead 2026-10-10). Rows: for each attribute block (none;
 * inherit TRUE; inherit FALSE; nLength 0, 12 and 1000; inherit 2), CreateFileW on an existing file and on a
 * missing one: whether the handle is valid, the last error, and GetHandleInformation's inherit flag.
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
static void end(void) { put("}\n"); }

static void open_with(const char *name, SECURITY_ATTRIBUTES *sa, const WCHAR *path, DWORD disposition) {
    SetLastError(SENTINEL);
    HANDLE h = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, sa, disposition, FILE_ATTRIBUTE_NORMAL, NULL);
    DWORD err = GetLastError();
    DWORD info = 0xA5;
    BOOL got = h != INVALID_HANDLE_VALUE && GetHandleInformation(h, &info);
    row(name); field("valid", h != INVALID_HANDLE_VALUE); field("err", err);
    if (got) field("inherit", info & HANDLE_FLAG_INHERIT);
    end();
    if (h != INVALID_HANDLE_VALUE) CloseHandle(h);
}

void start(void) {
    HANDLE h = CreateFileW(L"w2m_sec.txt", GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    CloseHandle(h);
    static SECURITY_ATTRIBUTES blocks[7];
    static const char *names[7] = {"none", "inherit", "no_inherit", "length_0", "length_12", "length_1000", "inherit_2"};
    DWORD lengths[7] = {0, 24, 24, 0, 12, 1000, 24};
    BOOL inherit[7] = {0, TRUE, FALSE, TRUE, TRUE, TRUE, 2};
    for (int i = 0; i < 7; i++) {
        blocks[i].nLength = lengths[i];
        blocks[i].lpSecurityDescriptor = NULL;
        blocks[i].bInheritHandle = inherit[i];
        static char name[48];
        unsigned k = 0;
        for (const char *s = names[i]; *s;) name[k++] = *s++;
        for (const char *s = ".existing"; *s;) name[k++] = *s++;
        name[k] = 0;
        open_with(name, i ? &blocks[i] : NULL, L"w2m_sec.txt", OPEN_EXISTING);
        k = 0;
        for (const char *s = names[i]; *s;) name[k++] = *s++;
        for (const char *s = ".missing"; *s;) name[k++] = *s++;
        name[k] = 0;
        open_with(name, i ? &blocks[i] : NULL, L"w2m_sec_missing.txt", OPEN_EXISTING);
    }
    DeleteFileW(L"w2m_sec.txt");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
