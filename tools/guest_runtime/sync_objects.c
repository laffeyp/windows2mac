// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* sync_objects.c — semaphores, events and mutexes on one thread (Sprint 046, family "sync objects").
 *
 * contracts/windows.sync.objects records Windows on four runners (8 identical outputs), one thread:
 *   - creating an unnamed semaphore, event or mutex gives a non-NULL handle, a multiple of 4, and sets the
 *     last error to 0; the handles are distinct
 *   - a zero-timeout wait: WAIT_OBJECT_0 when the object is signaled (a semaphore's count is taken, an
 *     auto-reset event resets, a mutex is owned again, recursively), WAIT_TIMEOUT (258) when not
 *   - ReleaseSemaphore past its maximum: FALSE, ERROR_TOO_MANY_POSTS (298), the previous count unwritten;
 *     ReleaseMutex once more than owned: FALSE, ERROR_NOT_OWNER (288)
 *   - WaitForMultipleObjects: the first signaled index (any) or WAIT_TIMEOUT when one is not (all)
 *   - CloseHandle twice: FALSE, ERROR_INVALID_HANDLE; a wait on a closed handle: WAIT_FAILED, the same error
 * One thread: a finite wait on an unsignaled object can only time out, so it returns WAIT_TIMEOUT at once;
 * an INFINITE one would never return and stops the run (Sprint 047), as do named objects, security
 * attributes and an initially owned mutex, which the rows do not show. Sleep spins on the guest counter.
 * Object handles are OBJECT_BASE + 4 * slot; file.c's CloseHandle passes them here.
 */
#include "runtime.h"

#define OBJECT_BASE 0x2000ull
#define OBJECTS 256
#define WAIT_OBJECT_0 0u
#define WAIT_TIMEOUT 258u
#define WAIT_FAILED 0xFFFFFFFFu
#define INFINITE 0xFFFFFFFFu
#define ERROR_INVALID_HANDLE 6u
#define ERROR_NOT_OWNER 288u
#define ERROR_TOO_MANY_POSTS 298u
#define ERROR_ALREADY_EXISTS 183u
#define CREATE_EVENT_MANUAL_RESET 1u
#define CREATE_EVENT_INITIAL_SET 2u

enum kind { FREE, SEMAPHORE, EVENT_MANUAL, EVENT_AUTO, MUTEX, TOKEN, THREAD, PROCESS, MAPPING,   /* TOKEN: token.c;
                                                                          * THREAD, PROCESS, MAPPING: Sprint 049 */
            SHARED_EVENT = W2M_OBJECT_SHARED_EVENT, PIPE_READ = W2M_OBJECT_PIPE_READ,
            PIPE_WRITE = W2M_OBJECT_PIPE_WRITE, REMOTE_THREAD = W2M_OBJECT_REMOTE_THREAD };
            /* Sprint 055 (processes.c): a named event, a pipe's ends and another process's thread, each the host's
             * object (src/w2m/kernel.py) whose id `count` holds; a PROCESS's `count` is its id, this one or another */

/* Objects, and the handles that name them (Sprint 049: DuplicateHandle gives a second handle to one object, which
 * lives until its last handle closes). count: semaphore count, event 0/1, mutex recursion, a thread's id, a file
 * mapping's host descriptor (its own; closed with the mapping); max: a mapping's size. */
static struct object { u32 kind; s64 count, max; u32 refs; } objects[OBJECTS];
static int handle_object[OBJECTS];              /* handle slot -> object index + 1; 0: the slot is free */
static u32 handle_flags[OBJECTS];               /* HANDLE_FLAG_INHERIT, HANDLE_FLAG_PROTECT_FROM_CLOSE (Sprint 055) */
static int inherited_ready;

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }
static u32 current_tid(void);

static void inherit_init(void);

static u64 handle_for(u64 api, int object) {
    inherit_init();
    for (int i = 0; i < OBJECTS; i++)
        if (!handle_object[i]) {
            handle_object[i] = object + 1;
            handle_flags[i] = 0;
            objects[object].refs++;
            return OBJECT_BASE + 4 * (u64)i;
        }
    w2m_unimplemented(api, OBJECTS);
    return 0;
}

