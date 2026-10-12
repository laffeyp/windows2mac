// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* system_fonts_copy.c — the runner's C:\Windows\Fonts, every file, into the run's artifact (Sprint 065).
 *
 * The frontier of 2026-10-12 opens C:\Windows\Fonts\Candara.ttf after 49,661,661,136 instructions; probe run
 * 38222954208 recorded the folder's names, attributes, sizes and times only. This copies each file the folder lists to
 * out\fonts\<name> and prints one JSON line per file: its name, CopyFileW's answer and last error, and the size
 * GetFileSizeEx reads from the copy; then the count copied and failed.
 */
#include <windows.h>
#include <stdio.h>

static void name(const WCHAR *w) {
    char u[1024];
    int n = WideCharToMultiByte(CP_UTF8, 0, w, -1, u, sizeof u, NULL, NULL);
    putchar('"');
    for (int i = 0; i + 1 < n; i++) {
        if (u[i] == '"' || u[i] == '\\') putchar('\\');
        putchar(u[i]);
    }
    putchar('"');
}

int main(void) {
    WCHAR dir[MAX_PATH], pattern[MAX_PATH + 16], from[MAX_PATH * 2], to[MAX_PATH * 2];
    UINT n = GetWindowsDirectoryW(dir, MAX_PATH);
    if (!n || n >= MAX_PATH) return 1;
    lstrcpyW(pattern, dir);
    lstrcatW(pattern, L"\\Fonts\\*");
    if (!CreateDirectoryW(L"out\\fonts", NULL) && GetLastError() != ERROR_ALREADY_EXISTS) return 2;
    WIN32_FIND_DATAW d;
    HANDLE h = FindFirstFileExW(pattern, FindExInfoBasic, &d, FindExSearchNameMatch, NULL, 0);
    if (h == INVALID_HANDLE_VALUE) return 3;
    unsigned copied = 0, failed = 0;
    do {
        if (d.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) continue;
        lstrcpyW(from, dir); lstrcatW(from, L"\\Fonts\\"); lstrcatW(from, d.cFileName);
        lstrcpyW(to, L"out\\fonts\\"); lstrcatW(to, d.cFileName);
        BOOL ok = CopyFileW(from, to, FALSE);
        DWORD err = ok ? 0 : GetLastError();
        LARGE_INTEGER size = {0};
        HANDLE f = ok ? CreateFileW(to, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, 0, NULL) : INVALID_HANDLE_VALUE;
        if (f != INVALID_HANDLE_VALUE) { GetFileSizeEx(f, &size); CloseHandle(f); }
        printf("{\"name\": ");
        name(d.cFileName);
        printf(", \"copied\": %d, \"error\": %lu, \"size\": %lld}\n", ok ? 1 : 0, err, size.QuadPart);
        if (ok) copied++; else failed++;
    } while (FindNextFileW(h, &d));
    FindClose(h);
    printf("{\"copied\": %u, \"failed\": %u}\n", copied, failed);
    return 0;
}
