// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* late_tls_dll.c — a DLL with static TLS that the late_tls probe loads with LoadLibrary (Sprint 065), built as
 * static_tls_dll.c is (the static C runtime's tlssup supplies the TLS directory). */
#include <windows.h>

__declspec(thread) unsigned long long marker = 0x1122334455667788ull;
__declspec(thread) unsigned char zeros[32];

extern ULONG _tls_index;

BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, LPVOID reserved) {
    (void)instance; (void)reserved;
    if (reason == DLL_PROCESS_ATTACH) marker += 1;      /* DllMain runs on the loading thread, after its block exists */
    return TRUE;
}

__declspec(dllexport) ULONG late_index(void) { return _tls_index; }
__declspec(dllexport) unsigned long long *late_marker(void) { return &marker; }
__declspec(dllexport) unsigned char *late_zeros(void) { return zeros; }