static int new_object(u64 api, u32 kind, s64 count, s64 max) {
    for (int i = 0; i < OBJECTS; i++)
        if (objects[i].kind == FREE) {
            objects[i] = (struct object){kind, count, max, 0};
            return i;
        }
    w2m_unimplemented(api, OBJECTS);
    return 0;
}

static u64 make(u64 api, u32 kind, s64 count, s64 max) {
    u64 h = handle_for(api, new_object(api, kind, count, max));
    set_error(0);                                  /* rows *.create: err 0 */
    return w2m_logged(api, kind, h);
}

static int slot_of(u64 h) {
    inherit_init();
    if (h < OBJECT_BASE || (h - OBJECT_BASE) % 4 || (h - OBJECT_BASE) / 4 >= OBJECTS) return -1;
    int i = (int)((h - OBJECT_BASE) / 4);
    return handle_object[i] ? i : -1;
}

static struct object *object_of(u64 h) {
    int i = slot_of(h);
    return i < 0 ? 0 : &objects[handle_object[i] - 1];
}

/* For token.c: a process token's handle, from this table so CloseHandle closes it; and whether h is one. The
 * last error is left (row open). */
u64 w2m_object_token(u64 api) { return handle_for(api, new_object(api, TOKEN, 0, 0)); }
int w2m_object_is_token(u64 h) { struct object *o = object_of(h); return o && o->kind == TOKEN; }
int w2m_object_is_open(u64 h) { return object_of(h) != 0; }        /* file.c's handle calls (Sprint 049) */

/* For file.c's CreateFileMappingW and MapViewOfFile (Sprint 049): a mapping's handle, holding host descriptor fd
 * and the mapping's size; and a mapping's descriptor and size by handle (0 when h names no mapping). */
u64 w2m_object_mapping(u64 api, u64 fd, u64 size) { return handle_for(api, new_object(api, MAPPING, (s64)fd, (s64)size)); }
int w2m_object_mapping_of(u64 h, u64 *fd, u64 *size) {
    struct object *o = object_of(h);
    if (!o || o->kind != MAPPING) return 0;
    *fd = (u64)o->count;
    *size = (u64)o->max;
    return 1;
}

/* file.c's CloseHandle, for a handle that is not a file's: 1 when it named an object. */
int w2m_object_close(u64 h) {
    int i = slot_of(h);
    if (i < 0) return 0;
    struct object *o = &objects[handle_object[i] - 1];
    handle_object[i] = 0;
    if (--o->refs == 0) {
        if (o->kind == MAPPING) w2m_host(W2M_FILE_CLOSE, (u64)o->count, 0, 0, 0, 0);   /* the views keep their copy */
        if (o->kind == SHARED_EVENT || o->kind == PIPE_READ || o->kind == PIPE_WRITE ||
            (o->kind == PROCESS && (u32)o->count != (u32)w2m_teb()->client_process))
            w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_CLOSE, (u64)o->count, 0, 0, 0);   /* the host's handle count */
        o->kind = FREE;
    }
    return 1;
}

u64 k32_CreateSemaphoreExW(u64 attributes, u64 initial, u64 maximum, u64 name, u64 rsp) {
    u32 flags = *(u32 *)(rsp + 0x28);
    if (attributes || name || flags || (s32)maximum <= 0 || (s32)initial < 0 || (s32)initial > (s32)maximum)
        w2m_unimplemented(W2M_API_CreateSemaphoreExW, name);
    return make(W2M_API_CreateSemaphoreExW, SEMAPHORE, (s32)initial, (s32)maximum);
}

/* A named event is the host's, so other processes open it by name (rows event.create_*, child.event_*: a name
 * that exists answers a new handle to the same event and ERROR_ALREADY_EXISTS; a new one sets the last error 0). */
static u64 create_event(u64 api, u64 attributes, u64 manual, u64 initial, u64 name) {
    if (attributes) w2m_unimplemented(api, attributes);
    if (!name) return make(api, manual ? EVENT_MANUAL : EVENT_AUTO, initial ? 1 : 0, 1);
    if (api != W2M_API_CreateEventW && api != W2M_API_CreateEventExW) w2m_unimplemented(api, name);   /* ANSI names */
    u64 chars = 0;
    while (((const unsigned short *)name)[chars]) chars++;
    u64 existed = 0;
    u64 id = w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_EVENT_OPEN, name, chars, (manual ? 1 : 0) | (initial ? 2 : 0), &existed);
    u64 h = handle_for(api, new_object(api, SHARED_EVENT, (s64)id, 0));
    set_error(existed ? ERROR_ALREADY_EXISTS : 0);
    return w2m_logged(api, existed, h);
}

