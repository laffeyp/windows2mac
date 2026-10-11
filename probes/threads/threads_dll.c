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
#pragma section(".CRT$XLB", read)
__declspec(allocate(".CRT$XLB")) PIMAGE_TLS_CALLBACK threads_dll_callback = on_tls;

BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, LPVOID reserved) {
    (void)instance; (void)reserved;
    log_event(2, reason);
    return TRUE;
}

/* kind (1 TLS callback, 2 DllMain), reason and thread id of event i; the number of events */
__declspec(dllexport) LONG threads_dll_count(void) { return count < EVENTS ? count : EVENTS; }
__declspec(dllexport) int threads_dll_event(LONG i, DWORD *reason, DWORD *tid) {
    *reason = events[i].reason;
    *tid = events[i].tid;
    return events[i].kind;
}
__declspec(dllexport) int *threads_dll_counter(void) { return &counter; }
