// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* current_directory_ansi.c — GetCurrentDirectoryA beside GetCurrentDirectoryW (Sprint 065, family "current directory
 * ANSI").
 *
 * No C runtime. steam_api64.dll imports GetCurrentDirectoryA (frontier 2026-10-12, 50,845,601,176 instructions). The
 * working directory is the runner's (an ASCII path); rows hold no path. n is GetCurrentDirectoryW's length. For each
 * buffer size (0 with NULL, 1, n, n + 1, MAX_PATH): the return less n, the last error after SetLastError(0xDEAD),
 * whether the bytes written equal the W path narrowed (with its NUL), and whether the byte after the room given (or
 * the first byte, for a call that does not fit) is still the 0xA5 fill.
 */
#include <windows.h>

static char out[4096];
static unsigned n;

static void put(const char *s) { while (*s) out[n++] = *s++; }
static void puti(long long v) {
    char t[24];
    int k = 0;
    if (v < 0) { out[n++] = '-'; v = -v; }
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, long long v) { put(",\""); put(k); put("\":"); puti(v); }
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void end(void) { put("}\n"); }

void start(void) {
    WCHAR w[MAX_PATH];
    DWORD len = GetCurrentDirectoryW(MAX_PATH, w);
    char narrow[MAX_PATH + 1];
    for (DWORD i = 0; i <= len; i++) narrow[i] = (char)w[i];
    static const struct { const char *name; long long size; int null; } forms[] = {
        {"a.null", 0, 1}, {"a.one", 1, 0}, {"a.exact", -1, 0}, {"a.room", -2, 0}, {"a.max_path", MAX_PATH, 0}};
    for (int f = 0; f < 5; f++) {
        DWORD size = forms[f].size == -1 ? len : forms[f].size == -2 ? len + 1 : (DWORD)forms[f].size;
        char buf[MAX_PATH + 8];
        for (int i = 0; i < MAX_PATH + 8; i++) buf[i] = (char)0xA5;
        SetLastError(0xDEAD);
        DWORD r = GetCurrentDirectoryA(size, forms[f].null ? NULL : buf);
        DWORD err = GetLastError();
        int fits = r < size;
        int same = 1;
        if (fits) for (DWORD i = 0; i <= r; i++) same &= buf[i] == narrow[i];
        row(forms[f].name); field("ret_less_n", (long long)r - (long long)len); field("error", err);
        if (!forms[f].null) {
            flag("text_equal", fits && same);
            flag("fill_kept", fits ? (unsigned char)buf[r + 1] == 0xA5 : (unsigned char)buf[0] == 0xA5);
        }
        end();
    }
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
