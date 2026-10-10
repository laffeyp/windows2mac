// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* local_free.c — LocalFree on CommandLineToArgvW's block and on pointers inside it (Sprint 048, family "local free").
 *
 * No C runtime. UnityPlayer.dll's function at 0x180527240 takes argv = CommandLineToArgvW(GetCommandLineW(), &n),
 * keeps argv + 1 and calls LocalFree(argv + 1) at 0x1805273e8: a pointer 8 bytes into the block. A first version of
 * this probe ran every case in one process and died with STATUS_HEAP_CORRUPTION (0xC0000374) before printing. Each
 * case now runs in a child process (this program with the case's number); the parent prints the child's exit code
 * and the child prints LocalFree's return (0 NULL, 1 the argument, 2 another value) and last error.
 */
#include <windows.h>
#include <shellapi.h>

#define SENTINEL 0xDEADu

static char out[256];
static unsigned n;

static void put(const char *s) { while (*s) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void flush(void) {
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}

static const char *const CASES[] = {"null", "argv", "argv_plus_1", "argv_plus_2", "heap_block", "heap_block_plus_8"};
#define CASE_COUNT 6

static void child(int c) {
    int argc = 0;
    LPWSTR *argv = CommandLineToArgvW(L"prog alpha beta", &argc);
    void *heap = HeapAlloc(GetProcessHeap(), 0, 64);
    HLOCAL p = c == 0 ? NULL : c == 1 ? (HLOCAL)argv : c == 2 ? (HLOCAL)(argv + 1) : c == 3 ? (HLOCAL)(argv + 2)
             : c == 4 ? (HLOCAL)heap : (HLOCAL)((char *)heap + 8);
    SetLastError(SENTINEL);
    HLOCAL r = LocalFree(p);
    DWORD err = GetLastError();
    put(",\"ret\":"); putu(r == NULL ? 0 : r == p ? 1 : 2); put(",\"err\":"); putu(err);
    flush();
    ExitProcess(0);
}

void start(void) {
    const WCHAR *line = GetCommandLineW();
    const WCHAR *last = line;
    for (const WCHAR *s = line; *s; s++) last = s;
    if (*last >= '0' && *last <= '9' && last > line && last[-1] == ' ') child(*last - '0');
    WCHAR self[MAX_PATH];
    GetModuleFileNameW(NULL, self, MAX_PATH);
    for (int c = 0; c < CASE_COUNT; c++) {
        static WCHAR cmd[MAX_PATH + 8];
        unsigned k = 0;
        cmd[k++] = '"';
        for (const WCHAR *s = self; *s;) cmd[k++] = *s++;
        cmd[k++] = '"'; cmd[k++] = ' '; cmd[k++] = (WCHAR)('0' + c); cmd[k] = 0;
        put("{\"s\":\""); put(CASES[c]); put("\"");
        flush();                                  /* the child's fields follow on the shared standard output */
        STARTUPINFOW si;
        PROCESS_INFORMATION pi;
        unsigned char *z = (unsigned char *)&si;
        for (unsigned i = 0; i < sizeof si; i++) z[i] = 0;
        si.cb = sizeof si;
        DWORD code = 0xFFFFFFFF;
        if (CreateProcessW(NULL, cmd, NULL, NULL, TRUE, 0, NULL, NULL, &si, &pi)) {
            WaitForSingleObject(pi.hProcess, INFINITE);
            GetExitCodeProcess(pi.hProcess, &code);
            CloseHandle(pi.hThread);
            CloseHandle(pi.hProcess);
        }
        put(",\"child_exit\":"); putu(code); put("}\n");
        flush();
    }
    ExitProcess(0);
}
