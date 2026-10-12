// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* processes.c — a second process, its pipe and the handles they share (Sprint 055, family "processes";
 * specs/work_list.json).
 *
 * contracts/windows.process.processes records Windows (Server 2025 and 2022) for UnityPlayer's start of its crash
 * handler and the handler's side (game run 38176899195):
 *   - CreatePipe answers two handles of FILE_TYPE_PIPE, inheritable as the SECURITY_ATTRIBUTES say, the last error
 *     left; SetHandleInformation clears HANDLE_FLAG_INHERIT on one
 *   - CreateProcessW(NULL, command line, NULL, NULL, TRUE, NORMAL_PRIORITY_CLASS, NULL, NULL, STARTUPINFO with
 *     STARTF_USESTDHANDLES, ...) answers TRUE, the last error 0, a process and a thread handle, ids that are not the
 *     caller's; the child gets the inheritable handles at the same values, the STARTUPINFO's as its standard handles
 *     (NULL output and error included), and the command line as given
 *   - a running child: WaitForSingleObject(0) WAIT_TIMEOUT, GetExitCodeProcess STILL_ACTIVE (259); after it exits,
 *     WAIT_OBJECT_0 and its exit code
 *   - a pipe's reader gets what was written, in order, as much as it asks and is there; with the writers' handles
 *     all closed and nothing left, ReadFile answers FALSE with ERROR_BROKEN_PIPE (109), a writer's process ending
 *     included (rows writer.*; Sprint 058)
 *   - a pipe made with size 0 holds 4096 bytes: a writer waits while it is full, and a write larger than the room
 *     left returns once the reader has taken enough that the rest fits (rows pipe.capacity, pipe.large_write)
 *   - OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, FALSE, the parent's id) answers a handle, the last
 *     error left; WaitForMultipleObjectsEx answers the first signaled index, as WaitForMultipleObjects
 *   - (contracts windows.time.local_and_error_mode 0.2.0) an application name that names the command line's exe, no
 *     STARTF_USESTDHANDLES: the child is a start of its own, and inherits the error mode
 *   - SetConsoleCtrlHandler adds a handler (TRUE), removes it (TRUE), and removing one it lacks answers FALSE with
 *     ERROR_INVALID_PARAMETER
 * The other process is a VM of its own; the events, pipes and process records both use are the host's
 * (src/w2m/kernel.py, host request W2M_HOST_KERNEL). Other forms (an application name, an environment, a
 * directory, creation flags but NORMAL_PRIORITY_CLASS, no standard handles, a write no process reads) stop the run.
 */
#include "runtime.h"

typedef unsigned short u16;

#define ERROR_INVALID_HANDLE 6u
#define ERROR_INVALID_PARAMETER 87u
#define ERROR_BROKEN_PIPE 109u
#define HANDLE_FLAG_INHERIT 1u
#define NORMAL_PRIORITY_CLASS 0x20u
#define STARTF_USESTDHANDLES 0x100u
#define STILL_ACTIVE 259u
#define INHERITED 64

u64 w2m_object_new(u64 api, u32 kind, s64 count, u32 flags);    /* sync_objects.c */
u32 w2m_object_kind(u64 h, s64 *count);
u32 *w2m_object_flags(u64 h);
int w2m_object_inheritable(u64 (*out)[3], int room);
const u16 *w2m_working_directory(void);                         /* paths.c */
u64 k32_WaitForMultipleObjects(u64 count, u64 handles, u64 all, u64 ms);

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }
static u32 current_pid(void) { return (u32)w2m_teb()->client_process; }

/* CreatePipe(hReadPipe, hWritePipe, lpPipeAttributes, nSize) */
u64 k32_CreatePipe(u64 read, u64 write, u64 attributes, u64 size) {
    if ((u32)size) w2m_unimplemented(W2M_API_CreatePipe, size);   /* a size of its own: not in the rows */
    u32 inherit = 0;
    if (attributes) {
        const u8 *sa = (const u8 *)attributes;             /* nLength, lpSecurityDescriptor, bInheritHandle */
        if (*(const u64 *)(sa + 8)) w2m_unimplemented(W2M_API_CreatePipe, attributes);
        inherit = *(const u32 *)(sa + 16) ? HANDLE_FLAG_INHERIT : 0;
    }
    u64 w = 0;
    u64 r = w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_PIPE_CREATE, 0, 0, 0, &w);
    *(u64 *)read = w2m_object_new(W2M_API_CreatePipe, W2M_OBJECT_PIPE_READ, (s64)r, inherit);
    *(u64 *)write = w2m_object_new(W2M_API_CreatePipe, W2M_OBJECT_PIPE_WRITE, (s64)w, inherit);
    return w2m_logged(W2M_API_CreatePipe, r, 1);
}

