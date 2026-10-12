// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* threads_dll.c — the DLL the threads probe imports (Sprint 051): static TLS (one initialized thread variable), a
 * TLS callback and DllMain that each log every call with its reason and thread id, so the probe can print what
 * Windows runs on a thread's start and end. Built with the static C runtime, whose tlssup supplies the TLS
 * directory (as static_tls_dll.c). */
#include <windows.h>

__declspec(thread) int counter = 41;

#define EVENTS 64
static struct { int kind; DWORD reason; DWORD tid; } events[EVENTS];
static volatile LONG count;

static void log_event(int kind, DWORD reason) {
    LONG i = InterlockedIncrement(&count) - 1;
    if (i < EVENTS) { events[i].kind = kind; events[i].reason = reason; events[i].tid = GetCurrentThreadId(); }
}

static void NTAPI on_tls(PVOID instance, DWORD reason, PVOID reserved) {
    (void)instance; (void)reserved;
    log_event(1, reason);
}
#ifdef _MSC_VER                                   /* the runner's clang targets MSVC; the gate's mingw check does not */
#pragma section(".CRT$XLB", read)
#endif
__declspec(allocate(".CRT$XLB")) PIMAGE_TLS_CALLBACK threads_dll_callback = on_tls;

/* FLS inside a thread's attach (Sprint 061): the game's C runtime reads FLS in DLL_THREAD_ATTACH, and Windows
 * answered NULL with error 87 there (game run 38203364352 seq 1254) where a thread's own code later reads error 0 */
static DWORD fls_watch;
static volatile ULONG_PTR fls_at_attach = 0xA5A5;
static volatile DWORD fls_err_at_attach = 0xA5A5;

BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, LPVOID reserved) {
    (void)instance; (void)reserved;
    log_event(2, reason);
    if (reason == DLL_THREAD_ATTACH && fls_watch) {
        DWORD kept = GetLastError();                   /* the thread's first last error is a row (worker.start) */
        SetLastError(0xDEAD);
        fls_at_attach = (ULONG_PTR)FlsGetValue(fls_watch);
        fls_err_at_attach = GetLastError();
        fls_watch = 0;                                 /* the first thread's attach only */
        SetLastError(kept);
    }
    return TRUE;
}

__declspec(dllexport) void threads_dll_watch_fls(DWORD index) { fls_watch = index; }
__declspec(dllexport) void threads_dll_fls_seen(ULONG_PTR *value, DWORD *err) {
    *value = fls_at_attach;
    *err = fls_err_at_attach;
}

/* kind (1 TLS callback, 2 DllMain), reason and thread id of event i; the number of events */
__declspec(dllexport) LONG threads_dll_count(void) { return count < EVENTS ? count : EVENTS; }
__declspec(dllexport) int threads_dll_event(LONG i, DWORD *reason, DWORD *tid) {
    *reason = events[i].reason;
    *tid = events[i].tid;
    return events[i].kind;
}
__declspec(dllexport) int *threads_dll_counter(void) { return &counter; }
