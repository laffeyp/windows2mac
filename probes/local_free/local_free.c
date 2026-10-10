// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* local_free.c — LocalFree on CommandLineToArgvW's block and on pointers inside it (Sprint 048, family "local free").
 *
 * No C runtime. UnityPlayer.dll's function at 0x180527240 takes argv = CommandLineToArgvW(GetCommandLineW(), &n),
 * keeps argv + 1, and calls LocalFree(argv + 1) at 0x1805273e8: a pointer 8 bytes into the block. The game runs on
 * Windows, so Windows tolerates it. Rows: LocalFree's return (NULL, the argument, or another value) and last error
 * (after SetLastError(0xDEAD)) for NULL, a pointer 8 and 16 bytes into a block, the block itself afterwards, and a
 * HeapAlloc block; and whether the block's strings still read the same after the interior free.
 */
#include <windows.h>
#include <shellapi.h>

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

/* LocalFree(p): 0 for NULL returned, 1 for p itself, 2 for anything else */
static void free_row(const char *name, HLOCAL p) {
    SetLastError(SENTINEL);
    HLOCAL r = LocalFree(p);
    DWORD err = GetLastError();
    row(name); field("ret", r == NULL ? 0 : r == p ? 1 : 2); field("err", err); end();
}

static int same(const WCHAR *a, const WCHAR *b) { while (*a && *a == *b) { a++; b++; } return *a == *b; }

void start(void) {
    free_row("null", NULL);
    int argc = 0;
    LPWSTR *argv = CommandLineToArgvW(L"prog alpha beta", &argc);
    free_row("argv_plus_1", (HLOCAL)(argv + 1));
    row("after_interior"); field("strings_intact", argc == 3 && same(argv[1], L"alpha") && same(argv[2], L"beta")); end();
    free_row("argv_plus_2", (HLOCAL)(argv + 2));
    free_row("argv", (HLOCAL)argv);
    LPWSTR *again = CommandLineToArgvW(L"prog gamma", &argc);
    row("after_free"); field("new_block_ok", again != NULL && argc == 2 && same(again[1], L"gamma")); end();
    free_row("again", (HLOCAL)again);
    void *h = HeapAlloc(GetProcessHeap(), 0, 64);
    free_row("heap_block", (HLOCAL)h);
    void *h2 = HeapAlloc(GetProcessHeap(), 0, 64);
    free_row("heap_block_plus_8", (HLOCAL)((char *)h2 + 8));
    SetLastError(SENTINEL);
    BOOL ok = HeapFree(GetProcessHeap(), 0, h2);
    DWORD err = GetLastError();
    row("heap_block_after_interior"); field("heapfree_ok", ok != 0); field("err", err); end();
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
