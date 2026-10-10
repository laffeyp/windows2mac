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
#define CREATE_EVENT_MANUAL_RESET 1u
#define CREATE_EVENT_INITIAL_SET 2u

enum kind { FREE, SEMAPHORE, EVENT_MANUAL, EVENT_AUTO, MUTEX };

static struct object { u32 kind; s64 count, max; } objects[OBJECTS];   /* count: semaphore count, event 0/1, mutex recursion */

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

static u64 make(u64 api, u32 kind, s64 count, s64 max) {
    for (int i = 0; i < OBJECTS; i++)
        if (objects[i].kind == FREE) {
            objects[i] = (struct object){kind, count, max};
            set_error(0);                              /* rows *.create: err 0 */
            return w2m_logged(api, kind, OBJECT_BASE + 4 * (u64)i);
        }
    w2m_unimplemented(api, OBJECTS);
    return 0;
}

static struct object *object_of(u64 h) {
    if (h < OBJECT_BASE || (h - OBJECT_BASE) % 4 || (h - OBJECT_BASE) / 4 >= OBJECTS) return 0;
    struct object *o = &objects[(h - OBJECT_BASE) / 4];
    return o->kind == FREE ? 0 : o;
}

/* file.c's CloseHandle, for a handle that is not a file's: 1 when it named an object. */
int w2m_object_close(u64 h) {
    struct object *o = object_of(h);
    if (!o) return 0;
    o->kind = FREE;
    return 1;
}

u64 k32_CreateSemaphoreExW(u64 attributes, u64 initial, u64 maximum, u64 name, u64 rsp) {
    u32 flags = *(u32 *)(rsp + 0x28);
    if (attributes || name || flags || (s32)maximum <= 0 || (s32)initial < 0 || (s32)initial > (s32)maximum)
        w2m_unimplemented(W2M_API_CreateSemaphoreExW, name);
    return make(W2M_API_CreateSemaphoreExW, SEMAPHORE, (s32)initial, (s32)maximum);
}

static u64 create_event(u64 api, u64 attributes, u64 manual, u64 initial, u64 name) {
    if (attributes || name) w2m_unimplemented(api, name);
    return make(api, manual ? EVENT_MANUAL : EVENT_AUTO, initial ? 1 : 0, 1);
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

u64 k32_SetEvent(u64 h) { must(W2M_API_SetEvent, h, EVENT_MANUAL, EVENT_AUTO)->count = 1; return w2m_logged(W2M_API_SetEvent, h, 1); }
u64 k32_ResetEvent(u64 h) { must(W2M_API_ResetEvent, h, EVENT_MANUAL, EVENT_AUTO)->count = 0; return w2m_logged(W2M_API_ResetEvent, h, 1); }

u64 k32_ReleaseSemaphore(u64 h, u64 count, u64 previous) {
    struct object *o = must(W2M_API_ReleaseSemaphore, h, SEMAPHORE, SEMAPHORE);
    if ((s32)count <= 0) w2m_unimplemented(W2M_API_ReleaseSemaphore, count);
    if (o->count + (s32)count > o->max) {
        set_error(ERROR_TOO_MANY_POSTS);
        return w2m_logged(W2M_API_ReleaseSemaphore, h, 0);
    }
    if (previous) *(s32 *)previous = (s32)o->count;
    o->count += (s32)count;
    return w2m_logged(W2M_API_ReleaseSemaphore, h, 1);
}

u64 k32_ReleaseMutex(u64 h) {
    struct object *o = must(W2M_API_ReleaseMutex, h, MUTEX, MUTEX);
    if (o->count == 0) {
        set_error(ERROR_NOT_OWNER);
        return w2m_logged(W2M_API_ReleaseMutex, h, 0);
    }
    o->count--;
    return w2m_logged(W2M_API_ReleaseMutex, h, 1);
}

static int signaled(struct object *o) { return o->kind == MUTEX || o->count > 0; }   /* one thread owns every mutex */

static void take(struct object *o) {
    if (o->kind == MUTEX) o->count++;
    else if (o->kind != EVENT_MANUAL) o->count--;
}

static u64 timeout(u64 api, u32 ms) {
    if (ms == INFINITE) w2m_unimplemented(api, ms);            /* no other thread: it would never return */
    return WAIT_TIMEOUT;
}

static u64 wait_one(u64 api, u64 h, u32 ms) {
    struct object *o = object_of(h);
    if (!o) {
        if (h < OBJECT_BASE) w2m_unimplemented(api, h);         /* a file, a process, a pseudo-handle: not in the rows */
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(api, h, WAIT_FAILED);
    }
    if (!signaled(o)) return w2m_logged(api, h, timeout(api, ms));
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
            if (!signaled(os[i])) return w2m_logged(W2M_API_WaitForMultipleObjects, n, timeout(W2M_API_WaitForMultipleObjects, (u32)ms));
        for (u32 i = 0; i < n; i++) take(os[i]);
        return w2m_logged(W2M_API_WaitForMultipleObjects, n, WAIT_OBJECT_0);
    }
    for (u32 i = 0; i < n; i++)
        if (signaled(os[i])) {
            take(os[i]);
            return w2m_logged(W2M_API_WaitForMultipleObjects, n, WAIT_OBJECT_0 + i);
        }
    return w2m_logged(W2M_API_WaitForMultipleObjects, n, timeout(W2M_API_WaitForMultipleObjects, (u32)ms));
}

/* Sleep: the time passes on the guest counter (10 MHz); one thread, so nothing else runs meanwhile. */
u64 k32_Sleep(u64 ms) {
    u64 until = w2m_counter_now() + (u64)(u32)ms * 10000;
    while (w2m_counter_now() < until) {}
    return w2m_logged(W2M_API_Sleep, (u32)ms, 0);
}