u64 k32_CreateEventW(u64 attributes, u64 manual, u64 initial, u64 name) {
    return create_event(W2M_API_CreateEventW, attributes, (u32)manual, (u32)initial, name);
}

u64 k32_CreateEventA(u64 attributes, u64 manual, u64 initial, u64 name) {
    return create_event(W2M_API_CreateEventA, attributes, (u32)manual, (u32)initial, name);
}

u64 k32_CreateEventExW(u64 attributes, u64 name, u64 flags, u64 access) {
    (void)access;
    if ((u32)flags & ~(CREATE_EVENT_MANUAL_RESET | CREATE_EVENT_INITIAL_SET)) w2m_unimplemented(W2M_API_CreateEventExW, flags);
    return create_event(W2M_API_CreateEventExW, attributes, (u32)flags & CREATE_EVENT_MANUAL_RESET,
                        (u32)flags & CREATE_EVENT_INITIAL_SET, name);
}

u64 k32_CreateMutexA(u64 attributes, u64 owned, u64 name) {
    if (attributes || name || (u32)owned) w2m_unimplemented(W2M_API_CreateMutexA, name);
    return make(W2M_API_CreateMutexA, MUTEX, 0, 0);
}

static struct object *must(u64 api, u64 h, u32 kind1, u32 kind2) {
    struct object *o = object_of(h);
    if (!o || (o->kind != kind1 && o->kind != kind2)) w2m_unimplemented(api, h);   /* not in the rows */
    return o;
}

u64 k32_SetEvent(u64 h) {
    struct object *o = object_of(h);
    if (o && o->kind == SHARED_EVENT) w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_EVENT_SET, (u64)o->count, 0, 0, 0);
    else must(W2M_API_SetEvent, h, EVENT_MANUAL, EVENT_AUTO)->count = 1;
    w2m_signal();
    return w2m_logged(W2M_API_SetEvent, h, 1);
}
u64 k32_ResetEvent(u64 h) {
    struct object *o = object_of(h);
    if (o && o->kind == SHARED_EVENT) w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_EVENT_RESET, (u64)o->count, 0, 0, 0);
    else must(W2M_API_ResetEvent, h, EVENT_MANUAL, EVENT_AUTO)->count = 0;
    return w2m_logged(W2M_API_ResetEvent, h, 1);
}

u64 k32_ReleaseSemaphore(u64 h, u64 count, u64 previous) {
    struct object *o = must(W2M_API_ReleaseSemaphore, h, SEMAPHORE, SEMAPHORE);
    if ((s32)count <= 0) w2m_unimplemented(W2M_API_ReleaseSemaphore, count);
    if (o->count + (s32)count > o->max) {
        set_error(ERROR_TOO_MANY_POSTS);
        return w2m_logged(W2M_API_ReleaseSemaphore, h, 0);
    }
    if (previous) *(s32 *)previous = (s32)o->count;
    o->count += (s32)count;
    w2m_signal();
    return w2m_logged(W2M_API_ReleaseSemaphore, h, 1);
}

u64 k32_ReleaseMutex(u64 h) {
    struct object *o = must(W2M_API_ReleaseMutex, h, MUTEX, MUTEX);
    if (o->count == 0 || (u32)o->max != current_tid()) {
        set_error(ERROR_NOT_OWNER);
        return w2m_logged(W2M_API_ReleaseMutex, h, 0);
    }
    if (--o->count == 0) {
        o->max = 0;
        w2m_signal();
    }
    return w2m_logged(W2M_API_ReleaseMutex, h, 1);
}

int w2m_thread_ended(u32 tid);        /* threads.c */

/* A thread's handle signals when the thread has ended (rows wait.worker, wait.again; Sprint 051); a mutex when no
 * thread owns it or this one does (`max` holds its owner's id, `count` the recursion). */
