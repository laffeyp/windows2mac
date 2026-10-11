// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* registry.c — opening registry keys as the game first does (Sprint 054, family "registry"; specs/work_list.json).
 *
 * No C runtime. The batch trace's first registry call (seq 1653) is RegOpenKeyExW(HKEY_CURRENT_USER, <a key under
 * Software>, 0, KEY_READ) answering ERROR_FILE_NOT_FOUND: the game's keys do not exist on a fresh machine. The rows
 * open a missing key (one and two levels below Software), Software itself, the root by a NULL and an empty subkey,
 * a handle that is no key, and close keys, a closed key and no key. Each row keeps the code, what was written to
 * the result, and the last error after SetLastError(0xDEAD) (registry functions return their code).
 */
#include <windows.h>

#define SENTINEL 0xDEADu
#define UNTOUCHED ((HKEY)(ULONG_PTR)0x1234)

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

static HKEY open(const char *name, HKEY root, const WCHAR *sub) {
    HKEY h = UNTOUCHED;
    SetLastError(SENTINEL);
    LONG r = RegOpenKeyExW(root, sub, 0, KEY_READ, &h);
    row(name); field("ret", (DWORD)r); field("err", GetLastError());
    flag("result_null", h == NULL); flag("result_untouched", h == UNTOUCHED);
    flag("result_is_root", h == root);
    end();
    return r == ERROR_SUCCESS ? h : NULL;
}

static void close(const char *name, HKEY h) {
    SetLastError(SENTINEL);
    LONG r = RegCloseKey(h);
    row(name); field("ret", (DWORD)r); field("err", GetLastError()); end();
}

void start(void) {
    open("open.missing", HKEY_CURRENT_USER, L"Software\\w2m-probe-missing-5d1c");
    open("open.missing_two_levels", HKEY_CURRENT_USER, L"Software\\w2m-probe-missing-5d1c\\Deeper");
    open("open.missing_case", HKEY_CURRENT_USER, L"SOFTWARE\\W2M-PROBE-MISSING-5D1C");
    HKEY software = open("open.software", HKEY_CURRENT_USER, L"Software");
    HKEY software_case = open("open.software_case", HKEY_CURRENT_USER, L"sOFTWARE");
    if (software) open("open.missing_below_software_handle", software, L"w2m-probe-missing-5d1c");
    HKEY by_null = open("open.root_null_subkey", HKEY_CURRENT_USER, NULL);
    HKEY by_empty = open("open.root_empty_subkey", HKEY_CURRENT_USER, L"");
    open("open.not_a_key", UNTOUCHED, L"Software");
    /* Sprint 056 (review of 054): names with letters above ASCII, which w2m had narrowed to char */
    open("open.u0173_oftware", HKEY_CURRENT_USER, L"\x0173oftware");
    open("open.software_u0100_x", HKEY_CURRENT_USER, L"Software\x0100x");
    open("open.long_s_oftware", HKEY_CURRENT_USER, L"\x017Foftware");
    open("open.software_dotless_i", HKEY_CURRENT_USER, L"Softwar\x0131e");
    if (software) close("close.software", software);
    if (software_case) close("close.software_case", software_case);
    if (software) close("close.software_again", software);
    if (by_null) close("close.root_null_subkey", by_null);
    if (by_empty) close("close.root_empty_subkey", by_empty);
    close("close.not_a_key", UNTOUCHED);
    close("close.null", NULL);
    ExitProcess(0);
}
