// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* late_tls.c — static TLS in a DLL loaded with LoadLibrary after the process started (Sprint 065).
 *
 * The frontier of 2026-10-12 stops at LoadLibrary of steam_api64.dll (an 8-byte TLS template, no zero fill, no
 * callbacks) after 50,784,721,550 instructions: w2m has no rows for a late DLL's static TLS. This program starts a
 * worker that waits, loads late_tls_dll.dll (which it does not import), and prints: the index the DLL's _tls_index
 * holds; TEB.ThreadLocalStoragePointer before and after the load on the main thread, and the slot count it can see;
 * the marker each thread reads (main after DllMain, the worker that existed before the load, a thread started after
 * it), whether its 32 zero-fill bytes are zero, and whether the blocks of two threads are distinct.
 */
#include <windows.h>
#include <stdio.h>

typedef ULONG (*index_fn)(void);
typedef unsigned long long *(*marker_fn)(void);
typedef unsigned char *(*zeros_fn)(void);
static marker_fn marker;
static zeros_fn zeros;
static HANDLE go;

static void *tls_array(void) { return *(void **)((char *)NtCurrentTeb() + 0x58); }

static void report(const char *who) {
    unsigned long long *m = marker();
    unsigned char *z = zeros();
    int zero = 1;
    for (int i = 0; i < 32; i++) zero &= z[i] == 0;
    printf("{\"thread\": \"%s\", \"marker\": %llu, \"zeros\": %d, \"block\": \"%p\"}\n", who, *m, zero, (void *)m);
}

static DWORD WINAPI worker(void *arg) {
    (void)arg;
    WaitForSingleObject(go, INFINITE);
    report("before_load");
    return 0;
}

static DWORD WINAPI later(void *arg) {
    (void)arg;
    report("after_load");
    return 0;
}

int main(void) {
    go = CreateEventW(NULL, TRUE, FALSE, NULL);
    HANDLE w = CreateThread(NULL, 0, worker, NULL, 0, NULL);
    void *before = tls_array();
    HMODULE m = LoadLibraryW(L"late_tls_dll.dll");
    if (!m) { printf("{\"error\": %lu}\n", GetLastError()); return 1; }
    void *after = tls_array();
    index_fn index = (index_fn)GetProcAddress(m, "late_index");
    marker = (marker_fn)GetProcAddress(m, "late_marker");
    zeros = (zeros_fn)GetProcAddress(m, "late_zeros");
    ULONG i = index();
    printf("{\"index\": %lu, \"array_moved\": %d, \"slot_is_block\": %d}\n", i, before != after,
           ((void **)after)[i] == (void *)marker());
    report("main");
    SetEvent(go);
    WaitForSingleObject(w, INFINITE);
    HANDLE l = CreateThread(NULL, 0, later, NULL, 0, NULL);
    WaitForSingleObject(l, INFINITE);
    return 0;
}