static int signaled(struct object *o) {
    if (o->kind == THREAD) return w2m_thread_ended((u32)o->count);
    if (o->kind == PROCESS)                                    /* this process runs; another's is the host's */
        return (u32)o->count != (u32)w2m_teb()->client_process &&
               w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_PROCESS_STATE, (u64)o->count, 0, 0, 0) != 259;
    if (o->kind == SHARED_EVENT) return (int)w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_EVENT_STATE, (u64)o->count, 0, 0, 0);
    if (o->kind == PIPE_READ || o->kind == PIPE_WRITE || o->kind == REMOTE_THREAD) w2m_unimplemented(W2M_API_WaitForSingleObject, o->kind);
    if (o->kind == MUTEX) return o->count == 0 || (u32)o->max == current_tid();
    return o->count > 0;
}

static void take(struct object *o) {
    if (o->kind == MUTEX) {
        o->count++;
        o->max = current_tid();
    } else if (o->kind == SEMAPHORE || o->kind == EVENT_AUTO) {
        o->count--;
    } else if (o->kind == SHARED_EVENT) {
        w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_EVENT_TAKE, (u64)o->count, 0, 0, 0);
    }
}

/* Not signaled: 1 when the wait is over (WAIT_TIMEOUT now); 0 when the thread blocks and retries the call (runtime.h
 * W2M_SCHEDULE_RETRY), other threads running meanwhile. ms is from the first try, the deadline kept in the TEB. */
static int timed_out(u32 ms) {
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    if (ms == 0) return 1;
    if (!sc->wait_deadline) {
        w2m_block(ms == INFINITE ? ~0ull : w2m_counter_now() + (u64)ms * 10000);
        return 0;
    }
    if (sc->wait_deadline != ~0ull && w2m_counter_now() >= sc->wait_deadline) {
        sc->wait_deadline = 0;
        return 1;
    }
    w2m_block(sc->wait_deadline);
    return 0;
}

static u64 wait_one(u64 api, u64 h, u32 ms) {
    struct object *o = object_of(h);
    if (!o) {
        if (h < OBJECT_BASE) w2m_unimplemented(api, h);         /* a file, a process, a pseudo-handle: not in the rows */
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(api, h, WAIT_FAILED);
    }
    if (!signaled(o)) {
        if (!timed_out(ms)) return 0;                          /* retried: the bridge does not return */
        return w2m_logged(api, h, WAIT_TIMEOUT);
    }
    w2m_thread_scratch()->wait_deadline = 0;
    take(o);
    return w2m_logged(api, h, WAIT_OBJECT_0);
}

u64 k32_WaitForSingleObject(u64 h, u64 ms) { return wait_one(W2M_API_WaitForSingleObject, h, (u32)ms); }

u64 k32_WaitForSingleObjectEx(u64 h, u64 ms, u64 alertable) {
    (void)alertable;                                           /* no APC is ever queued: one thread */
    return wait_one(W2M_API_WaitForSingleObjectEx, h, (u32)ms);
}

u64 k32_WaitForMultipleObjects(u64 count, u64 handles, u64 all, u64 ms) {
    const u64 *hs = (const u64 *)handles;
    u32 n = (u32)count;
    if (n == 0 || n > 64) w2m_unimplemented(W2M_API_WaitForMultipleObjects, n);
    struct object *os[64];
    for (u32 i = 0; i < n; i++) {
        os[i] = object_of(hs[i]);
        if (!os[i]) w2m_unimplemented(W2M_API_WaitForMultipleObjects, hs[i]);   /* not in the rows */
    }
    if ((u32)all) {
        for (u32 i = 0; i < n; i++)
            if (!signaled(os[i])) {
                if (!timed_out((u32)ms)) return 0;
                return w2m_logged(W2M_API_WaitForMultipleObjects, n, WAIT_TIMEOUT);
            }
        w2m_thread_scratch()->wait_deadline = 0;
        for (u32 i = 0; i < n; i++) take(os[i]);
        return w2m_logged(W2M_API_WaitForMultipleObjects, n, WAIT_OBJECT_0);
    }
    for (u32 i = 0; i < n; i++)
        if (signaled(os[i])) {
            w2m_thread_scratch()->wait_deadline = 0;
            take(os[i]);
            return w2m_logged(W2M_API_WaitForMultipleObjects, n, WAIT_OBJECT_0 + i);
        }
    if (!timed_out((u32)ms)) return 0;
    return w2m_logged(W2M_API_WaitForMultipleObjects, n, WAIT_TIMEOUT);
}

