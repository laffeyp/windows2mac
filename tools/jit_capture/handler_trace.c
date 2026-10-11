// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* handler_trace.c — UnityCrashHandler64.exe's import calls, logged as jit_capture logs the game's (Sprint 055).
 *
 * jit_capture.exe starts the handler suspended and queues LoadLibraryW of this DLL as an APC on its main thread
 * (QueueUserAPC); the APC runs once the loader has bound the handler's static imports and before its entry point.
 * DllMain then points every import slot of the handler's own image at a counting, tracing thunk (call_trace.h), so the
 * handler runs as its own exe (its manifest, resources and module handle unchanged) while its calls are recorded. The
 * files are jit_capture's, under "<W2M_JIT_CAPTURE>.handler": .imports.tsv, .dynamic.tsv, .trace.tsv, .strings.tsv,
 * .threads.txt, rewritten every 5 s and when the process detaches.
 */
#include <windows.h>
#include <intrin.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "call_trace.h"

BOOL WINAPI DllMain(HINSTANCE self, DWORD reason, LPVOID reserved) {
    (void)self, (void)reserved;
    if (reason == DLL_PROCESS_ATTACH) {
        WCHAR base[MAX_PATH];
        DWORD n = GetEnvironmentVariableW(L"W2M_JIT_CAPTURE", base, MAX_PATH - 16);
        if (!n || n >= MAX_PATH - 16) return TRUE;                /* not under a capture: trace nothing */
        lstrcatW(base, L".handler");
        if (trace_init(base)) return TRUE;
        hook_image(GetModuleHandleW(NULL), "UnityCrashHandler64.exe");
        write_logs();
        CreateThread(NULL, 0, monitor, NULL, 0, NULL);
    } else if (reason == DLL_PROCESS_DETACH && hooks) {
        write_logs();
    }
    return TRUE;
}