/* file.c's ReadFile on a pipe's read end: 1 done, 0 failed (the error set), -1 the thread blocks and retries. */
int w2m_pipe_read(u64 handle, u64 buffer, u64 count, u64 read) {
    s64 id = 0;
    if (w2m_object_kind(handle, &id) != W2M_OBJECT_PIPE_READ) return -2;
    u64 status = 0;
    u64 n = w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_PIPE_READ, (u64)id, buffer, count & 0xFFFFFFFFull, &status);
    if (status == 1) {                                     /* nothing yet and a writer left: wait for it */
        w2m_block(~0ull);
        return -1;
    }
    w2m_thread_scratch()->wait_deadline = 0;
    *(u32 *)read = (u32)n;
    if (status == 2) {
        set_error(ERROR_BROKEN_PIPE);
        return 0;
    }
    return 1;
}

/* file.c's WriteFile on a pipe's write end: 1 written, -1 the pipe is full (the thread blocks and retries, the bytes
 * taken so far in its scratch), -2 not a pipe. The host takes what fits. */
int w2m_pipe_write(u64 handle, u64 buffer, u64 count, u64 written) {
    s64 id = 0;
    if (w2m_object_kind(handle, &id) != W2M_OBJECT_PIPE_WRITE) return -2;
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    u64 done = sc->progress;
    done += w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_PIPE_WRITE, (u64)id, buffer + done, (count & 0xFFFFFFFFull) - done, 0);
    w2m_signal();
    if (done < (count & 0xFFFFFFFFull)) {
        sc->progress = done;
        w2m_block(~0ull);
        return -1;
    }
    sc->progress = 0;
    sc->wait_deadline = 0;
    *(u32 *)written = (u32)done;
    return 1;
}

int w2m_is_pipe(u64 handle) {
    s64 id = 0;
    u32 k = w2m_object_kind(handle, &id);
    return k == W2M_OBJECT_PIPE_READ || k == W2M_OBJECT_PIPE_WRITE;
}

/* Whether an application name names the command line's first token (quoted or not), without case: the one form
 * of both the rows hold (time_and_error_mode's child). */
static int names_first(u64 application, u64 command_line) {
    const u16 *a = (const u16 *)application, *c = (const u16 *)command_line;
    u16 end = ' ';
    if (*c == '"') { c++; end = '"'; }
    for (; *a; a++, c++) {
        u16 x = *a >= 'A' && *a <= 'Z' ? (u16)(*a + 32) : *a, y = *c >= 'A' && *c <= 'Z' ? (u16)(*c + 32) : *c;
        if (x != y) return 0;
    }
    return *c == end || *c == 0;
}

/* CreateProcessW(lpApplicationName, lpCommandLine, lpProcessAttributes, lpThreadAttributes, bInheritHandles,
 * dwCreationFlags, lpEnvironment, lpCurrentDirectory, lpStartupInfo, lpProcessInformation) */
