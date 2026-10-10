// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* file_access.c — CreateFileW's specific access rights (Sprint 048, family "file access").
 *
 * No C runtime. UnityPlayer.dll opens "<game>_Data\app.info" with FILE_GENERIC_READ (0x120089), FILE_SHARE_READ |
 * WRITE | DELETE, OPEN_EXISTING (frontier 2026-10-10, 2,562,879 instructions). Rows, for 13 access masks on an
 * existing 5-byte file: whether the open succeeds, its last error, and whether ReadFile and WriteFile then succeed
 * (with their errors); then sharing: a second open of each mask against a first handle with share none, share read,
 * and share read/write, given GENERIC_READ first.
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
static void end(void) { put("}\n"); }

static const struct { const char *name; DWORD mask; } MASKS[] = {
    {"generic_read", GENERIC_READ}, {"file_generic_read", FILE_GENERIC_READ}, {"read_data", FILE_READ_DATA},
    {"read_attributes", FILE_READ_ATTRIBUTES}, {"generic_write", GENERIC_WRITE}, {"file_generic_write", FILE_GENERIC_WRITE},
    {"write_data", FILE_WRITE_DATA}, {"append_data", FILE_APPEND_DATA}, {"file_all_access", FILE_ALL_ACCESS},
    {"generic_all", GENERIC_ALL}, {"maximum_allowed", MAXIMUM_ALLOWED}, {"synchronize", SYNCHRONIZE}, {"zero", 0},
};
#define MASK_COUNT (sizeof MASKS / sizeof MASKS[0])

static void name_of(char *dst, const char *a, const char *b) {
    unsigned k = 0;
    for (; *a;) dst[k++] = *a++;
    for (; *b;) dst[k++] = *b++;
    dst[k] = 0;
}

void start(void) {
    HANDLE f = CreateFileW(L"w2m_access.txt", GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    DWORD done;
    WriteFile(f, "hello", 5, &done, NULL);
    CloseHandle(f);
    static char name[64];
    for (unsigned i = 0; i < MASK_COUNT; i++) {
        SetLastError(SENTINEL);
        HANDLE h = CreateFileW(L"w2m_access.txt", MASKS[i].mask, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                               NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
        DWORD err = GetLastError();
        name_of(name, "open.", MASKS[i].name);
        row(name); field("valid", h != INVALID_HANDLE_VALUE); field("err", err);
        if (h != INVALID_HANDLE_VALUE) {
            char buf[8];
            DWORD got = 0xA5;
            SetLastError(SENTINEL);
            BOOL r = ReadFile(h, buf, 2, &got, NULL);
            DWORD rerr = GetLastError();
            field("read_ok", r != 0); field("read_err", rerr); field("read_bytes", r ? got : 99);
            SetLastError(SENTINEL);
            BOOL w = WriteFile(h, "x", 1, &got, NULL);
            DWORD werr = GetLastError();
            field("write_ok", w != 0); field("write_err", werr);
            CloseHandle(h);
        }
        end();
        if (MASKS[i].mask & (GENERIC_WRITE | FILE_WRITE_DATA | FILE_APPEND_DATA | GENERIC_ALL)) {  /* restore 5 bytes */
            f = CreateFileW(L"w2m_access.txt", GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
            WriteFile(f, "hello", 5, &done, NULL);
            CloseHandle(f);
        }
    }
    static const struct { const char *name; DWORD share; } FIRST[] = {
        {"none", 0}, {"read", FILE_SHARE_READ}, {"read_write", FILE_SHARE_READ | FILE_SHARE_WRITE}};
    for (unsigned s = 0; s < 3; s++)
        for (unsigned i = 0; i < MASK_COUNT; i++) {
            HANDLE first = CreateFileW(L"w2m_access.txt", GENERIC_READ, FIRST[s].share, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
            SetLastError(SENTINEL);
            HANDLE h = CreateFileW(L"w2m_access.txt", MASKS[i].mask, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                                   NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
            DWORD err = GetLastError();
            static char prefix[48];
            name_of(prefix, "share.", FIRST[s].name);
            name_of(name, prefix, ".");
            name_of(name + 0, name, MASKS[i].name);
            row(name); field("first_valid", first != INVALID_HANDLE_VALUE); field("valid", h != INVALID_HANDLE_VALUE); field("err", err); end();
            if (h != INVALID_HANDLE_VALUE) CloseHandle(h);
            if (first != INVALID_HANDLE_VALUE) CloseHandle(first);
        }
    DeleteFileW(L"w2m_access.txt");
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &done, NULL);
    ExitProcess(0);
}
