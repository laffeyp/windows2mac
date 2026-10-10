// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* local_free.c — LocalFree on CommandLineToArgvW's block and on pointers inside it (Sprint 048, family "local free").
 *
 * No C runtime. UnityPlayer.dll's function at 0x180527240 takes argv = CommandLineToArgvW(GetCommandLineW(), &n),
 * keeps argv + 1 and calls LocalFree(argv + 1) at 0x1805273e8: a pointer 8 bytes into the block. Run with "safe",
 * the probe prints a row per case that leaves the process alive (NULL; argv + 1; argv itself after it; a HeapAlloc
 * block + 8; the block itself): LocalFree's return (0 NULL, 1 the argument, 2 another value) and last error, and
 * whether the strings read the same after the interior free. Run with "16", it frees argv + 2 (16 bytes in), which
 * ends the process. Run with no argument, it runs both as children and prints their exit codes.
 */
#include <windows.h>
#include <shellapi.h>

#define SENTINEL 0xDEADu

static char out[4096];
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
static int same(const WCHAR *a, const WCHAR *b) { while (*a && *a == *b) { a++; b++; } return *a == *b; }

static void free_row(const char *name, HLOCAL p) {
    SetLastError(SENTINEL);
    HLOCAL r = LocalFree(p);
    DWORD err = GetLastError();
    put("{\"s\":\""); put(name); put("\",\"ret\":"); putu(r == NULL ? 0 : r == p ? 1 : 2); put(",\"err\":"); putu(err);
    put("}\n");
}

static void safe(void) {
    int argc = 0;
    LPWSTR *argv = CommandLineToArgvW(L"prog alpha beta", &argc);
    free_row("null", NULL);
    free_row("argv_plus_1", (HLOCAL)(argv + 1));
    put("{\"s\":\"after_interior\",\"strings_intact\":"); putu(argc == 3 && same(argv[1], L"alpha") && same(argv[2], L"beta"));
    put("}\n");
    free_row("argv", (HLOCAL)argv);
    void *heap = HeapAlloc(GetProcessHeap(), 0, 64);
    free_row("heap_block_plus_8", (HLOCAL)((char *)heap + 8));
    free_row("heap_block", (HLOCAL)heap);
    flush();
    ExitProcess(0);
}

static void sixteen(void) {
    int argc = 0;
    LPWSTR *argv = CommandLineToArgvW(L"prog alpha beta", &argc);
    LocalFree((HLOCAL)(argv + 2));
    ExitProcess(0);
}

static int ends_with(const WCHAR *line, const WCHAR *tail) {
    const WCHAR *a = line, *b = tail;
    while (*a) a++;
    while (*b) b++;
    while (b > tail) if (a == line || *--a != *--b) return 0;
    return a > line && a[-1] == ' ';
}

static void run_child(const WCHAR *self, const WCHAR *arg, const char *name) {
    static WCHAR cmd[MAX_PATH + 16];
    unsigned k = 0;
    cmd[k++] = '"';
    for (const WCHAR *s = self; *s;) cmd[k++] = *s++;
    cmd[k++] = '"'; cmd[k++] = ' ';
    for (const WCHAR *s = arg; *s;) cmd[k++] = *s++;
    cmd[k] = 0;
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
    put("{\"s\":\"exit."); put(name); put("\",\"code\":"); putu(code); put("}\n");
    flush();
}

void start(void) {
    const WCHAR *line = GetCommandLineW();
    if (ends_with(line, L"safe")) safe();
    if (ends_with(line, L"16")) sixteen();
    WCHAR self[MAX_PATH];
    GetModuleFileNameW(NULL, self, MAX_PATH);
    run_child(self, L"safe", "safe");
    run_child(self, L"16", "argv_plus_2");
    ExitProcess(0);
}