u64 k32_CreateProcessW(u64 application, u64 command_line, u64 process_attributes, u64 thread_attributes, u64 rsp) {
    const u64 *stack = (const u64 *)(rsp + 0x28);
    u32 inherit = (u32)stack[0], flags = (u32)stack[1];
    u64 environment = stack[2], directory = stack[3], startup = stack[4], info = stack[5];
    if (!command_line || process_attributes || thread_attributes || environment || directory ||
        (flags & ~NORMAL_PRIORITY_CLASS) || (application && !names_first(application, command_line)))
        w2m_unimplemented(W2M_API_CreateProcessW, flags);              /* not in the rows */
    const u8 *si = (const u8 *)startup;                               /* STARTUPINFOW: cb, ..., dwFlags at 60 */
    u32 si_flags = *(const u32 *)(si + 60);
    if (*(const u32 *)si < 104 || (si_flags & ~STARTF_USESTDHANDLES))
        w2m_unimplemented(W2M_API_CreateProcessW, si_flags);
    static u64 list[INHERITED + 5][3];
    int n = inherit ? w2m_object_inheritable(list, INHERITED) : 0;
    for (int i = 0; (si_flags & STARTF_USESTDHANDLES) && i < 3; i++) {   /* hStdInput, hStdOutput, hStdError */
        list[n][0] = (u64)i;
        list[n][1] = W2M_INHERIT_STANDARD;
        list[n][2] = *(const u64 *)(si + 80 + 8 * i);
        n++;
    }
    list[n][0] = 0;                                                   /* "A child process inherits the error mode" */
    list[n][1] = W2M_INHERIT_ERROR_MODE;
    list[n][2] = w2m_error_mode();
    n++;
    const u16 *working = w2m_working_directory();                     /* "the same current drive and directory" */
    u64 working_chars = 0;
    while (working[working_chars]) working_chars++;
    list[n][0] = working_chars;
    list[n][1] = W2M_INHERIT_DIRECTORY;
    list[n][2] = (u64)working;
    n++;
    u64 chars = 0;
    while (((const u16 *)command_line)[chars]) chars++;
    u64 tid = 0;
    u64 pid = w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_PROCESS_CREATE, command_line, chars | (u64)n << 32, (u64)list, &tid);
    u8 *pi = (u8 *)info;                                              /* PROCESS_INFORMATION */
    *(u64 *)pi = w2m_object_new(W2M_API_CreateProcessW, 7, (s64)pid, 0);   /* PROCESS */
    *(u64 *)(pi + 8) = w2m_object_new(W2M_API_CreateProcessW, W2M_OBJECT_REMOTE_THREAD, (s64)tid, 0);
    *(u32 *)(pi + 16) = (u32)pid;
    *(u32 *)(pi + 20) = (u32)tid;
    set_error(0);                                                     /* row process.create */
    return w2m_logged(W2M_API_CreateProcessW, pid, 1);
}

/* OpenProcess(dwDesiredAccess, bInheritHandle, dwProcessId) */
u64 k32_OpenProcess(u64 access, u64 inherit, u64 pid) {
    (void)access;                                                     /* access checks: not in the rows */
    if ((u32)inherit) w2m_unimplemented(W2M_API_OpenProcess, inherit);
    if ((u32)pid != current_pid() &&
        w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_PROCESS_STATE, (u32)pid, 0, 0, 0) == ~0ull) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_OpenProcess, pid, 0);
    }
    if ((u32)pid != current_pid()) w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_REF, (u32)pid, 0, 0, 0);
    return w2m_logged(W2M_API_OpenProcess, pid, w2m_object_new(W2M_API_OpenProcess, 7, (s64)(u32)pid, 0));
}

/* GetExitCodeProcess(hProcess, lpExitCode) */
u64 k32_GetExitCodeProcess(u64 h, u64 out) {
    s64 pid = 0;
    if (w2m_object_kind(h, &pid) != 7) w2m_unimplemented(W2M_API_GetExitCodeProcess, h);   /* not in the rows */
    u64 code = (u32)pid == current_pid() ? STILL_ACTIVE
                                          : w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_PROCESS_STATE, (u64)pid, 0, 0, 0);
    *(u32 *)out = (u32)code;
    return w2m_logged(W2M_API_GetExitCodeProcess, h, 1);
}

/* WaitForMultipleObjectsEx(nCount, lpHandles, bWaitAll, dwMilliseconds, bAlertable): no APC is ever queued */
u64 k32_WaitForMultipleObjectsEx(u64 count, u64 handles, u64 all, u64 ms, u64 rsp) {
    (void)rsp;
    return k32_WaitForMultipleObjects(count, handles, all, ms);
}

/* SetConsoleCtrlHandler(HandlerRoutine, Add): the list Windows would call on Ctrl+C; w2m sends no console event */
static u64 handlers[32];
static int nhandlers;

u64 k32_SetConsoleCtrlHandler(u64 routine, u64 add) {
    if (!routine) w2m_unimplemented(W2M_API_SetConsoleCtrlHandler, add);   /* ignoring Ctrl+C: not in the rows */
    if ((u32)add) {
        if (nhandlers == 32) w2m_unimplemented(W2M_API_SetConsoleCtrlHandler, 32);
        handlers[nhandlers++] = routine;
        return w2m_logged(W2M_API_SetConsoleCtrlHandler, routine, 1);
    }
    for (int i = nhandlers - 1; i >= 0; i--)
        if (handlers[i] == routine) {
            for (int j = i; j < nhandlers - 1; j++) handlers[j] = handlers[j + 1];
            nhandlers--;
            return w2m_logged(W2M_API_SetConsoleCtrlHandler, routine, 1);
        }
    set_error(ERROR_INVALID_PARAMETER);
    return w2m_logged(W2M_API_SetConsoleCtrlHandler, routine, 0);
}
