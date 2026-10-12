// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* system_fonts.c — the runner's C:\Windows\Fonts as UnityPlayer.dll lists it (Sprint 065).
 *
 * Game run 38216081440 seq 2775-2778: GetWindowsDirectoryW, then FindFirstFileExW("C:\Windows\Fonts\*",
 * FindExInfoBasic, &data, FindExSearchNameMatch, NULL, 0) and FindNextFileW to the end; no font file is opened in
 * the run. One JSON line per entry in the order Windows answers: the name (UTF-8), dwFileAttributes, the size, the
 * creation, last access and last write times (FILETIME, decimal) and the reparse tag (dwReserved0); then a line
 * with the entry count, the last error FindNextFileW ended with and the Windows directory.
 */
#include <windows.h>
#include <stdio.h>

static void name(const WCHAR *w) {
    char u[1024];
    int n = WideCharToMultiByte(CP_UTF8, 0, w, -1, u, sizeof u, NULL, NULL);
    putchar('"');
    for (int i = 0; i + 1 < n; i++) {
        unsigned char c = (unsigned char)u[i];
        if (c == '"' || c == '\\') printf("\\%c", c);
        else if (c < 0x20) printf("\\u%04x", c);
        else putchar(c);
    }
    putchar('"');
}

static unsigned long long ft(FILETIME t) { return (unsigned long long)t.dwHighDateTime << 32 | t.dwLowDateTime; }

int main(void) {
    WCHAR dir[MAX_PATH], pattern[MAX_PATH + 8];
    UINT n = GetWindowsDirectoryW(dir, MAX_PATH);
    if (!n || n >= MAX_PATH) return 1;
    lstrcpyW(pattern, dir);
    lstrcatW(pattern, L"\\Fonts\\*");
    WIN32_FIND_DATAW d;
    HANDLE h = FindFirstFileExW(pattern, FindExInfoBasic, &d, FindExSearchNameMatch, NULL, 0);
    if (h == INVALID_HANDLE_VALUE) {
        printf("{\"error\": %lu}\n", GetLastError());
        return 2;
    }
    unsigned count = 0;
    do {
        printf("{\"name\": ");
        name(d.cFileName);
        printf(", \"attributes\": %lu, \"size\": %llu, \"creation\": %llu, \"access\": %llu, \"write\": %llu, "
               "\"reserved0\": %lu}\n", d.dwFileAttributes, (unsigned long long)d.nFileSizeHigh << 32 | d.nFileSizeLow,
               ft(d.ftCreationTime), ft(d.ftLastAccessTime), ft(d.ftLastWriteTime), d.dwReserved0);
        count++;
    } while (FindNextFileW(h, &d));
    DWORD last = GetLastError();
    FindClose(h);
    printf("{\"count\": %u, \"last_error\": %lu, \"windows_directory\": ", count, last);
    name(dir);
    printf("}\n");
    return 0;
}