/* Sleep: threads.c (Sprint 051), which lets other threads run while it waits. */

/* For processes.c (Sprint 055): a new handle on a new object; a handle's kind (0: no object) and `count`; its
 * flags; the inheritable handles; and the handles a parent passed, installed at their values on first use. */
u64 w2m_object_new(u64 api, u32 kind, s64 count, u32 flags) {
    u64 h = handle_for(api, new_object(api, kind, count, 0));
    handle_flags[(h - OBJECT_BASE) / 4] = flags;
    return h;
}
u32 w2m_object_kind(u64 h, s64 *count) {
    struct object *o = object_of(h);
    if (!o) return 0;
    *count = o->count;
    return o->kind;
}
u32 *w2m_object_flags(u64 h) {
    int i = slot_of(h);
    return i < 0 ? 0 : &handle_flags[i];
}
int w2m_object_inheritable(u64 (*out)[3], int room) {
    int n = 0;
    for (int i = 0; i < OBJECTS && n < room; i++)
        if (handle_object[i] && (handle_flags[i] & 1)) {
            struct object *o = &objects[handle_object[i] - 1];
            if (o->kind != SHARED_EVENT && o->kind != PIPE_READ && o->kind != PIPE_WRITE && o->kind != PROCESS)
                w2m_unimplemented(W2M_API_CreateProcessW, o->kind);    /* inheriting such a handle: not in the rows */
            out[n][0] = OBJECT_BASE + 4 * (u64)i;
            out[n][1] = o->kind;
            out[n][2] = (u64)o->count;
            n++;
        }
    return n;
}

static void inherit_init(void) {
    if (inherited_ready) return;
    inherited_ready = 1;
    const u64 *t = (const u64 *)W2M_INHERIT_VA;              /* count, the three standard handles, then entries */
    for (u64 k = 0; k < t[0]; k++) {
        u64 h = t[4 + 3 * k], kind = t[5 + 3 * k], id = t[6 + 3 * k];
        int i = (int)((h - OBJECT_BASE) / 4);
        if (h < OBJECT_BASE || (h - OBJECT_BASE) % 4 || i >= OBJECTS) w2m_unimplemented(W2M_API_CreateProcessW, h);
        int o = new_object(W2M_API_CreateProcessW, (u32)kind, (s64)id, 0);
        handle_object[i] = o + 1;
        handle_flags[i] = 1;                                    /* inherited handles stay inheritable */
        objects[o].refs = 1;
    }
}

/* DuplicateHandle, OpenThread, GetThreadId, GetProcessId, CreateSemaphoreW (Sprint 049, family "thread handles"), as
 * contracts/windows.handle.thread_handles records Windows answering (Server 2025 and 2022):
 *   - the pseudo-handles are -1 (the process) and -2 (the thread); DuplicateHandle of either gives a real handle,
 *     a new one each time, naming the current thread or process (GetThreadId, GetProcessId); a thread's handle
 *     does not signal while it runs
 *   - a duplicate of an event names the same event; with DUPLICATE_CLOSE_SOURCE the copy takes the source's value
 *   - a source that is no handle: FALSE, ERROR_INVALID_HANDLE, *lpTargetHandle NULL; success leaves the last error
 *   - OpenThread(THREAD_ALL_ACCESS, FALSE, the current thread's id): a real handle; an id no thread has: NULL, 87
 *   - CreateSemaphoreW: success sets the last error to 0; an initial count above the maximum: NULL, 87 */
#define CURRENT_PROCESS (~0ull)
#define CURRENT_THREAD (~0ull - 1)
#define DUPLICATE_CLOSE_SOURCE 1u
#define DUPLICATE_SAME_ACCESS 2u
#define ERROR_INVALID_PARAMETER 87u

int w2m_is_file_handle(u64 h);       /* file.c */
int w2m_is_std_handle(u64 h);        /* console.c */

static u32 current_tid(void) { return (u32)w2m_teb()->client_thread; }
static u32 current_pid(void) { return (u32)w2m_teb()->client_process; }

/* For threads.c (Sprint 051): a new handle on thread tid's object, and the thread a handle names. */
u64 w2m_object_thread(u64 api, u32 tid) { return handle_for(api, new_object(api, THREAD, tid, 0)); }
int w2m_object_thread_of(u64 h, u32 *tid) {
    struct object *o = object_of(h);
    if (!o || o->kind != THREAD) return 0;
    *tid = (u32)o->count;
    return 1;
}

