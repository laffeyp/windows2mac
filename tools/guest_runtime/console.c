// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* console.c — standard handles, the console and the command line (Sprint 044, family "console").
 *
 * GetStdHandle, SetStdHandle, GetFileType, GetConsoleMode, GetConsoleCP, WriteConsoleW,
 * FlushFileBuffers, GetCommandLineW and GetCommandLineA, as contracts/windows.console.gui_start
 * records Windows answering a GUI process started without handles (the Steam-like start; BLACKBOARD
 * 2026-10-10) on four runners:
 *   - GetStdHandle answers NULL for all three, leaving the last error
 *   - GetFileType(NULL): FILE_TYPE_UNKNOWN, ERROR_INVALID_HANDLE; a disk file: FILE_TYPE_DISK
 *   - GetConsoleMode, GetConsoleCP, WriteConsoleW without a console: 0, ERROR_INVALID_HANDLE;
 *     WriteConsoleW leaves *lpNumberOfCharsWritten as it was
 *   - SetStdHandle succeeds and GetStdHandle then answers that handle
 * The start mode is the host's (src/w2m/vm_backend.py): 0, a console program whose standard output
 * the runner's cmd redirected (the probes), or 1, the GUI start. In the console start standard input
 * and error answer NULL, as for a process started without them: the runner's are its own pipes, no
 * probe reads them, and a probe's C runtime asks for them at start-up (Sprint 046). The command line is the host's too: ProcessParameters.CommandLine (Sprint 059).
 */
#include "runtime.h"

#define STD_INPUT_HANDLE 0xFFFFFFF6u
#define STD_OUTPUT_HANDLE 0xFFFFFFF5u
#define STD_ERROR_HANDLE 0xFFFFFFF4u
#define STDOUT_HANDLE 0x74ull                        /* runtime.c's redirected standard output */
#define ERROR_INVALID_HANDLE 6u
#define FILE_TYPE_UNKNOWN 0u
#define FILE_TYPE_DISK 1u

static char command_line_a[2048];           /* the process page holds 2047 units and a NUL (vm_backend.py) */

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

static int gui_start(void) {
    return *(const volatile u64 *)(W2M_MONITOR_DATA_VA + W2M_START_MODE_OFF) == 1;
}

static int std_index(u64 which) {
    which = (u32)which;
    return which == STD_INPUT_HANDLE ? 0 : which == STD_OUTPUT_HANDLE ? 1 : which == STD_ERROR_HANDLE ? 2 : -1;
}

/* The standard handles are ProcessParameters' (+0x20, +0x28, +0x30; Sprint 059): the host wrote the start's (the
 * GUI start: none; the console start: STDOUT_HANDLE as output; a child: the ones its parent gave), and SetStdHandle
 * changes them there (row after.set_std_handle). */
static u64 *std_handles(void) { return (u64 *)(w2m_parameters() + 0x20); }

u64 k32_GetStdHandle(u64 which) {
    int i = std_index(which);
    if (i < 0) w2m_unimplemented(W2M_API_GetStdHandle, (u32)which);
    return w2m_logged(W2M_API_GetStdHandle, (u32)which, std_handles()[i]);
}

u64 k32_SetStdHandle(u64 which, u64 handle) {
    int i = std_index(which);
    if (i < 0) w2m_unimplemented(W2M_API_SetStdHandle, (u32)which);
    std_handles()[i] = handle;
    return w2m_logged(W2M_API_SetStdHandle, (u32)which, 1);
}

u64 k32_GetFileType(u64 handle) {
    if (handle == 0 || handle == ~0ull) {
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_GetFileType, handle, FILE_TYPE_UNKNOWN);
    }
    if (w2m_file_is_open(handle)) return w2m_logged(W2M_API_GetFileType, handle, FILE_TYPE_DISK);
    if (w2m_is_pipe(handle)) return w2m_logged(W2M_API_GetFileType, handle, 3);      /* FILE_TYPE_PIPE (Sprint 055) */
    /* the console start's standard output: the runner's cmd redirected it into a file (> out\body-N.jsonl) */
    if (handle == STDOUT_HANDLE && !gui_start()) return w2m_logged(W2M_API_GetFileType, handle, FILE_TYPE_DISK);
    w2m_unimplemented(W2M_API_GetFileType, handle);
    return 0;
}

static u64 no_console(u64 api, u64 arg) {
    if (!gui_start()) w2m_unimplemented(api, arg);           /* a console exists there: not observed */
    set_error(ERROR_INVALID_HANDLE);
    return w2m_logged(api, arg, 0);
}

u64 k32_GetConsoleMode(u64 handle, u64 mode) {
    (void)mode;
    return no_console(W2M_API_GetConsoleMode, handle);
}

u64 k32_GetConsoleCP(void) {
    return no_console(W2M_API_GetConsoleCP, 0);
}

u64 k32_WriteConsoleW(u64 handle, u64 buffer, u64 count, u64 written, u64 rsp) {
    (void)buffer; (void)count; (void)written; (void)rsp;
    return no_console(W2M_API_WriteConsoleW, handle);
}

/* FlushFileBuffers: a disk file of file.c is written through already; nonzero. */
u64 k32_FlushFileBuffers(u64 handle) {
    if (!w2m_file_is_open(handle)) w2m_unimplemented(W2M_API_FlushFileBuffers, handle);
    return w2m_logged(W2M_API_FlushFileBuffers, handle, 1);
}

u64 k32_GetCommandLineW(void) {
    return w2m_logged(W2M_API_GetCommandLineW, 0, *(u64 *)(w2m_parameters() + 0x78));   /* CommandLine.Buffer */
}

/* GetCommandLineA: the same line, each character below 128 as itself (the line is ASCII; a wider
 * character would need the ANSI code page, which the code-pages family will bring). */
u64 k32_GetCommandLineA(void) {
    const unsigned short *w = *(const unsigned short *const *)(w2m_parameters() + 0x78);
    u64 i = 0;
    for (; w[i] && i + 1 < sizeof command_line_a; i++) {
        if (w[i] >= 128) w2m_unimplemented(W2M_API_GetCommandLineA, w[i]);
        command_line_a[i] = (char)w[i];
    }
    command_line_a[i] = 0;
    return w2m_logged(W2M_API_GetCommandLineA, 0, (u64)command_line_a);
}

/* For sync_objects.c's DuplicateHandle (Sprint 049): whether h is one of the standard handles. */
int w2m_is_std_handle(u64 h) {
    const u64 *s = std_handles();
    return h && (h == s[0] || h == s[1] || h == s[2] || h == STDOUT_HANDLE);
}
