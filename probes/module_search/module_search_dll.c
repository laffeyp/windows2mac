// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* module_search_dll.c — the DLL the module_search probe loads only through SetDllDirectoryW (Sprint 063). No C
 * runtime. The probe copies this file into a subdirectory under another name, so no search finds it there except the
 * directory SetDllDirectoryW names. */
#include <windows.h>

BOOL WINAPI dll_main(HINSTANCE instance, DWORD reason, LPVOID reserved) {
    (void)instance; (void)reason; (void)reserved;
    return TRUE;
}

__declspec(dllexport) DWORD module_search_value(void) { return 42; }