/* DuplicateHandle(hSourceProcessHandle, hSourceHandle, hTargetProcessHandle, lpTargetHandle, dwDesiredAccess,
 * bInheritHandle, dwOptions): the last three on the guest's stack at RSP + 0x28, + 0x30, + 0x38. */
u64 k32_DuplicateHandle(u64 source_process, u64 source, u64 target_process, u64 out, u64 rsp) {
    u32 inherit = *(u32 *)(rsp + 0x30), options = *(u32 *)(rsp + 0x38);
    if (source_process != CURRENT_PROCESS || target_process != CURRENT_PROCESS || !out || inherit
        || (options & ~(DUPLICATE_CLOSE_SOURCE | DUPLICATE_SAME_ACCESS)) || !(options & DUPLICATE_SAME_ACCESS))
        w2m_unimplemented(W2M_API_DuplicateHandle, options);
    u64 h;
    if (source == CURRENT_THREAD || source == CURRENT_PROCESS) {
        if (options & DUPLICATE_CLOSE_SOURCE) w2m_unimplemented(W2M_API_DuplicateHandle, source);
        h = source == CURRENT_THREAD ? handle_for(W2M_API_DuplicateHandle, new_object(W2M_API_DuplicateHandle, THREAD, current_tid(), 0))
                                     : handle_for(W2M_API_DuplicateHandle, new_object(W2M_API_DuplicateHandle, PROCESS, current_pid(), 0));
    } else {
        int i = slot_of(source);
        if (i < 0) {
            if (w2m_is_file_handle(source) || w2m_is_std_handle(source))   /* a file's or the console's: not in the rows */
                w2m_unimplemented(W2M_API_DuplicateHandle, source);
            *(u64 *)out = 0;
            set_error(ERROR_INVALID_HANDLE);
            return w2m_logged(W2M_API_DuplicateHandle, source, 0);
        }
        if (options & DUPLICATE_CLOSE_SOURCE) h = source;          /* the copy keeps the slot (row dup.close_source) */
        else h = handle_for(W2M_API_DuplicateHandle, handle_object[i] - 1);
    }
    *(u64 *)out = h;
    return w2m_logged(W2M_API_DuplicateHandle, source, 1);
}

/* OpenThread(dwDesiredAccess, bInheritHandle, dwThreadId): one thread runs, so only its id names a thread. */
u64 k32_OpenThread(u64 access, u64 inherit, u64 tid) {
    (void)access;
    if ((u32)inherit) w2m_unimplemented(W2M_API_OpenThread, inherit);
    if ((u32)tid != current_tid()) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_OpenThread, tid, 0);
    }
    return w2m_logged(W2M_API_OpenThread, tid, handle_for(W2M_API_OpenThread, new_object(W2M_API_OpenThread, THREAD, current_tid(), 0)));
}

u64 k32_GetThreadId(u64 h) {
    if (h == CURRENT_THREAD) return w2m_logged(W2M_API_GetThreadId, h, current_tid());
    struct object *o = object_of(h);
    if (!o || o->kind != THREAD) w2m_unimplemented(W2M_API_GetThreadId, h);           /* not in the rows */
    return w2m_logged(W2M_API_GetThreadId, h, (u64)o->count);
}

u64 k32_GetProcessId(u64 h) {
    if (h == CURRENT_PROCESS) return w2m_logged(W2M_API_GetProcessId, h, current_pid());
    struct object *o = object_of(h);
    if (!o || o->kind != PROCESS) w2m_unimplemented(W2M_API_GetProcessId, h);         /* not in the rows */
    return w2m_logged(W2M_API_GetProcessId, h, (u64)o->count);
}

u64 k32_CreateSemaphoreW(u64 attributes, u64 initial, u64 maximum, u64 name) {
    if (attributes || name || (s32)maximum <= 0 || (s32)initial < 0) w2m_unimplemented(W2M_API_CreateSemaphoreW, name);
    if ((s32)initial > (s32)maximum) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateSemaphoreW, initial, 0);
    }
    return make(W2M_API_CreateSemaphoreW, SEMAPHORE, (s32)initial, (s32)maximum);
}

