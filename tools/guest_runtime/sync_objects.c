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
#define WAIT_ABANDONED 0x80u
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
            PIPE_WRITE = W2M_OBJECT_PIPE_WRITE, REMOTE_THREAD = W2M_OBJECT_REMOTE_THREAD, TIMER = 13 };
            /* Sprint 055 (processes.c): a named event, a pipe's ends and another process's thread, each the host's
             * object (src/w2m/kernel.py) whose id `count` holds; a PROCESS's `count` is its id, this one or another.
             * TIMER (Sprint 370): a waitable timer, this file's, `count` its signaled state (0 or 1), `max` 1 when it
             * is a manual-reset timer and 0 when a synchronization timer */

/* Objects, and the handles that name them (Sprint 049: DuplicateHandle gives a second handle to one object, which
 * lives until its last handle closes). count: semaphore count, event 0/1, mutex recursion, a thread's id, a file
 * mapping's host descriptor (its own; closed with the mapping); max: a mapping's size. refs: its handles and the
 * blocked waits' entries that name it (Sprint 350), and each lookup in flight (`object_of`, Sprint 352), every blocked
 * wait of any kind holding an entry, so an object outlives its last handle while a waiter holds it, as
 * Windows keeps one while a pointer reference holds it ("Such objects can still be accessed by pointer as long as their
 * reference count remains greater than zero", Microsoft Learn, Life Cycle of an Object): before, CloseHandle freed it
 * under a waiter's entry, the next object took its index, and that object's first release went to the stale entry
 * (probes/wait_closed_handle). handles: its handles alone, as Windows counts an object's handles apart from its
 * references ("Each routine that opens a handle increases both the object reference count and the object handle
 * count by one", the same page); the host's closes come when they reach 0, at CloseHandle of the last, as before Sprint
 * 350 (reviewer-4's nineteenth review, Medium 1), so another process sees a shared object close when it did. */
static struct object { u32 kind; s64 count, max; u32 refs, handles, abandoned; u64 name; u64 due; u32 period, active; }
    objects[OBJECTS];
/* due, period, active (Sprint 370): a set timer's next expiry on the 10 MHz counter the waits read, its period in ms (0:
 * signaled once), and whether it is set; `sync_lock`'s, as a timer's `count` is. A timer's state is brought to a time
 * only when a wait or SetWaitableTimer reads it (`timer_run`): the counter is read before `sync_lock` is taken, since on
 * the host's clock it is a host request, which a leaf does not make. */
/* name (Sprint 369): a named mutex's or semaphore's id in the host's namespace (kernel.rs NAMED_OPEN), 0 when unnamed;
 * written before the object's first handle is published and fixed after, as `kind` is. */
/* abandoned (Sprint 356's amendment): a mutex whose owner thread ended holding it (`w2m_mutexes_abandoned`), cleared by
 * the first wait that takes it, which answers WAIT_ABANDONED (or WAIT_ABANDONED_0 + its index) once; Microsoft Learn,
 * WaitForSingleObject: "The specified object is a mutex object that was not released by the thread that owned the
 * mutex object before the owning thread terminated. Ownership of the mutex object is granted to the calling thread and
 * the mutex state is set to nonsignaled." `sync_lock`'s, as `count` and `max`. */
static int handle_object[OBJECTS];              /* handle slot -> object index + 1; 0: the slot is free */
static u32 handle_flags[OBJECTS];               /* HANDLE_FLAG_INHERIT, HANDLE_FLAG_PROTECT_FROM_CLOSE (Sprint 055) */
static int inherited_ready;

/* The handle table's own lock (Sprint 355; card 4 of docs/design-handles-and-sync-objects-2026-11-26.md, "Which lock
 * guards which state"): 0 free, 1 held, in the image's zeroed data. It guards the slots (`handle_object`), their flags
 * (`handle_flags`), the one-time install of a parent's handles (`inherited_ready`, and the inherit table it reads), the
 * allocation of an entry of `objects` (`new_object`), a FREE entry's `kind`, and every entry's `refs` and `handles`.
 * An object's `kind`, `count` and `max` are written before its handle is published under the lock and are read with no
 * lock by a holder of a reference; a semaphore's, an event's and a mutex's `count` and `max` change after, under the
 * sync objects' lock (`sync_lock`, Sprint 356). CreateSemaphoreExW, CreateSemaphoreW, CreateMutexA, CreateEventW,
 * CreateEventA, CreateEventExW, DuplicateHandle, OpenThread, GetThreadId and GetProcessId take no runtime word
 * (`services.rs` `HANDLES_LOCK`), nor do the waits and releases (`SYNC_LOCK`, Sprint 356), which never take it with
 * `sync_lock` held; every other caller takes the lock inside the word. It is a leaf: held, it takes no other lock,
 * makes no host request, readies no thread and calls neither w2m_unimplemented nor w2m_logged, so it never waits on
 * the runtime word, whose holder may want it. */
static u32 handles_lock;

/* The table held: compare-and-swap of 0 for 1, acquire, as thread_storage.c's `hold_slots` (CASA), a waiter reading the
 * word by plain loads until it reads 0 before it tries the swap. Nothing reenters it. Its answer, 1, is its guard's
 * (W2M_HELD, runtime.h; Sprint 359). */
static int hold_handles(void) {
#ifdef W2M_TIME_LOCKS
    w2m_lock_take(&handles_lock, W2M_LOCK_HANDLES);      /* timed (Sprint 358, runtime.h) */
#else
    for (;;) {
        while (__atomic_load_n(&handles_lock, __ATOMIC_RELAXED)) {}
        u32 free = 0;
        if (__atomic_compare_exchange_n(&handles_lock, &free, 1, 0, __ATOMIC_ACQUIRE, __ATOMIC_RELAXED)) break;
    }
#endif
    return 1;
}

/* The table let go: a store of 0, release (STLR), as `let_slots_go`. */
#ifdef W2M_TIME_LOCKS
static void let_handles_go(void) { w2m_lock_let_go(&handles_lock, W2M_LOCK_HANDLES); }
#else
static void let_handles_go(void) { __atomic_store_n(&handles_lock, 0, __ATOMIC_RELEASE); }
#endif

/* The table's guard's end (W2M_HELD, runtime.h; Sprint 359): the lock let go unless let go early. */
static void handles_guard_end(int *held) {
    if (*held) let_handles_go();
}

/* The runtime word, taken by a call whose bridge holds none (runtime.c, Sprint 355; runtime.h declares it, with its
 * guard W2M_HELD(runtime_word)): a named event's EVENT_OPEN and DuplicateHandle's look at the file table and the
 * standard handles take it so, with no other lock held, and the waits and releases to park, to ready and for the host's
 * objects (Sprint 356). */

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }
static u32 current_tid(void);

static void inherit_init(void);

/* A kind whose state this file keeps (Sprint 356): a semaphore's, an event's and a mutex's, which change under
 * `sync_lock`, and a token's and a mapping's, fixed at creation (`signaled` reads a mapping's descriptor as a count, as
 * before). The rest are the host's (a thread, a process, a shared event) or answer no wait (a pipe's ends, another
 * process's thread). */
static int sync_kind(const struct object *o) {
    return o->kind == SEMAPHORE || o->kind == EVENT_MANUAL || o->kind == EVENT_AUTO || o->kind == MUTEX ||
           o->kind == TOKEN || o->kind == MAPPING || o->kind == TIMER;
}

/* A kind whose `count` and `max` change after creation, under `sync_lock`: a semaphore, an event, a mutex, a timer's
 * `count` (Sprint 370). Every other kind's are fixed, a mapping's descriptor and size among them, and read with no lock
 * by a holder of a reference. */
static int changes(const struct object *o) {
    return o->kind == SEMAPHORE || o->kind == EVENT_MANUAL || o->kind == EVENT_AUTO || o->kind == MUTEX ||
           o->kind == TIMER;
}

/* A free slot taken for entry `object`, its counts raised: the handle; 0 when every slot is taken. The table held. */
static u64 handle_for(int object) {
    inherit_init();
    for (int i = 0; i < OBJECTS; i++)
        if (!handle_object[i]) {
            handle_object[i] = object + 1;
            handle_flags[i] = 0;
            objects[object].refs++;
            objects[object].handles++;
            return OBJECT_BASE + 4 * (u64)i;
        }
    return 0;
}

/* The first FREE entry made an object: its index; -1 when none is FREE. The table held. */
static int new_object(u32 kind, s64 count, s64 max) {
    for (int i = 0; i < OBJECTS; i++)
        if (objects[i].kind == FREE) {
            objects[i] = (struct object){kind, count, max, 0, 0};
            return i;
        }
    return -1;
}

/* A new object and a handle naming it, its slot's flags `flags`, under the table's lock, so the object is whole before
 * its handle is published and another vCPU's lookup of the handle reads it whole. A full table or a full slot list
 * lets the lock go, then stops the run. */
static u64 publish(u64 api, u32 kind, s64 count, s64 max, u32 flags) {
    u64 h;
    {
        W2M_HELD(handles);
        int o = new_object(kind, count, max);
        h = o < 0 ? 0 : handle_for(o);
        if (h) handle_flags[(h - OBJECT_BASE) / 4] = flags;
        else if (o >= 0) objects[o].kind = FREE;
    }
    if (!h) w2m_unimplemented(api, OBJECTS);
    return h;
}

static u64 make(u64 api, u32 kind, s64 count, s64 max) {
    u64 h = publish(api, kind, count, max, 0);
    set_error(0);                                  /* rows *.create: err 0 */
    return w2m_logged(api, kind, h);
}

/* The slot h names, -1 when none. The table held. */
static int slot_of(u64 h) {
    inherit_init();
    if (h < OBJECT_BASE || (h - OBJECT_BASE) % 4 || (h - OBJECT_BASE) / 4 >= OBJECTS) return -1;
    int i = (int)((h - OBJECT_BASE) / 4);
    return handle_object[i] ? i : -1;
}

/* One reference to `o` dropped, a handle's, a lookup's or a waiter entry's; at none its index is free. No host request:
 * the host's closes are the last handle's (w2m_object_close). The table held. */
static void unref(struct object *o) {
    if (--o->refs == 0) o->kind = FREE;
}

/* The object h names, holding a reference the caller drops by `object_done` when it is done with the object (Sprint
 * 352, card 2 of docs/design-handles-and-sync-objects-2026-11-26.md), as an NT lookup holds one: "Incrementing this
 * count prevents the object from being deleted while the pointer is being referenced. The caller must decrement the
 * reference count with ObDereferenceObject as soon as it is done with the object" (Microsoft Learn,
 * ObReferenceObjectByHandle). A wait that blocks keeps its lookups' references in its entry instead. 0: no object.
 * The reference is taken, and dropped, under the table's lock (Sprint 355), as ReactOS takes it with the handle entry
 * locked (ob/obref.c 633-672). */
static struct object *object_of(u64 h) {
    W2M_HELD(handles);
    int i = slot_of(h);
    struct object *o = i < 0 ? 0 : &objects[handle_object[i] - 1];
    if (o) o->refs++;
    return o;
}

static void object_done(struct object *o) {
    if (!o) return;
    W2M_HELD(handles);
    unref(o);
}

/* For token.c: a process token's handle, from this table so CloseHandle closes it; and whether h is one. The
 * last error is left (row open). Each accessor below answers a value, never an object, and drops its lookup's
 * reference before it returns, so file.c, processes.c, token.c and threads.c hold none. */
u64 w2m_object_token(u64 api) { return publish(api, TOKEN, 0, 0, 0); }
int w2m_object_is_token(u64 h) {
    struct object *o = object_of(h);
    int token = o && o->kind == TOKEN;
    object_done(o);
    return token;
}
int w2m_object_is_open(u64 h) {                                     /* file.c's handle calls (Sprint 049) */
    struct object *o = object_of(h);
    object_done(o);
    return o != 0;
}

/* For file.c's CreateFileMappingW and MapViewOfFile (Sprint 049): a mapping's handle, holding host descriptor fd
 * and the mapping's size; and a mapping's descriptor and size by handle (0 when h names no mapping). */
u64 w2m_object_mapping(u64 api, u64 fd, u64 size) { return publish(api, MAPPING, (s64)fd, (s64)size, 0); }
int w2m_object_mapping_of(u64 h, u64 *fd, u64 *size) {
    struct object *o = object_of(h);
    int mapping = o && o->kind == MAPPING;
    if (mapping) {
        *fd = (u64)o->count;
        *size = (u64)o->max;
    }
    object_done(o);
    return mapping;
}

/* file.c's CloseHandle, for a handle that is not a file's: 1 when it named an object. The last handle makes the
 * host's closes, though a wait's entry may still hold the object. The slot is freed and both counts dropped under the
 * table's lock; the host's closes are made after it is let go (a leaf makes no host request), from the kind and id
 * read under it, since once the reference is dropped another vCPU may make the entry a new object (Sprint 355). */
int w2m_object_close(u64 h) {
    int last;
    u32 kind;
    s64 id;
    u64 name;
    {
        W2M_HELD(handles);
        int i = slot_of(h);
        if (i < 0) return 0;
        struct object *o = &objects[handle_object[i] - 1];
        handle_object[i] = 0;
        last = --o->handles == 0;
        kind = o->kind;
        id = changes(o) ? 0 : o->count;       /* a semaphore's, an event's, a mutex's: `sync_lock`'s, and unused here */
        name = o->name;
        unref(o);
    }
    if (last) {
        if (kind == MAPPING) w2m_host(W2M_FILE_CLOSE, (u64)id, 0, 0, 0, 0);       /* the views keep their copy */
        if (kind == SHARED_EVENT || kind == PIPE_READ || kind == PIPE_WRITE ||
            (kind == PROCESS && (u32)id != (u32)w2m_teb()->client_process))
            w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_CLOSE, (u64)id, 0, 0, 0);   /* the host's handle count */
        if (name)                         /* a named mutex's or semaphore's name goes with its last handle (Sprint 369) */
            w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_CLOSE, name, 0, 0, 0);
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
 * that exists answers a new handle to the same event and ERROR_ALREADY_EXISTS; a new one sets the last error 0). A
 * name a named mutex or semaphore holds answers NULL and ERROR_INVALID_HANDLE (Sprint 369): "If lpName matches the
 * name of another kind of object in the same namespace (such as an existing semaphore, mutex, waitable timer, job, or
 * file-mapping object), the function fails and the GetLastError function returns ERROR_INVALID_HANDLE" (Microsoft
 * Learn, CreateEventW). */
static u64 create_event(u64 api, u64 attributes, u64 manual, u64 initial, u64 name) {
    if (attributes) w2m_unimplemented(api, attributes);
    if (!name) return make(api, manual ? EVENT_MANUAL : EVENT_AUTO, initial ? 1 : 0, 1);
    if (api != W2M_API_CreateEventW && api != W2M_API_CreateEventExW) w2m_unimplemented(api, name);   /* ANSI names */
    u64 chars = 0;
    while (((const unsigned short *)name)[chars]) chars++;
    u64 existed = 0, id;
    {
        W2M_HELD(runtime_word);                     /* the transport page is every vCPU's; no other lock held */
        id = w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_EVENT_OPEN, name, chars, (manual ? 1 : 0) | (initial ? 2 : 0), &existed);
    }
    if (existed == W2M_NAME_OTHER_KIND) {          /* a named mutex's or semaphore's (Sprint 369): CreateEventW's page */
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(api, existed, 0);
    }
    u64 h = publish(api, SHARED_EVENT, (s64)id, 0, 0);
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

/* The named sync objects the game declares and the Windows batch run never called (Sprint 369;
 * docs/unserved-declared-imports-2026-11-27.md): CreateMutexW, OpenMutexW, OpenEventW, OpenEventA and OpenSemaphoreW,
 * held to probes/named_sync_objects' Windows rows (contracts/windows.sync.named_objects). A name is the host's, in the
 * one namespace the named events share (kernel.rs NAMED_OPEN), since "these objects share the same namespace"
 * (Microsoft Learn, CreateMutexW): a name of another kind answers NULL and ERROR_INVALID_HANDLE, and an open of a name
 * nothing holds NULL and ERROR_FILE_NOT_FOUND ("If a named mutex does not exist, the function fails and GetLastError
 * returns ERROR_FILE_NOT_FOUND", OpenMutexW). A named event's state is the host's, each handle an object of this file's
 * naming the host's id, as CreateEventW's. A named mutex's or semaphore's state is this file's: its one entry of
 * `objects` names the host's id (`name`), every handle made or opened by the name names that entry, and its last
 * handle's close frees the name ("The mutex object is destroyed when its last handle has been closed"); another
 * process's open of it stops the run by name (the host's). An opened handle has every right its kind has: the rows
 * show no call refused for want of one, and no right narrower than the kind's all-access is kept. */
#define ERROR_FILE_NOT_FOUND 2u
#define NAME_CHARS 260                         /* "The name is limited to MAX_PATH characters" (CreateMutexW) */
#define SYNCHRONIZE 0x00100000u
#define MUTEX_ALL_ACCESS 0x1F0001u             /* Synchronization Object Security and Access Rights */
#define EVENT_ALL_ACCESS 0x1F0003u
#define SEMAPHORE_ALL_ACCESS 0x1F0003u

/* A name's UTF-16 units before its terminator; a longer one than MAX_PATH stops the run. */
static u64 name_chars(u64 api, const unsigned short *name) {
    u64 chars = 0;
    while (name[chars])
        if (++chars > NAME_CHARS) w2m_unimplemented(api, chars);
    return chars;
}

/* A named mutex or semaphore made (`create`), or found by its name: its handle; 0 with the last error set when the
 * name is absent from an open or another kind's. A make sets the last error, ERROR_ALREADY_EXISTS when the name was
 * the kind's already (its count, maximum and owner then the maker's: "the bInitialOwner parameter is ignored because
 * it has already been set by the creating process", CreateMutexW; "the lInitialCount and lMaximumCount parameters are
 * ignored", CreateSemaphoreW), else 0; an open that finds it leaves the last error. The runtime word is held from the
 * host's request to the handle's publication, so no other vCPU's make, open or last close of the name, each the
 * word's (CloseHandle's bridge holds it), falls between them; the table's lock taken under it, a leaf. */
static u64 named(u64 api, u32 kind, u64 name, int create, s64 count, s64 max) {
    u64 chars = name_chars(api, (const unsigned short *)name), second = 0, h = 0;
    int found = 1;
    W2M_HELD(runtime_word);
    u64 host_kind = kind == MUTEX ? W2M_NAMED_MUTEX : kind == TIMER ? W2M_NAMED_TIMER : W2M_NAMED_SEMAPHORE;
    u64 id = w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_NAMED_OPEN, name, chars, host_kind | (create ? W2M_NAMED_CREATE : 0),
                      &second);
    if (second == W2M_NAME_ABSENT || second == W2M_NAME_OTHER_KIND) {
        set_error(second == W2M_NAME_ABSENT ? ERROR_FILE_NOT_FOUND : ERROR_INVALID_HANDLE);
        return 0;
    }
    {
        W2M_HELD(handles);
        int o = -1;
        if (second == W2M_NAME_CREATED) {
            o = new_object(kind, count, max);
            if (o >= 0) objects[o].name = id;           /* before its handle is published */
        } else {
            for (int i = 0; i < OBJECTS && o < 0; i++)
                if (objects[i].kind == kind && objects[i].handles && objects[i].name == id) o = i;
            found = o >= 0;
        }
        h = o < 0 ? 0 : handle_for(o);
        if (!h && o >= 0 && second == W2M_NAME_CREATED) objects[o].kind = FREE;
    }
    if (!found) w2m_unimplemented(api, id);             /* the host's name and no object of this file's naming it */
    if (!h) w2m_unimplemented(api, OBJECTS);
    if (create) set_error(second == W2M_NAME_EXISTED ? ERROR_ALREADY_EXISTS : 0);
    return h;
}

/* CreateMutexW(lpMutexAttributes, bInitialOwner, lpName): "If this value is TRUE and the caller created the mutex, the
 * calling thread obtains initial ownership of the mutex object" (Microsoft Learn), a recursion of 1 and its owner. */
u64 k32_CreateMutexW(u64 attributes, u64 owned, u64 name) {
    if (attributes) w2m_unimplemented(W2M_API_CreateMutexW, attributes);
    s64 count = (u32)owned ? 1 : 0, owner = (u32)owned ? current_tid() : 0;
    if (!name) return make(W2M_API_CreateMutexW, MUTEX, count, owner);
    return w2m_logged(W2M_API_CreateMutexW, MUTEX, named(W2M_API_CreateMutexW, MUTEX, name, 1, count, owner));
}

/* An open's access, inheritance and name as the rows hold them: a mask within the kind's all-access (one wider, or
 * MAXIMUM_ALLOWED or a generic right, stops the run), no inheritance, a name. */
static void open_form(u64 api, u64 access, u64 inherit, u64 name, u32 all) {
    if ((u32)access & ~all) w2m_unimplemented(api, access);
    if ((u32)inherit || !name) w2m_unimplemented(api, inherit);
}

/* OpenMutexW(dwDesiredAccess, bInheritHandle, lpName): "Only the SYNCHRONIZE access right is required to use a
 * mutex" (Microsoft Learn). */
u64 k32_OpenMutexW(u64 access, u64 inherit, u64 name) {
    open_form(W2M_API_OpenMutexW, access, inherit, name, MUTEX_ALL_ACCESS);
    return w2m_logged(W2M_API_OpenMutexW, access, named(W2M_API_OpenMutexW, MUTEX, name, 0, 0, 0));
}

/* OpenSemaphoreW(dwDesiredAccess, bInheritHandle, lpName): "The function succeeds only if some process has already
 * created the semaphore by using the CreateSemaphore function" (Microsoft Learn). */
u64 k32_OpenSemaphoreW(u64 access, u64 inherit, u64 name) {
    open_form(W2M_API_OpenSemaphoreW, access, inherit, name, SEMAPHORE_ALL_ACCESS);
    return w2m_logged(W2M_API_OpenSemaphoreW, access, named(W2M_API_OpenSemaphoreW, SEMAPHORE, name, 0, 0, 0));
}

/* A named event opened by its UTF-16 name: the host's event, a new object of this file's naming its id, as a
 * CreateEventW that finds its name; absent or another kind's, NULL and the open's error. */
static u64 open_event(u64 api, u64 access, const unsigned short *name) {
    u64 chars = name_chars(api, name), second = 0, id;
    {
        W2M_HELD(runtime_word);                     /* the transport page is every vCPU's; no other lock held */
        id = w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_NAMED_OPEN, (u64)name, chars, W2M_NAMED_EVENT, &second);
    }
    if (second != W2M_NAME_EXISTED) {
        set_error(second == W2M_NAME_ABSENT ? ERROR_FILE_NOT_FOUND : ERROR_INVALID_HANDLE);
        return w2m_logged(api, access, 0);
    }
    return w2m_logged(api, access, publish(api, SHARED_EVENT, (s64)id, 0, 0));
}

/* OpenEventW(dwDesiredAccess, bInheritHandle, lpName): "The function succeeds only if some process has already
 * created the event by using the CreateEvent function" (Microsoft Learn). */
u64 k32_OpenEventW(u64 access, u64 inherit, u64 name) {
    open_form(W2M_API_OpenEventW, access, inherit, name, EVENT_ALL_ACCESS);
    return open_event(W2M_API_OpenEventW, access, (const unsigned short *)name);
}

/* OpenEventA: the ANSI name widened, byte for unit; a byte past ASCII, which the code page would map, stops the run. */
u64 k32_OpenEventA(u64 access, u64 inherit, u64 name) {
    open_form(W2M_API_OpenEventA, access, inherit, name, EVENT_ALL_ACCESS);
    unsigned short wide[NAME_CHARS + 1];
    const unsigned char *a = (const unsigned char *)name;
    u64 k = 0;
    for (; a[k]; k++) {
        if (k == NAME_CHARS || a[k] >= 0x80) w2m_unimplemented(W2M_API_OpenEventA, k);
        wide[k] = a[k];
    }
    wide[k] = 0;
    return open_event(W2M_API_OpenEventA, access, wide);
}

/* CreateWaitableTimerA(lpTimerAttributes, bManualReset, lpTimerName) (Sprint 370): "If this parameter is TRUE, the timer
 * is a manual-reset notification timer. Otherwise, the timer is a synchronization timer"; "Timers are initially
 * inactive" (Microsoft Learn, CreateWaitableTimerW, which documents the A form: Learn has no page of its own for it, and
 * SetWaitableTimer). Unnamed, `make`'s object, its last error 0 as the other creates set it; named, the name's object,
 * made or found (`named`), "If the named timer object exists before the function call, the function returns a handle to
 * the existing object and GetLastError returns ERROR_ALREADY_EXISTS", its manual-reset flag then the maker's; the ANSI
 * name widened byte for unit as OpenEventA's, a byte past ASCII stopping the run. Held to
 * probes/apcs_and_waitable_timers' rows timer.manual and timer.named. */
u64 k32_CreateWaitableTimerA(u64 attributes, u64 manual, u64 name) {
    if (attributes) w2m_unimplemented(W2M_API_CreateWaitableTimerA, attributes);
    s64 reset = (u32)manual ? 1 : 0;
    if (!name) return make(W2M_API_CreateWaitableTimerA, TIMER, 0, reset);
    unsigned short wide[NAME_CHARS + 1];
    const unsigned char *a = (const unsigned char *)name;
    u64 k = 0;
    for (; a[k]; k++) {
        if (k == NAME_CHARS || a[k] >= 0x80) w2m_unimplemented(W2M_API_CreateWaitableTimerA, k);
        wide[k] = a[k];
    }
    wide[k] = 0;
    return w2m_logged(W2M_API_CreateWaitableTimerA, TIMER,
                      named(W2M_API_CreateWaitableTimerA, TIMER, (u64)wide, 1, 0, reset));
}

/* Threads blocked on one semaphore, auto-reset event or mutex, in the order they began to wait. A release grants the
 * object to them at once, as Windows does (probes/wait_handoff, runs 38304235237 and 38304238413): the releaser's
 * own zero-timeout wait right after finds it taken (rows *.one_waiter: releaser_wait 258), two units go to the first
 * two of three waiters (semaphore.three_waiters_two_released) and of two waiters the first wakes (semaphore.order).
 * A granted waiter's retried wait returns WAIT_OBJECT_0 without taking the object again. Before this, the running
 * thread took each unit ahead of the waiters it woke: the game's Background Job.Worker 1-15 never returned from
 * their wait (frontier 643a6283510c-1791830368398119000; game run 38216081440 seq 1402-1508). A thread waiting on any
 * of several objects is a waiter on each (from Sprint 352 on every one, not only those that hand over, so its retry
 * reads them all from its entries), `slot` its index in the wait's array (-1 for a wait on one):
 * a release grants it the object unless another of its entries was granted first, and its retried wait returns that
 * slot (laptop run lap-20261014T094031671Z-93f645c3, row event_auto.multiple_waiter: the waiter gets the second event,
 * the setter's zero-timeout wait finds it taken). A wait for all of several re-checks as before (no row). */
#define WAITERS 256
static struct waiter { u32 tid; int object; int granted; int slot; } waiters[WAITERS];
static int nwaiters;

/* The sync objects' own lock (Sprint 356; card 5 of docs/design-handles-and-sync-objects-2026-11-26.md, "Which lock
 * guards which state"): 0 free, 1 held, in the image's zeroed data. It guards a semaphore's, an event's and a mutex's
 * `count` and `max`, the waiters on one object (`waiters`, `nwaiters`) and the waits for all (`all_waiters`, `nall`).
 * SetEvent, ResetEvent, ReleaseSemaphore, ReleaseMutex, WaitForSingleObject(Ex) and WaitForMultipleObjects(Ex) take no
 * runtime word from their bridges (`services.rs` `SYNC_LOCK`): each looks its handles up under the table's lock and
 * lets it go, checks or changes the state under this lock and lets it go, and takes the word itself to park
 * (`w2m_block`), to ready (`w2m_ready_thread`, `w2m_signal`) and for the host's objects, with no leaf held; references
 * are dropped under the table's lock once this one is let go (the design, "The order, and the two leaves"). A leaf, as
 * the table's: held, it takes no other lock, the table's included, makes no host request, readies no thread and calls
 * neither w2m_unimplemented nor w2m_logged. One lock for every object, not one each: `grant` scans every waiter, a wait
 * on several reads its other entries (`has_grant`), and a release takes for a wait for all objects not its own. */
static u32 sync_lock;

/* The state held, in the form of `hold_handles`: compare-and-swap of 0 for 1, acquire (CASA), plain loads while
 * another vCPU holds it. Nothing reenters it. Its answer, 1, is its guard's (W2M_HELD, runtime.h; Sprint 359). */
static int hold_sync(void) {
#ifdef W2M_TIME_LOCKS
    w2m_lock_take(&sync_lock, W2M_LOCK_SYNC_OBJECTS);    /* timed (Sprint 358, runtime.h) */
#else
    for (;;) {
        while (__atomic_load_n(&sync_lock, __ATOMIC_RELAXED)) {}
        u32 free = 0;
        if (__atomic_compare_exchange_n(&sync_lock, &free, 1, 0, __ATOMIC_ACQUIRE, __ATOMIC_RELAXED)) break;
    }
#endif
    return 1;
}

/* The state let go: a store of 0, release (STLR). */
#ifdef W2M_TIME_LOCKS
static void let_sync_go(void) { w2m_lock_let_go(&sync_lock, W2M_LOCK_SYNC_OBJECTS); }
#else
static void let_sync_go(void) { __atomic_store_n(&sync_lock, 0, __ATOMIC_RELEASE); }
#endif

/* The state's guard's end (W2M_HELD, runtime.h; Sprint 359): the lock let go unless let go early. */
static void sync_guard_end(int *held) {
    if (*held) let_sync_go();
}

/* The runtime word as a release holds it when it may need it, taken with no leaf held, at the first `take_word`:
 * `held` while this call holds it, `taken` when this call took it (a traced call's bridge holds it already, as does a
 * run on the host's clock, `services.rs` `lock_in`, and this call then keeps it). WORD_WHEN_NEEDED declares one as a
 * guard (Sprint 359): clang's `cleanup` attribute, as W2M_HELD's (runtime.h), calls `let_word_go` at the scope's end,
 * which lets the word go when this call took it. */
struct word { int held, taken; };
static void take_word(struct word *w) {
    if (w->held) return;
    w->taken = w2m_take_runtime_word();
    w->held = 1;
}
static void let_word_go(struct word *w) {
    if (!w->held) return;
    w2m_let_runtime_word_go(w->taken);
    w->held = w->taken = 0;
}
#define WORD_WHEN_NEEDED(name) struct word name __attribute__((cleanup(let_word_go))) = {0, 0}

/* threads.c (Sprint 356): the running thread's woken mark armed, at a wait's check under `sync_lock`, so a readying
 * between the check and the park marks it and the park leaves it ready (Sprint 354's Deferred (a)); and the mark set on
 * it, so its park leaves it ready and its call is checked again at once. */
void w2m_wait_armed(void);
void w2m_wait_again(void);

/* The threads a release's grants ready, readied under the runtime word once `sync_lock` is let go: a grant readies at
 * most every waiter on one object, and the waits for all at most every one of them. */
struct readied { u32 n; u32 tids[WAITERS + 16]; };
static void to_ready(struct readied *r, u32 tid) {
    for (u32 i = 0; i < r->n; i++)
        if (r->tids[i] == tid) return;                 /* readied once: a thread waits once (amendment: many mutexes) */
    r->tids[r->n++] = tid;
}
static void ready_all(struct readied *r, struct word *w) {
    if (!r->n) return;
    take_word(w);
    for (u32 i = 0; i < r->n; i++) w2m_ready_thread(r->tids[i]);
}

/* A wait's park (W2M_SCHEDULE_RETRY), the runtime word taken around `w2m_block` alone, `sync_lock` let go. */
static void park(u64 deadline, u64 key) {
    W2M_HELD(runtime_word);
    w2m_block(deadline, key);
}

static int waiter_of(u32 tid) {
    for (int i = 0; i < nwaiters; i++)
        if (waiters[i].tid == tid) return i;
    return -1;
}

/* A waiter entered, holding a reference to its object (Sprint 350): from Sprint 352 its lookup's, which the entry keeps
 * (design, "A wait that blocks keeps its lookups' references in its waiter entry"). Every wait that blocks enters one,
 * of any kind; only a semaphore's, an auto-reset event's and a mutex's are granted. `sync_lock` held. */
static void waiter_add(u32 tid, struct object *o, int slot) {
    waiters[nwaiters++] = (struct waiter){tid, (int)(o - objects), 0, slot};
}

/* Thread tid's entries taken out, `sync_lock` held: its one wait's, one entry for a wait on one object, one an object
 * for a wait on several. Their references are the caller's, which drops them under the table's lock once `sync_lock`
 * is let go (`object_done`, `objects_done`): before Sprint 356 `waiter_drop` and `waiters_drop` dropped them here. */
static void waiters_out(u32 tid) {
    int k = 0;
    for (int i = 0; i < nwaiters; i++)
        if (waiters[i].tid != tid) waiters[k++] = waiters[i];
    nwaiters = k;
}

/* A granted entry's `granted`: 1, or ABANDONED_GRANT when the grant was an abandoned mutex's (Sprint 356's
 * amendment), which its retried wait answers as WAIT_ABANDONED (+ its slot). */
#define ABANDONED_GRANT 2

/* The answer of thread tid's granted entry, a wait on several's, WAIT_OBJECT_0 or WAIT_ABANDONED_0 plus its slot;
 * WAIT_TIMEOUT when none is granted. `sync_lock` held. */
static u64 granted_slot(u32 tid) {
    for (int w = 0; w < nwaiters; w++)
        if (waiters[w].tid == tid && waiters[w].granted)
            return (waiters[w].granted == ABANDONED_GRANT ? WAIT_ABANDONED : WAIT_OBJECT_0) + (u32)waiters[w].slot;
    return WAIT_TIMEOUT;
}

static int has_grant(u32 tid) {
    for (int i = 0; i < nwaiters; i++)
        if (waiters[i].tid == tid && waiters[i].granted) return 1;
    return 0;
}

static int handed_over(struct object *o) {
    return o->kind == SEMAPHORE || o->kind == EVENT_AUTO || o->kind == MUTEX;
}

/* `sync_lock` held; each waiter granted is readied once it is let go (`ready_all`). */
static void grant(struct object *o, struct readied *r) {
    if (!handed_over(o)) return;           /* a manual-reset event's waiters re-check its state (w2m_signal) */
    int index = (int)(o - objects);
    for (int i = 0; i < nwaiters; i++) {
        if (waiters[i].object != index || waiters[i].granted) continue;
        if (waiters[i].slot >= 0 && has_grant(waiters[i].tid)) continue;   /* a wait on several took one already */
        waiters[i].granted = 1;
        if (o->kind == MUTEX) {
            if (o->count) {
                waiters[i].granted = 0;
                return;
            }
            o->count = 1;
            o->max = waiters[i].tid;
            if (o->abandoned) waiters[i].granted = ABANDONED_GRANT;   /* its owner ended holding it (amendment) */
            o->abandoned = 0;
        } else {
            if (o->count <= 0) {
                waiters[i].granted = 0;
                return;
            }
            o->count--;
        }
        to_ready(r, waiters[i].tid);            /* the one waiter the release satisfies (Sprint 095) */
    }
}

/* A wait for all of several objects (Sprint 095; laptop run lap-20261017T040446837Z-dc268176, row
 * wait_all.semaphore_and_event): Windows satisfies it at the release that makes its last object signaled, taking
 * every object for the waiter then, so the releaser's own zero-timeout wait right after finds them taken. A blocked
 * wait-all registers its objects; each release, after its own waiters' grants, takes for the first registered
 * wait-alls whose objects are all signaled for them, and readies those threads. A wait for all that names one of the
 * host's objects (`host`) is never taken for by a release (Sprint 356): its host objects' state is the runtime word's
 * and a host request, which `sync_lock` may not make, so a release of one of its objects that leaves every object of
 * this file's it names signaled for it readies it, and its retried call checks every object again, under the word. */
#define ALL_WAITERS 16
static struct all_waiter { u32 tid, n; int granted, host; int objects[64]; } all_waiters[ALL_WAITERS];
static int nall;

static int all_of(u32 tid) {
    for (int i = 0; i < nall; i++)
        if (all_waiters[i].tid == tid) return i;
    return -1;
}

/* Entry i taken out, `sync_lock` held; its references the caller's, dropped once the lock is let go (before Sprint 356,
 * `all_drop` dropped them here). */
static void all_out(int i) {
    for (; i + 1 < nall; i++) all_waiters[i] = all_waiters[i + 1];
    nall--;
}

/* One of this file's objects signaled for thread tid (a mutex is its owner's: `max` holds its owner's id, `count` the
 * recursion), and taken for it. `sync_lock` held. Before Sprint 356 `signaled` and `take` served every kind under the
 * runtime word, `signaled_for` and `take_for` a thread other than the running one. */
static int sync_signaled(const struct object *o, u32 tid) {
    if (o->kind == MUTEX) return o->count == 0 || (u32)o->max == tid;
    return o->count > 0;
}
/* 1 when the object taken was a mutex its owner abandoned (Sprint 356's amendment), its mark cleared: the take answers
 * WAIT_ABANDONED once. */
static int sync_take(struct object *o, u32 tid) {
    int abandoned = 0;
    if (o->kind == MUTEX) {
        o->count++;
        o->max = tid;
        abandoned = (int)o->abandoned;
        o->abandoned = 0;
    } else if (o->kind == SEMAPHORE || o->kind == EVENT_AUTO) {
        o->count--;
    } else if (o->kind == TIMER && !o->max) {
        o->count = 0;     /* "a synchronization timer ... remains in this state until a thread completes a wait" */
    }
    return abandoned;
}

/* Every one of n objects of this file's (by index, `indices`, or by pointer, `os`) taken for thread tid, `sync_lock`
 * held: WAIT_OBJECT_0, or WAIT_ABANDONED_0 itself when any of them is an abandoned mutex (Microsoft Learn,
 * WaitForMultipleObjects: "If bWaitAll is TRUE, a return value within the specified range indicates that the state of
 * all specified objects is signaled and at least one of the objects is an abandoned mutex object"; the page names no
 * index, and Windows answers 128 for all of {a set event, an abandoned mutex at index 1}: probes/mutex_abandoned, run
 * 45871497648, row "all", three runs alike). */
static u64 take_every(u32 n, const int *indices, struct object **os, u32 tid) {
    u64 ret = WAIT_OBJECT_0;
    for (u32 k = 0; k < n; k++) {
        struct object *o = indices ? &objects[indices[k]] : os[k];
        if (sync_kind(o) && sync_take(o, tid)) ret = WAIT_ABANDONED;
    }
    return ret;
}

int w2m_thread_ended(u32 tid);        /* threads.c */

/* One of the host's objects signaled: a thread's handle when the thread has ended (rows wait.worker, wait.again; Sprint
 * 051), a process's when it has, a shared event's when the host's event is set; a pipe's end or another process's
 * thread stops the run. A host-backed object a wait for all still holds after its last handle closed (Sprint 350) reads
 * unsignaled: its host object is closed, and the host refuses an event request on an id no longer an event
 * (crates/w2m_vm/src/kernel.rs 209-211). Host requests: the runtime word held, `sync_lock` not. */
static int host_signaled(struct object *o) {
    if (o->kind == SHARED_EVENT || o->kind == PROCESS) {               /* `handles` is the table's (Sprint 355) */
        W2M_HELD(handles);
        if (!o->handles) return 0;                                     /* the guard's end lets the table go first */
    }
    if (o->kind == THREAD) return w2m_thread_ended((u32)o->count);
    if (o->kind == PROCESS)                                    /* this process runs; another's is the host's */
        return (u32)o->count != (u32)w2m_teb()->client_process &&
               w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_PROCESS_STATE, (u64)o->count, 0, 0, 0) != 259;
    if (o->kind == SHARED_EVENT) return (int)w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_EVENT_STATE, (u64)o->count, 0, 0, 0);
    w2m_unimplemented(W2M_API_WaitForSingleObject, o->kind);     /* a pipe's end, another process's thread */
    return 0;
}

/* A host-backed object taken: a shared event's EVENT_TAKE; a thread or a process gives nothing. The word held. */
static void host_take(struct object *o) {
    if (o->kind == SHARED_EVENT) w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_EVENT_TAKE, (u64)o->count, 0, 0, 0);
}

/* `sync_lock` held; each wait for all taken for is readied once it is let go. A wait for all naming a host object is
 * readied to check again when the release's object `released` is among its objects and every object of this file's it
 * names is signaled for it. */
static void grant_all(const struct object *released, struct readied *r) {
    int index = (int)(released - objects);
    for (int i = 0; i < nall; i++) {
        struct all_waiter *w = &all_waiters[i];
        if (w->granted) continue;
        u32 k = 0;
        if (w->host) {
            int names = 0, all = 1;
            for (k = 0; k < w->n; k++) {
                const struct object *o = &objects[w->objects[k]];
                if (w->objects[k] == index) names = 1;
                if (sync_kind(o) && !sync_signaled(o, w->tid)) all = 0;
            }
            if (names && all) to_ready(r, w->tid);
            continue;
        }
        while (k < w->n && sync_signaled(&objects[w->objects[k]], w->tid)) k++;
        if (k < w->n) continue;
        w->granted = 1 + (int)take_every(w->n, w->objects, 0, w->tid);   /* the answer + 1: 1, or WAIT_ABANDONED_0 + 1 */
        to_ready(r, w->tid);
    }
}

/* Every mutex thread `tid` owns, abandoned as the thread ends (Sprint 356's amendment; reviewer-4's twenty-sixth
 * review, Medium 1): left unowned and marked, then granted to its first waiter, which answers WAIT_ABANDONED, or left
 * for the next wait to take and answer so once. threads.c's W2mThreadEnd calls it, the last call of every thread that
 * ends (ExitThread, and a start routine's return, which the attach stub turns into ExitThread), its bridge holding the
 * runtime word, before the host's THREAD_END. The mutexes are found, and each held by a reference so none is freed and
 * made anew meanwhile, under the table's lock; their state changed and their waiters granted under `sync_lock`; the
 * granted readied under the word; the references dropped under the table's lock. */
void w2m_mutexes_abandoned(u32 tid) {
    int held[OBJECTS], n = 0;
    {
        W2M_HELD(handles);
        for (int i = 0; i < OBJECTS; i++)
            if (objects[i].kind == MUTEX && objects[i].refs) {
                objects[i].refs++;
                held[n++] = i;
            }
    }
    struct readied r;                                  /* its tids read only below n: not zeroed */
    r.n = 0;
    {
        W2M_HELD(sync);
        for (int k = 0; k < n; k++) {
            struct object *o = &objects[held[k]];
            if (!o->count || (u32)o->max != tid) continue;
            o->count = 0;
            o->max = 0;
            o->abandoned = 1;
            grant(o, &r);
            grant_all(o, &r);
        }
    }
    {
        WORD_WHEN_NEEDED(word);
        ready_all(&r, &word);
    }
    for (int k = 0; k < n; k++) object_done(&objects[held[k]]);
}

/* The object of one of two kinds that h's lookup gave, its reference the caller's; any other stops the run. */
static struct object *must(u64 api, u64 h, struct object *o, u32 kind1, u32 kind2) {
    if (!o || (o->kind != kind1 && o->kind != kind2)) w2m_unimplemented(api, h);   /* not in the rows */
    return o;
}

/* The releases (Sprint 356): the object looked up under the table's lock; its state changed and its waiters granted
 * under `sync_lock`; then, that let go, the granted readied and a manual-reset event's waiters signaled under the
 * runtime word, and the lookup's reference dropped under the table's lock. A shared event is the host's: its request,
 * and the signal of this process's waiters, under the word. */
/* An event set, a semaphore released by `count` (`*previous` its count before) or a mutex released, as SetEvent,
 * ReleaseSemaphore and ReleaseMutex do (Sprint 369: theirs, and SignalObjectAndWait's signal): 0, or the error the
 * release fails with, the state then unchanged. `o` is a lookup's, the caller's to drop, of one of those kinds. */
static u32 signal_object(struct object *o, s32 count, s32 *previous) {
    struct readied r;                              /* its tids read only below n: not zeroed */
    r.n = 0;
    WORD_WHEN_NEEDED(word);
    if (o->kind == SHARED_EVENT) {
        take_word(&word);
        w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_EVENT_SET, (u64)o->count, 0, 0, 0);
        w2m_signal(0);
        return 0;
    }
    {
        W2M_HELD(sync);
        if (o->kind == SEMAPHORE) {
            if (o->count + count > o->max) return ERROR_TOO_MANY_POSTS;
            *previous = (s32)o->count;
            o->count += count;
        } else if (o->kind == MUTEX) {
            if (o->count == 0 || (u32)o->max != current_tid()) return ERROR_NOT_OWNER;
            if (--o->count) return 0;                  /* still owned, recursively */
            o->max = 0;
        } else {
            o->count = 1;
        }
        grant(o, &r);
        grant_all(o, &r);
    }
    ready_all(&r, &word);                        /* an auto-reset event's grants ready its waiters (095) */
    if (o->kind == EVENT_MANUAL) {
        take_word(&word);
        w2m_signal((u64)o);                      /* a manual-reset event releases every waiter */
    }
    return 0;
}

u64 k32_SetEvent(u64 h) {
    struct object *o = object_of(h);
    if (!o || o->kind != SHARED_EVENT) must(W2M_API_SetEvent, h, o, EVENT_MANUAL, EVENT_AUTO);
    signal_object(o, 0, 0);
    object_done(o);
    return w2m_logged(W2M_API_SetEvent, h, 1);
}
u64 k32_ResetEvent(u64 h) {
    struct object *o = object_of(h);
    if (o && o->kind == SHARED_EVENT) {
        W2M_HELD(runtime_word);
        w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_EVENT_RESET, (u64)o->count, 0, 0, 0);
    } else {
        must(W2M_API_ResetEvent, h, o, EVENT_MANUAL, EVENT_AUTO);
        W2M_HELD(sync);
        o->count = 0;
    }
    object_done(o);
    return w2m_logged(W2M_API_ResetEvent, h, 1);
}

u64 k32_ReleaseSemaphore(u64 h, u64 count, u64 previous) {
    struct object *o = must(W2M_API_ReleaseSemaphore, h, object_of(h), SEMAPHORE, SEMAPHORE);
    if ((s32)count <= 0) w2m_unimplemented(W2M_API_ReleaseSemaphore, count);
    s32 before = 0;
    u32 error = signal_object(o, (s32)count, &before);
    object_done(o);
    if (error) {
        set_error(error);                              /* ERROR_TOO_MANY_POSTS, the previous count unwritten */
        return w2m_logged(W2M_API_ReleaseSemaphore, h, 0);
    }
    if (previous) *(s32 *)previous = before;
    return w2m_logged(W2M_API_ReleaseSemaphore, h, 1);
}

u64 k32_ReleaseMutex(u64 h) {
    struct object *o = must(W2M_API_ReleaseMutex, h, object_of(h), MUTEX, MUTEX);
    u32 error = signal_object(o, 0, 0);
    object_done(o);
    if (error) {
        set_error(error);                              /* ERROR_NOT_OWNER */
        return w2m_logged(W2M_API_ReleaseMutex, h, 0);
    }
    return w2m_logged(W2M_API_ReleaseMutex, h, 1);
}

/* Not signaled: 1 when the wait is over (WAIT_TIMEOUT now, the deadline cleared); 0 when the thread parks until
 * `*deadline` and retries the call (runtime.h W2M_SCHEDULE_RETRY), other threads running meanwhile; -1 when the counter
 * is needed first (`now`, `have_now`), read with no leaf held, as it is a host request on the host's clock. ms is from
 * the first try, the deadline kept in the TEB, the waiting thread's own (only it and its vCPU's switch read it). */
/* The wait's own deadline (Sprint 370): `wait_timeout` while a timer's time comes first and `wait_deadline` holds that
 * (`park_until`), else `wait_deadline`; 0 on a first try. A wait that names no timer and runs no APC never sets
 * `wait_timeout`, so for it this is `wait_deadline`, as before. */
static u64 own_deadline(const struct w2m_thread_scratch *sc) {
    return sc->wait_timeout ? sc->wait_timeout : sc->wait_deadline;
}

/* A wait over: both words cleared (Sprint 370; before it, `wait_deadline` alone, `wait_timeout` then always 0). */
static void wait_over(struct w2m_thread_scratch *sc) {
    sc->wait_deadline = 0;
    sc->wait_timeout = 0;
}

static int timed_out_at(u32 ms, int have_now, u64 now, u64 *deadline) {
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    if (ms == 0) return 1;
    u64 own = own_deadline(sc);
    if (!own) {
        if (ms == INFINITE) {
            *deadline = ~0ull;
            return 0;
        }
        if (!have_now) return -1;
        *deadline = now + (u64)ms * 10000;
        return 0;
    }
    if (own != ~0ull) {
        if (!have_now) return -1;
        if (now >= own) {
            wait_over(sc);
            return 1;
        }
    }
    *deadline = own;
    return 0;
}

/* A wait's park (W2M_SCHEDULE_RETRY) until its own `deadline`, or until `wake` when that comes first (Sprint 370): a
 * timer's next expiry among its objects, or the expiry of a timer's APC queued to it. The thread is woken at the
 * earlier, its wait checked again; `wait_deadline` holds that time, which the switch and the host read (threads.rs
 * reads a parked thread's deadline from the scratch's first word), and `wait_timeout` the wait's own. With no earlier
 * `wake`, the words are as before Sprint 370, so a wait that names no timer parks as it did. */
static void park_until(u64 deadline, u64 wake, u64 key) {
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    if (wake < deadline) {
        sc->wait_timeout = deadline;
        sc->wait_deadline = wake;
        park(wake, key);
        return;
    }
    if (sc->wait_timeout) {                        /* the timer's time passed or went: the wait's own again */
        sc->wait_deadline = deadline;
        sc->wait_timeout = 0;
    }
    park(deadline, key);
}

/* `timed_out_at` with the counter read when it is needed, no leaf held. */
static int timed_out_now(u32 ms, u64 *deadline) {
    int t = timed_out_at(ms, 0, 0, deadline);
    return t < 0 ? timed_out_at(ms, 1, w2m_counter_now(), deadline) : t;
}

/* Waitable timers and asynchronous procedure calls (Sprint 370; docs/unserved-declared-imports-2026-11-27.md:
 * mono-2.0-bdwgc.dll's QueueUserAPC, UnityPlayer.dll's CreateWaitableTimerA and SetWaitableTimer), held to
 * probes/apcs_and_waitable_timers' Windows rows (contracts/windows.sync.apcs_and_waitable_timers).
 *
 * A timer's state is brought to a time when it is read (`timer_run`): at its due time it is signaled, and a periodic
 * timer's next due time is its first after that time, "A periodic timer automatically reactivates each time the period
 * elapses" (Microsoft Learn, SetWaitableTimer); a timer set once becomes inactive, "When the specified due time arrives,
 * the timer becomes inactive". A wait that names a timer parks until the timer's due time if that comes before its own
 * deadline (`park_until`), so it checks the timer again then; SetWaitableTimer readies the waits on the timer, which
 * check it again at its new due time.
 *
 * The APC queue, one for every thread, entries by thread id: QueueUserAPC's, "Each thread has its own APC queue. The
 * queuing of an APC is a request for the thread to call the APC function" (QueueUserAPC), and SetWaitableTimer's
 * completion routine's, queued "to the thread that set the timer" at the due time. A timer's entry is made at
 * SetWaitableTimer and is due at its expiry (`at`), before which no wait takes it: so "if there is no outstanding APC
 * already queued" holds by the one entry, which a periodic timer's delivery puts at the first expiry after it; a second
 * SetWaitableTimer removes it, "it does cancel any pending completion routines". An alertable wait takes the thread's
 * due entries in the order they were queued, a timer's at its expiry and QueueUserAPC's at the counter read as it was
 * queued, and calls each, "the thread handles all pending APCs in first in, first out (FIFO) order, and the wait
 * operation returns WAIT_IO_COMPLETION" (QueueUserAPC), an APC that one of them queues among them, "An alertable
 * function will return only after all APC entries have been processed" (Using a Waitable Timer with an Asynchronous
 * Procedure Call). The calls are guest_calls.c's: a frame below the wait's stack pointer, each APC called through the
 * CALL stub, the next taken at its return (`apc_next`), the wait's caller answered WAIT_IO_COMPLETION once none is due.
 * The queue is `sync_lock`'s, so an alertable wait's check of its objects, of its queue and its arming are one holding:
 * QueueUserAPC queues under the lock and readies the thread under the runtime word, which finds it parked or armed, as a
 * release's grant does. It readies the thread in any wait, alertable or not: a wait that is not checks again and parks
 * again, its answer the same. With no APC queued and no timer named, every wait reads the queue's count and nothing
 * more, and its answer and its writes are those before Sprint 370. */
#define WAIT_IO_COMPLETION 0xC0u                   /* "0x000000C0L" (WaitForSingleObjectEx) */
#define ERROR_INVALID_PARAMETER 87u
#define CURRENT_THREAD (~0ull - 1)
#define APCS 64

/* A timer brought to `now`, `sync_lock` held: its next due time while it is set, ~0 once it is not. */
static u64 timer_run(struct object *o, u64 now) {
    if (o->active && now >= o->due) {
        o->count = 1;
        if (o->period) {
            u64 p = (u64)o->period * 10000;
            o->due += ((now - o->due) / p + 1) * p;
        } else {
            o->active = 0;
        }
    }
    return o->active ? o->due : ~0ull;
}

/* Every timer of n objects brought to `now`, `sync_lock` held: the earliest next due time among them, ~0 for none. */
static u64 timers_run(struct object **os, u32 n, u64 now) {
    u64 wake = ~0ull;
    for (u32 i = 0; i < n; i++)
        if (os[i]->kind == TIMER) {
            u64 d = timer_run(os[i], now);
            if (d < wake) wake = d;
        }
    return wake;
}

/* Whether any of n objects is a timer: its kind is fixed, read with no lock by a holder of a reference. */
static int has_timer(struct object *const *os, u32 n) {
    for (u32 i = 0; i < n; i++)
        if (os[i]->kind == TIMER) return 1;
    return 0;
}

/* The queue: a thread's entry, its function and argument, the time it is due from (`at`: a timer's expiry, a
 * QueueUserAPC's counter as it was queued, due at once), the order of its queuing (`seq`), and for a timer's
 * completion routine the timer, its entry holding a reference (Sprint 352's rule, as a waiter entry holds one), and its
 * period in counter ticks. `napcs` is written under `sync_lock`, by a relaxed store, and read with no lock by SleepEx
 * and a thread's end only to see whether any entry exists (`w2m_apcs_queued`). */
static struct apc { u32 tid; u64 fn, arg, at, seq, period; struct object *timer; } apcs[APCS];
static u32 napcs;
static u64 apc_seq;

static void apc_remove(u32 i) {
    apcs[i] = apcs[napcs - 1];
    __atomic_store_n(&napcs, napcs - 1, __ATOMIC_RELAXED);
}

/* An entry taken for its call: its function and argument, and a timer's expiry and the reference to drop. */
struct apc_call { u64 fn, arg, expiry; struct object *done; int timer; };

/* Thread tid's first due entry taken, `sync_lock` held: 1 and `*c`; 0 when none is due, `*wake` then the earliest time
 * a timer's entry of its is due from; -1 when it has a timer's entry and the counter is needed (`now`, `have_now`). A
 * periodic timer's entry stays, due from its first expiry after `now`; any other is removed. */
static int apc_take(u32 tid, int have_now, u64 now, struct apc_call *c, u64 *wake) {
    int best = -1;
    for (u32 i = 0; i < napcs; i++) {
        const struct apc *a = &apcs[i];
        if (a->tid != tid) continue;
        if (a->timer) {
            if (!have_now) return -1;
            if (a->at > now) {
                if (a->at < *wake) *wake = a->at;
                continue;
            }
        }
        if (best < 0 || a->at < apcs[best].at || (a->at == apcs[best].at && a->seq < apcs[best].seq)) best = (int)i;
    }
    if (best < 0) return 0;
    struct apc *a = &apcs[best];
    *c = (struct apc_call){a->fn, a->arg, a->at, 0, a->timer != 0};
    if (a->timer && a->period) {
        a->at += ((now - a->at) / a->period + 1) * a->period;       /* the first expiry after this delivery */
        a->seq = ++apc_seq;
        return 1;
    }
    c->done = a->timer;
    apc_remove((u32)best);
    return 1;
}

/* A taken entry called through the CALL stub from `frame`: a timer's reference dropped (the table's lock, no leaf held)
 * and its routine given "the UTC-based time at which the timer was signaled" (TimerAPCProc), its expiry as the system
 * time, low and high words; a QueueUserAPC's function its data alone (PAPCFUNC). */
static void apc_call_guest(struct w2m_guest_call *frame, struct apc_call *c) {
    object_done(c->done);
    u64 low = 0, high = 0;
    if (c->timer) {
        u64 t;
        {
            W2M_HELD(runtime_word);            /* the system time's first reading, and the host's clock, ask the host */
            u64 ft = w2m_filetime_now(), now = w2m_counter_now();
            t = now >= c->expiry ? ft - (now - c->expiry) : ft;
        }
        low = (u32)t;
        high = t >> 32;
    }
    w2m_call_guest(frame, c->fn, c->arg, low, high, 0);
}

/* The APCs' frame: the wait's stack pointer, its API and logged argument, for its answer once none is due. */
struct apc_frame {
    struct w2m_guest_call call;
    u64 rsp, api, arg0;
};

/* An APC returned (W2mGuestReturn, the runtime word held): the thread's next due entry called, or the wait's caller
 * answered WAIT_IO_COMPLETION. */
static void apc_next(struct w2m_guest_call *frame, u64 answer) {
    (void)answer;
    struct apc_frame *f = (struct apc_frame *)frame;
    u32 me = current_tid();
    struct apc_call c;
    u64 now = 0, wake;
    int have_now = 0, t;
    for (;;) {
        wake = ~0ull;
        {
            W2M_HELD(sync);
            t = apc_take(me, have_now, now, &c, &wake);
        }
        if (t >= 0) break;
        now = w2m_counter_now();
        have_now = 1;
    }
    if (t) {
        apc_call_guest(frame, &c);
        return;
    }
    w2m_return_to(f->rsp, w2m_logged(f->api, f->arg0, WAIT_IO_COMPLETION));
}

/* An alertable wait's APCs run, the first taken (`c`), no leaf held: its frame made below the wait's stack pointer
 * `rsp` and the first called; the answer is its function's address, which the bridge puts in RAX, where the CALL stub
 * reads it (`w2m_call_guest` writes it too, and a service's answer overwrites RAX in the state block, guest.rs). */
static u64 apcs_start(u64 api, u64 arg0, u64 rsp, struct apc_call *c) {
    struct apc_frame *f = (struct apc_frame *)w2m_guest_frame(rsp, sizeof *f, apc_next);
    f->rsp = rsp;
    f->api = api;
    f->arg0 = arg0;
    apc_call_guest(&f->call, c);
    return c->fn;
}

/* For threads.c: whether any entry is queued, read with no lock (SleepEx's and a thread end's first look). */
int w2m_apcs_queued(void) { return __atomic_load_n(&napcs, __ATOMIC_RELAXED) != 0; }

/* QueueUserAPC(pfnAPC, hThread, dwData): "Adds a user-mode asynchronous procedure call (APC) object to the APC queue of
 * the specified thread"; "If the function succeeds, the return value is nonzero" (Microsoft Learn). The thread is
 * this one (GetCurrentThread's pseudo-handle) or a thread handle's; another process's thread, or an object of another
 * kind, stops the run (not in the rows). The entry is queued under `sync_lock`, then the thread readied under the
 * runtime word, unless it is this one. */
u64 k32_QueueUserAPC(u64 fn, u64 h, u64 data) {
    u32 tid = current_tid();
    if (!fn) w2m_unimplemented(W2M_API_QueueUserAPC, fn);
    if (h != CURRENT_THREAD) {
        struct object *o = object_of(h);
        if (!o) {
            if (h < OBJECT_BASE) w2m_unimplemented(W2M_API_QueueUserAPC, h);      /* a file's, the process's: no row */
            set_error(ERROR_INVALID_HANDLE);                                     /* row apc.invalid, closed */
            return w2m_logged(W2M_API_QueueUserAPC, h, 0);
        }
        int thread = o->kind == THREAD;
        if (thread) tid = (u32)o->count;
        object_done(o);
        if (!thread) w2m_unimplemented(W2M_API_QueueUserAPC, h);
    }
    u64 at = w2m_counter_now();
    {
        W2M_HELD(sync);
        if (napcs == APCS) {
            W2M_LET_GO_EARLY(sync);
            w2m_unimplemented(W2M_API_QueueUserAPC, APCS);
        }
        apcs[napcs] = (struct apc){tid, fn, data, at, ++apc_seq, 0, 0};
        __atomic_store_n(&napcs, napcs + 1, __ATOMIC_RELAXED);
    }
    if (tid != current_tid()) {
        W2M_HELD(runtime_word);
        w2m_ready_thread(tid);
    }
    return w2m_logged(W2M_API_QueueUserAPC, fn, 1);
}

/* SetWaitableTimer(hTimer, lpDueTime, lPeriod, pfnCompletionRoutine, lpArgToCompletionRoutine, fResume), the last two on
 * the guest's stack at RSP + 0x28 and + 0x30: lpDueTime "in 100 nanosecond intervals ... Positive values indicate
 * absolute time ... Negative values indicate relative time", on the counter the waits read (an absolute time by its
 * distance from the system time now, one past at once); "If the timer is already active when you call
 * SetWaitableTimer, the timer is stopped, then it is reactivated ... it does cancel any pending completion routines";
 * "When a timer is set, it is canceled if it was already active, the state of the timer is nonsignaled" (Waitable
 * Timer Objects). A routine's entry is queued to this thread and keeps the lookup's reference; the waits on the timer
 * are readied to check it again. fResume TRUE, a restore from a power state, stops the run (not in the rows). */
u64 k32_SetWaitableTimer(u64 h, u64 due_time, u64 period, u64 routine, u64 rsp) {
    u64 arg = *(const u64 *)(rsp + 0x28);
    u32 resume = *(const u32 *)(rsp + 0x30);
    struct object *o = object_of(h);
    if (!o || o->kind != TIMER) {
        if (h < OBJECT_BASE) w2m_unimplemented(W2M_API_SetWaitableTimer, h);
        object_done(o);
        set_error(ERROR_INVALID_HANDLE);                                         /* row timer.invalid, event */
        return w2m_logged(W2M_API_SetWaitableTimer, h, 0);
    }
    if ((s32)period < 0) {                                         /* "If lPeriod is less than zero, the function fails" */
        object_done(o);
        set_error(ERROR_INVALID_PARAMETER);                                      /* row timer.invalid, period */
        return w2m_logged(W2M_API_SetWaitableTimer, h, 0);
    }
    if (resume || !due_time) w2m_unimplemented(W2M_API_SetWaitableTimer, resume);
    s64 due = *(const s64 *)due_time;
    u64 at;
    {
        W2M_HELD(runtime_word);            /* the system time's first reading, and the host's clock, ask the host */
        if (due < 0) {
            at = w2m_counter_now() + ((u64)0 - (u64)due);
        } else {
            u64 ft = w2m_filetime_now(), now = w2m_counter_now();
            at = (u64)due > ft ? now + ((u64)due - ft) : now;
        }
    }
    struct readied r;                              /* its tids read only below n: not zeroed */
    r.n = 0;
    struct object *cancelled[APCS];
    u32 nc = 0;
    {
        W2M_HELD(sync);
        for (u32 i = 0; i < napcs;)
            if (apcs[i].timer == o) {
                cancelled[nc++] = o;
                apc_remove(i);
            } else {
                i++;
            }
        o->count = 0;
        o->due = at;
        o->period = (u32)period;
        o->active = 1;
        if (routine) {
            if (napcs == APCS) {
                W2M_LET_GO_EARLY(sync);
                w2m_unimplemented(W2M_API_SetWaitableTimer, APCS);
            }
            apcs[napcs] = (struct apc){current_tid(), routine, arg, at, ++apc_seq, (u64)(u32)period * 10000, o};
            __atomic_store_n(&napcs, napcs + 1, __ATOMIC_RELAXED);
        }
        int index = (int)(o - objects);
        for (int i = 0; i < nwaiters; i++)
            if (waiters[i].object == index) to_ready(&r, waiters[i].tid);
        for (int i = 0; i < nall; i++)
            for (u32 k = 0; k < all_waiters[i].n; k++)
                if (all_waiters[i].objects[k] == index) to_ready(&r, all_waiters[i].tid);
    }
    {
        WORD_WHEN_NEEDED(word);
        ready_all(&r, &word);
    }
    for (u32 i = 0; i < nc; i++) object_done(cancelled[i]);
    if (!routine) object_done(o);                  /* with a routine, its entry holds the lookup's reference */
    return w2m_logged(W2M_API_SetWaitableTimer, h, 1);
}

/* For threads.c's SleepEx, alertable, an APC queued or its time INFINITE, its bridge holding the runtime word: the
 * thread's due APCs run (WAIT_IO_COMPLETION), or with none a yield for 0 ms ("A value of zero causes the thread to
 * relinquish the remainder of its time slice"), the sleep's end at its time (0), or its park until then or until a
 * timer's entry of its is due; "If an APC is queued to the thread (QueueUserAPC), the function returns when either the
 * time-out period has elapsed or when the APC function is called" (SleepEx). */
u64 w2m_sleep_alertable(u32 ms, u64 rsp) {
    u32 me = current_tid();
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    struct apc_call c;
    u64 now = 0, wake, deadline = 0;
    int have_now = 0, t;
    for (;;) {
        wake = ~0ull;
        {
            W2M_HELD(sync);
            t = apc_take(me, have_now, now, &c, &wake);
        }
        if (t >= 0) break;
        now = w2m_counter_now();
        have_now = 1;
    }
    if (t) {
        wait_over(sc);
        return apcs_start(W2M_API_SleepEx, ms, rsp, &c);
    }
    if (ms == 0) {
        w2m_state()->schedule |= W2M_SCHEDULE_YIELD;
        return w2m_logged(W2M_API_SleepEx, 0, 0);
    }
    int over = timed_out_at(ms, have_now, now, &deadline);
    if (over < 0) over = timed_out_at(ms, 1, w2m_counter_now(), &deadline);
    if (over) return w2m_logged(W2M_API_SleepEx, ms, 0);
    park_until(deadline, wake, W2M_KEY_NONE);
    return 0;                                      /* retried: the bridge does not return */
}

/* For threads.c's W2mThreadEnd, its bridge holding the runtime word: thread tid's entries removed, "When the thread is
 * terminated using the ExitThread function or TerminateThread function function, the APCs in its APC queue are lost"
 * (QueueUserAPC), and each timer it set with a routine canceled at its state now, "If the thread that set the timer
 * terminates and there is an associated completion routine, the timer is canceled. However, the state of the timer
 * remains unchanged" (SetWaitableTimer). With no entry queued, nothing is read or written. */
void w2m_apcs_ended(u32 tid) {
    if (!w2m_apcs_queued()) return;
    u64 now = w2m_counter_now();
    struct object *dropped[APCS];
    u32 nd = 0;
    {
        W2M_HELD(sync);
        for (u32 i = 0; i < napcs;) {
            struct apc *a = &apcs[i];
            if (a->tid != tid) {
                i++;
                continue;
            }
            if (a->timer) {
                timer_run(a->timer, now);
                a->timer->active = 0;
                dropped[nd++] = a->timer;
            }
            apc_remove(i);
        }
    }
    for (u32 i = 0; i < nd; i++) object_done(dropped[i]);
}

/* Every way out (Sprint 350; Sprint 352 every kind): a handle below the objects' stops the run; one that names no object
 * answers WAIT_FAILED and ERROR_INVALID_HANDLE and clears the deadline, and only a first try meets it, a retry reading
 * its entry; a grant, a signaled object and a timeout each take the object if signaled, then drop the entry on a retry
 * or the lookup's reference on a first try, and clear the deadline (`timed_out_at` clears its own); a block keeps the
 * deadline and, on a first try, enters the thread's entry with the lookup's reference (a full list stops the run), of
 * any kind of object. A retried wait waits on the entry's object, not on what `h` names now: the object outlives a
 * handle closed meanwhile, as an NT wait holds its object by reference (ReactOS NtWaitForSingleObject,
 * ObReferenceObjectByHandle before KeWaitForSingleObject and ObDereferenceObject after).
 * Sprint 356: the entry read under `sync_lock`, the lookup under the table's lock, neither held with the other. One of
 * this file's objects is checked, and its entry made, in one holding of `sync_lock`, so a release either comes before
 * the check, which sees it, or finds the entry and grants it; the woken mark is armed in that holding, so a readying
 * between it and the park leaves the thread ready, and the park takes the word alone (`park`). One of the host's
 * objects is checked under the word, held from the check to the park (`wait_one_held`). */
static u64 wait_one_held(u64 api, u64 h, struct object *o, u32 ms, int retry, int alertable, u64 rsp);

/* Sprint 359: each holding of `sync_lock` a guard's scope, the loop's body in the waits below. What must run with no
 * leaf held after it, the counter's read, the park and the lookups' references dropped, runs after the scope's end:
 * `need_now` and a `continue` bring the read to the loop's top, `parked` and a `break` the park to the loop's end.
 * Sprint 370: a timer is brought to the time first, the counter read before the holding (`need_now` from the start);
 * an alertable wait whose object is not signaled takes a due APC before it times out or parks, "If dwMilliseconds is
 * zero, the function tests the object's state, checks for queued completion routines or APCs, and returns
 * immediately" (SignalObjectAndWait), its entry and its words then dropped as a timeout's are, its APCs run after the
 * holding (`apcs_start`); a park waits to the timer's due time or a timer APC's when either comes first. */
static u64 wait_one(u64 api, u64 h, u32 ms, int alertable, u64 rsp) {
    u32 me = current_tid();
    int w;
    struct object *o;
    {
        W2M_HELD(sync);
        w = waiter_of(me);
        o = w >= 0 ? &objects[waiters[w].object] : 0;          /* a retry: only this thread drops its entry */
    }
    int retry = o != 0;
    if (!retry) o = object_of(h);
    if (!o) {
        if (h < OBJECT_BASE) w2m_unimplemented(api, h);         /* a file, a process, a pseudo-handle: not in the rows */
        w2m_thread_scratch()->wait_deadline = 0;
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(api, h, WAIT_FAILED);
    }
    if (!sync_kind(o)) return wait_one_held(api, h, o, ms, retry, alertable, rsp);
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    struct apc_call call;
    u64 ret = WAIT_OBJECT_0, now = 0, deadline = 0, wake = ~0ull;
    int have_now = 0, need_now = o->kind == TIMER, parked = 0, apc = 0;
    for (;;) {
        if (need_now) {                                        /* a host request on the host's clock: no leaf held */
            now = w2m_counter_now();
            have_now = 1;
            need_now = 0;
        }
        W2M_HELD(sync);
        wake = o->kind == TIMER ? timer_run(o, now) : ~0ull;
        w = waiter_of(me);
        if (w >= 0 && waiters[w].granted) {                    /* a release handed it over while this thread waited */
            wait_over(sc);
            if (waiters[w].granted == ABANDONED_GRANT) ret = WAIT_ABANDONED;   /* its owner ended (amendment) */
        } else if (!sync_signaled(o, me)) {
            if (alertable) {
                int a = apc_take(me, have_now, now, &call, &wake);
                if (a < 0) {
                    need_now = 1;
                    continue;
                }
                if (a) {
                    wait_over(sc);
                    apc = 1;
                    if (w >= 0) waiters_out(me);
                    break;
                }
            }
            int t = timed_out_at(ms, have_now, now, &deadline);
            if (t < 0) {
                need_now = 1;
                continue;
            }
            if (!t) {                                          /* retried: the bridge does not return */
                if (w < 0) {
                    if (nwaiters == WAITERS) {
                        W2M_LET_GO_EARLY(sync);
                        w2m_unimplemented(api, WAITERS);
                    }
                    waiter_add(me, o, -1);                     /* the lookup's reference, kept */
                }
                w2m_wait_armed();
                parked = 1;
                break;
            }
            ret = WAIT_TIMEOUT;
        } else {
            wait_over(sc);
            if (sync_take(o, me)) ret = WAIT_ABANDONED;         /* its owner ended holding it (amendment) */
        }
        if (w >= 0) waiters_out(me);
        break;
    }
    if (parked) {
        park_until(deadline, wake, (u64)o);
        return 0;
    }
    object_done(o);                                            /* the entry's reference, or the lookup's */
    if (apc) return apcs_start(api, h, rsp, &call);
    return w2m_logged(api, h, ret);
}

/* A wait on one of the host's objects (Sprint 356): the runtime word held from its check, a host request, to its park,
 * so no readying of the host's, nor this process's signal of a shared event, falls between them; its entry, never
 * granted, made and taken out under `sync_lock`, taken under the word as the order allows. */
/* Sprint 370: alertable, a due APC is taken before the timeout and the park, in its own holding of `sync_lock` under
 * the word, which a QueueUserAPC's readying waits for, so one queued after it finds the thread parked; its APCs run
 * once the word is let go. */
static u64 wait_one_held(u64 api, u64 h, struct object *o, u32 ms, int retry, int alertable, u64 rsp) {
    u64 ret = WAIT_OBJECT_0, deadline = 0, wake = ~0ull;
    struct apc_call call;
    int apc = 0;
    {
        W2M_HELD(runtime_word);
        if (!host_signaled(o)) {
            if (alertable) {
                u64 now = 0;
                int have_now = 0;
                for (;;) {
                    {
                        W2M_HELD(sync);
                        apc = apc_take(current_tid(), have_now, now, &call, &wake);
                    }
                    if (apc >= 0) break;
                    now = w2m_counter_now();
                    have_now = 1;
                }
            }
            if (apc) {
                wait_over(w2m_thread_scratch());
            } else if (!timed_out_now(ms, &deadline)) {      /* retried: the bridge does not return */
                if (!retry) {
                    W2M_HELD(sync);
                    if (nwaiters == WAITERS) {
                        W2M_LET_GO_EARLY(sync);
                        w2m_unimplemented(api, WAITERS);
                    }
                    waiter_add(current_tid(), o, -1);          /* the lookup's reference, kept */
                }
                park_until(deadline, wake, (u64)o);
                return 0;                                      /* the word's guard lets it go after the block */
            } else {
                ret = WAIT_TIMEOUT;
            }
        } else {
            w2m_thread_scratch()->wait_deadline = 0;
            host_take(o);
        }
        if (retry) {
            W2M_HELD(sync);
            waiters_out(current_tid());
        }
    }
    object_done(o);
    if (apc) return apcs_start(api, h, rsp, &call);
    return w2m_logged(api, h, ret);
}

u64 k32_WaitForSingleObject(u64 h, u64 ms) { return wait_one(W2M_API_WaitForSingleObject, h, (u32)ms, 0, 0); }

/* WaitForSingleObjectEx(hHandle, dwMilliseconds, bAlertable): "If this parameter is TRUE and the thread is in the
 * waiting state, the function returns when the system queues an I/O completion routine or APC, and the thread runs the
 * routine or function. Otherwise, the function does not return, and the completion routine or APC function is not
 * executed" (Microsoft Learn); the stack pointer, the bridge's fifth argument, for the APCs' frame (Sprint 370). */
u64 k32_WaitForSingleObjectEx(u64 h, u64 ms, u64 alertable, u64 unused, u64 rsp) {
    (void)unused;
    return wait_one(W2M_API_WaitForSingleObjectEx, h, (u32)ms, (u32)alertable != 0, rsp);
}

/* SignalObjectAndWait(hObjectToSignal, hObjectToWaitOn, dwMilliseconds, bAlertable) (Sprint 369): the object signaled
 * as SetEvent, ReleaseSemaphore by one or ReleaseMutex signals it ("This object can be a semaphore, a mutex, or an
 * event"; "If the handle is a mutex and the caller does not own the mutex, the function fails with ERROR_NOT_OWNER",
 * Microsoft Learn), then the wait WaitForSingleObject makes. A failed signal answers WAIT_FAILED with its error, the
 * wait not made; a handle that names no object WAIT_FAILED and ERROR_INVALID_HANDLE, both looked up before the signal.
 * The wait parks as WaitForSingleObject's does and its call is made again when the thread runs: that retry, which finds
 * this thread's waiter entry, signals nothing again. "Note that the "signal" and "wait" are not guaranteed to be
 * performed as an atomic operation": they are two holdings here. bAlertable is read as WaitForSingleObjectEx reads it
 * (Sprint 370): "If this parameter is TRUE, the function returns when the system queues an I/O completion routine or
 * APC function, and the thread calls the function", the stack pointer the bridge's fifth argument. */
u64 k32_SignalObjectAndWait(u64 signal, u64 wait, u64 ms, u64 alertable, u64 rsp) {
    int retry;
    {
        W2M_HELD(sync);
        retry = waiter_of(current_tid()) >= 0;             /* a retry: only this thread drops its entry */
    }
    if (!retry) {
        struct object *w = object_of(wait);
        int waits = w != 0;
        object_done(w);                                    /* wait_one looks it up again */
        struct object *o = object_of(signal);
        if (!o || !waits) {
            if (signal < OBJECT_BASE || wait < OBJECT_BASE) w2m_unimplemented(W2M_API_SignalObjectAndWait, signal);
            object_done(o);
            w2m_thread_scratch()->wait_deadline = 0;
            set_error(ERROR_INVALID_HANDLE);
            return w2m_logged(W2M_API_SignalObjectAndWait, signal, WAIT_FAILED);
        }
        if (o->kind != SEMAPHORE && o->kind != MUTEX && o->kind != EVENT_MANUAL && o->kind != EVENT_AUTO &&
            o->kind != SHARED_EVENT)
            w2m_unimplemented(W2M_API_SignalObjectAndWait, signal);   /* a thread's, a process's: not in the rows */
        s32 before;
        u32 error = signal_object(o, 1, &before);
        object_done(o);
        if (error) {
            w2m_thread_scratch()->wait_deadline = 0;
            set_error(error);
            return w2m_logged(W2M_API_SignalObjectAndWait, signal, WAIT_FAILED);
        }
    }
    return wait_one(W2M_API_SignalObjectAndWait, wait, (u32)ms, (u32)alertable != 0, rsp);
}

/* The objects of thread tid's blocked wait for any, by their index in its array, from its entries (Sprint 352); n. */
static u32 entries_of(u32 tid, struct object **os) {
    u32 n = 0;
    for (int w = 0; w < nwaiters; w++)
        if (waiters[w].tid == tid && waiters[w].slot >= 0) {
            os[waiters[w].slot] = &objects[waiters[w].object];
            n++;
        }
    return n;
}

/* The lookups' references of a first try, or a retry's entries', dropped (the table's lock, `sync_lock` let go). */
static void objects_done(struct object **os, u32 n) {
    for (u32 i = 0; i < n; i++) object_done(os[i]);
}

/* A wait for any of n objects of this file's (Sprint 356): its grant, its first signaled object taken, its timeout, or
 * its entries made and its mark armed, in one holding of `sync_lock`; the park takes the word alone. */
static u64 wait_any(u32 n, struct object **os, u32 ms, int retry, int alertable, u64 rsp) {
    u32 me = current_tid();
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    struct apc_call call;
    u64 ret, now = 0, deadline = 0, wake = ~0ull;
    int have_now = 0, need_now = has_timer(os, n), parked = 0, apc = 0;   /* timers: Sprint 370, as wait_one's */
    for (;;) {
        if (need_now) {                                        /* a host request on the host's clock: no leaf held */
            now = w2m_counter_now();
            have_now = 1;
            need_now = 0;
        }
        W2M_HELD(sync);
        wake = have_now ? timers_run(os, n, now) : ~0ull;
        ret = granted_slot(me);                                /* a release handed one over while this thread waited */
        if (ret != WAIT_TIMEOUT) wait_over(sc);
        for (u32 i = 0; ret == WAIT_TIMEOUT && i < n; i++)
            if (sync_signaled(os[i], me)) {
                wait_over(sc);
                ret = (sync_take(os[i], me) ? WAIT_ABANDONED : WAIT_OBJECT_0) + i;   /* abandoned: the amendment */
            }
        if (ret == WAIT_TIMEOUT && alertable) {
            int a = apc_take(me, have_now, now, &call, &wake);
            if (a < 0) {
                need_now = 1;
                continue;
            }
            if (a) {
                wait_over(sc);
                apc = 1;
                if (retry) waiters_out(me);
                break;
            }
        }
        if (ret == WAIT_TIMEOUT) {
            int t = timed_out_at(ms, have_now, now, &deadline);
            if (t < 0) {
                need_now = 1;
                continue;
            }
            if (!t) {                                          /* retried: the bridge does not return */
                if (!retry) {
                    if (nwaiters + (int)n > WAITERS) {
                        W2M_LET_GO_EARLY(sync);
                        w2m_unimplemented(W2M_API_WaitForMultipleObjects, WAITERS);
                    }
                    for (u32 i = 0; i < n; i++) waiter_add(me, os[i], (int)i);   /* the lookups' references, kept */
                }
                w2m_wait_armed();
                parked = 1;
                break;
            }
        }
        if (retry) waiters_out(me);
        break;
    }
    if (parked) {
        park_until(deadline, wake, 0);
        return 0;
    }
    objects_done(os, n);
    if (apc) return apcs_start(W2M_API_WaitForMultipleObjects, n, rsp, &call);
    return w2m_logged(W2M_API_WaitForMultipleObjects, n, ret);
}

/* A wait for any of n objects one of which is the host's (Sprint 356): the runtime word held from the check to the
 * park. The check runs in today's order, a grant first and then each object to the first signaled one, the host asked
 * only up to it, an object of this file's read in a holding of `sync_lock` of its own; then one holding decides: a
 * grant, else the first object signaled now of this file's or the host's one found so, taken, else the timeout or the
 * park, its entries made. A release between the check and that holding is seen by it; one after it finds the entries,
 * and its readying waits for the word, so the park is never passed over and no mark is needed. */
static u64 wait_any_held(u32 n, struct object **os, u32 ms, int retry, int alertable, u64 rsp) {
    u32 me = current_tid();
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    struct apc_call call;
    u64 wake = ~0ull;
    int apc = 0;
    W2M_HELD(runtime_word);
    u64 granted;
    {
        W2M_HELD(sync);
        granted = granted_slot(me);
    }
    int host = -1;
    for (u32 i = 0; granted == WAIT_TIMEOUT && i < n; i++) {
        if (sync_kind(os[i])) {
            W2M_HELD(sync);
            if (sync_signaled(os[i], me)) break;               /* the guard's end lets the state go first */
        } else if (host_signaled(os[i])) {
            host = (int)i;
            break;
        }
    }
    u64 ret, now = 0, deadline = 0;
    int have_now = 0, need_now = 0, parked = 0;
    for (;;) {
        if (need_now) {                                        /* a host request on the host's clock: no leaf held */
            now = w2m_counter_now();
            have_now = 1;
            need_now = 0;
        }
        W2M_HELD(sync);
        ret = granted_slot(me);
        for (u32 i = 0; ret == WAIT_TIMEOUT && i < n; i++)
            if ((int)i == host || (sync_kind(os[i]) && sync_signaled(os[i], me)))
                ret = ((int)i != host && sync_take(os[i], me) ? WAIT_ABANDONED : WAIT_OBJECT_0) + i;
        if (ret == WAIT_TIMEOUT && alertable) {                /* Sprint 370: a due APC before the timeout and park */
            int a = apc_take(me, have_now, now, &call, &wake);
            if (a < 0) {
                need_now = 1;
                continue;
            }
            if (a) {
                wait_over(sc);
                apc = 1;
                if (retry) waiters_out(me);
                break;
            }
        }
        if (ret == WAIT_TIMEOUT) {
            int t = timed_out_at(ms, have_now, now, &deadline);
            if (t < 0) {
                need_now = 1;
                continue;
            }
            if (!t) {                                          /* retried: the bridge does not return */
                if (!retry) {
                    if (nwaiters + (int)n > WAITERS) {
                        W2M_LET_GO_EARLY(sync);
                        w2m_unimplemented(W2M_API_WaitForMultipleObjects, WAITERS);
                    }
                    for (u32 i = 0; i < n; i++) waiter_add(me, os[i], (int)i);   /* the lookups' references, kept */
                }
                parked = 1;
                break;
            }
        } else {
            sc->wait_deadline = 0;
        }
        if (retry) waiters_out(me);
        break;
    }
    if (parked) {
        park_until(deadline, wake, 0);
        return 0;                                              /* the word's guard lets it go after the block */
    }
    if (!apc && host >= 0 && ret == WAIT_OBJECT_0 + (u32)host) host_take(os[host]);
    W2M_LET_GO_EARLY(runtime_word);
    objects_done(os, n);
    if (apc) return apcs_start(W2M_API_WaitForMultipleObjects, n, rsp, &call);
    return w2m_logged(W2M_API_WaitForMultipleObjects, n, ret);
}

/* A wait for all of n objects of this file's (Sprint 356): its grant (a release took them for it, Sprint 095), every
 * object signaled and taken, its timeout, or its entry made and its mark armed, in one holding of `sync_lock`. */
static u64 wait_all(u32 n, struct object **os, u32 ms, int alertable, u64 rsp) {
    u32 me = current_tid();
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    struct apc_call call;
    u64 ret, now = 0, deadline = 0, wake = ~0ull;
    int have_now = 0, need_now = has_timer(os, n), parked = 0, apc = 0;   /* timers: Sprint 370, as wait_one's */
    for (;;) {
        if (need_now) {                                        /* a host request on the host's clock: no leaf held */
            now = w2m_counter_now();
            have_now = 1;
            need_now = 0;
        }
        W2M_HELD(sync);
        wake = have_now ? timers_run(os, n, now) : ~0ull;
        int a = all_of(me);
        ret = WAIT_OBJECT_0;
        if (a >= 0 && all_waiters[a].granted) {
            wait_over(sc);
            ret = (u64)(all_waiters[a].granted - 1);           /* take_every's answer, kept + 1 (grant_all) */
        } else {
            u32 i = 0;
            while (i < n && sync_signaled(os[i], me)) i++;
            int got = 0;
            if (i < n && alertable) {
                got = apc_take(me, have_now, now, &call, &wake);
                if (got < 0) {
                    need_now = 1;
                    continue;
                }
            }
            if (got) {
                wait_over(sc);
                apc = 1;
            } else if (i < n) {
                int t = timed_out_at(ms, have_now, now, &deadline);
                if (t < 0) {
                    need_now = 1;
                    continue;
                }
                if (!t) {                                      /* retried: the bridge does not return */
                    if (a < 0) {
                        if (nall == ALL_WAITERS) {
                            W2M_LET_GO_EARLY(sync);
                            w2m_unimplemented(W2M_API_WaitForMultipleObjects, ALL_WAITERS);
                        }
                        all_waiters[nall] = (struct all_waiter){me, n, 0, 0, {0}};
                        for (u32 k = 0; k < n; k++)            /* the lookups' references, kept */
                            all_waiters[nall].objects[k] = (int)(os[k] - objects);
                        nall++;
                    }
                    w2m_wait_armed();
                    parked = 1;
                    break;
                }
                ret = WAIT_TIMEOUT;
            } else {
                wait_over(sc);
                ret = take_every(n, 0, os, me);
            }
        }
        if (a >= 0) all_out(a);
        break;
    }
    if (parked) {
        park_until(deadline, wake, 0);
        return 0;
    }
    objects_done(os, n);
    if (apc) return apcs_start(W2M_API_WaitForMultipleObjects, n, rsp, &call);
    return w2m_logged(W2M_API_WaitForMultipleObjects, n, ret);
}

/* A wait for all of n objects one of which is the host's (Sprint 356): the runtime word held from the check to the
 * park, no release ever taking for it (`grant_all`). The check runs in today's order, each object to the first one not
 * signaled, the host asked only up to it, an object of this file's read in a holding of `sync_lock` of its own; then
 * one holding decides: every object of this file's signaled now and the host's found signaled, all taken (the host's
 * after the holding), else the timeout or the park, its entry made. When the object the check stopped at is this
 * file's and is signaled now, a release came between them and found no entry: the thread is marked, and its call
 * checked again at once. A release after that holding finds the entry and readies it, its readying waiting for the
 * word. */
static u64 wait_all_held(u32 n, struct object **os, u32 ms, int retry, int alertable, u64 rsp) {
    u32 me = current_tid();
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    struct apc_call call;
    u64 wake = ~0ull;
    int apc = 0;
    W2M_HELD(runtime_word);
    u32 k = 0;
    for (; k < n; k++) {
        int s;
        if (sync_kind(os[k])) {
            W2M_HELD(sync);
            s = sync_signaled(os[k], me);
        } else {
            s = host_signaled(os[k]);
        }
        if (!s) break;
    }
    u64 ret, now = 0, deadline = 0;
    int have_now = 0, need_now = 0, parked = 0, again = 0;
    for (;;) {
        if (need_now) {                                        /* a host request on the host's clock: no leaf held */
            now = w2m_counter_now();
            have_now = 1;
            need_now = 0;
        }
        W2M_HELD(sync);
        int a = all_of(me), every = k == n;
        for (u32 i = 0; every && i < n; i++)
            if (sync_kind(os[i]) && !sync_signaled(os[i], me)) every = 0;
        int got = 0;
        if (!every && alertable) {                             /* Sprint 370: a due APC before the timeout and park */
            got = apc_take(me, have_now, now, &call, &wake);
            if (got < 0) {
                need_now = 1;
                continue;
            }
        }
        if (every) {
            ret = take_every(n, 0, os, me);
            sc->wait_deadline = 0;
        } else if (got) {
            wait_over(sc);
            apc = 1;
            ret = WAIT_TIMEOUT;                                /* no object taken; the host's are left */
        } else {
            int t = timed_out_at(ms, have_now, now, &deadline);
            if (t < 0) {
                need_now = 1;
                continue;
            }
            if (!t) {                                          /* retried: the bridge does not return */
                if (!retry) {
                    if (nall == ALL_WAITERS) {
                        W2M_LET_GO_EARLY(sync);
                        w2m_unimplemented(W2M_API_WaitForMultipleObjects, ALL_WAITERS);
                    }
                    all_waiters[nall] = (struct all_waiter){me, n, 0, 1, {0}};
                    for (u32 i = 0; i < n; i++)                /* the lookups' references, kept */
                        all_waiters[nall].objects[i] = (int)(os[i] - objects);
                    nall++;
                }
                again = k < n && sync_kind(os[k]) && sync_signaled(os[k], me);
                parked = 1;
                break;
            }
            ret = WAIT_TIMEOUT;
        }
        if (a >= 0) all_out(a);
        break;
    }
    if (parked) {
        if (again) w2m_wait_again();
        park_until(deadline, wake, 0);
        return 0;                                              /* the word's guard lets it go after the block */
    }
    if (ret != WAIT_TIMEOUT)                                   /* WAIT_OBJECT_0, or WAIT_ABANDONED_0 + k */
        for (u32 i = 0; i < n; i++)
            if (!sync_kind(os[i])) host_take(os[i]);
    W2M_LET_GO_EARLY(runtime_word);
    objects_done(os, n);
    if (apc) return apcs_start(W2M_API_WaitForMultipleObjects, n, rsp, &call);
    return w2m_logged(W2M_API_WaitForMultipleObjects, n, ret);
}

/* Every way out (Sprint 350; Sprint 352): a count of 0 or past 64, a handle below the objects' and a full list stop the
 * run. A first try looks its handles up, each lookup holding a reference; a handle that names no object answers
 * WAIT_FAILED and ERROR_INVALID_HANDLE, as Microsoft Learn's WaitForMultipleObjects documents a failure ("WAIT_FAILED
 * (DWORD)0xFFFFFFFF: The function has failed. To get extended error information, call GetLastError."), the references
 * taken dropped and the deadline cleared. A retry reads its objects from its entries, not by its handles again, so a
 * handle closed during the wait leaves the wait on its object. For all, a grant and every object signaled (taken then)
 * and a timeout drop its entry or the lookups' references and clear the deadline (`timed_out_at` clears its own on a
 * timeout); a block keeps them, a first try's references entering a wait-all's entry. For any, likewise, with an entry
 * for each object, of any kind, holding its lookup's reference; only the handed-over kinds' entries are granted.
 * Sprint 356: the entries read under `sync_lock` and the lookups made under the table's lock, neither held with the
 * other; a wait whose objects are all this file's is `wait_any` or `wait_all`, one naming the host's `wait_any_held` or
 * `wait_all_held`. */
static u64 wait_multiple(u64 count, u64 handles, u64 all, u64 ms, int alertable, u64 rsp) {
    const u64 *hs = (const u64 *)handles;
    u32 n = (u32)count;
    if (n == 0 || n > 64) w2m_unimplemented(W2M_API_WaitForMultipleObjects, n);
    u32 me = current_tid();
    struct object *os[64];
    u32 got = n;
    int retry;
    {
        W2M_HELD(sync);                                        /* a retry: only this thread drops its entries */
        int a = (u32)all ? all_of(me) : -1;
        retry = (u32)all ? a >= 0 : waiter_of(me) >= 0;
        if (retry && (u32)all)
            for (u32 i = 0; i < n; i++) os[i] = &objects[all_waiters[a].objects[i]];
        else if (retry)
            got = entries_of(me, os);
    }
    if (got != n)
        w2m_unimplemented(W2M_API_WaitForMultipleObjects, n);    /* a retry with another count: not in the rows */
    for (u32 i = 0; !retry && i < n; i++) {
        os[i] = object_of(hs[i]);
        if (os[i]) continue;
        if (hs[i] < OBJECT_BASE) w2m_unimplemented(W2M_API_WaitForMultipleObjects, hs[i]);   /* not in the rows */
        objects_done(os, i);
        w2m_thread_scratch()->wait_deadline = 0;
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_WaitForMultipleObjects, n, WAIT_FAILED);
    }
    int held = 0;
    for (u32 i = 0; i < n; i++)
        if (!sync_kind(os[i])) held = 1;                       /* a kind fixed at creation: read with no lock */
    if (held && has_timer(os, n))       /* a timer beside a thread, a process or a shared event: not in the rows */
        w2m_unimplemented(W2M_API_WaitForMultipleObjects, n);
    if ((u32)all)
        return held ? wait_all_held(n, os, (u32)ms, retry, alertable, rsp) : wait_all(n, os, (u32)ms, alertable, rsp);
    return held ? wait_any_held(n, os, (u32)ms, retry, alertable, rsp) : wait_any(n, os, (u32)ms, retry, alertable, rsp);
}

u64 k32_WaitForMultipleObjects(u64 count, u64 handles, u64 all, u64 ms) {
    return wait_multiple(count, handles, all, ms, 0, 0);
}

/* For processes.c's WaitForMultipleObjectsEx(nCount, lpHandles, bWaitAll, dwMilliseconds, bAlertable), bAlertable on the
 * guest's stack at RSP + 0x28 (Sprint 370): "If this parameter is TRUE and the thread is in the waiting state, the
 * function returns when the system queues an I/O completion routine or APC, and the thread runs the routine or
 * function" (Microsoft Learn). */
u64 w2m_wait_multiple_ex(u64 count, u64 handles, u64 all, u64 ms, u64 rsp) {
    return wait_multiple(count, handles, all, ms, *(const u32 *)(rsp + 0x28) != 0, rsp);
}

/* Sleep: threads.c (Sprint 051), which lets other threads run while it waits. */

/* For processes.c (Sprint 055): a new handle on a new object; a handle's kind (0: no object) and `count`; its
 * flags; the inheritable handles; and the handles a parent passed, installed at their values on first use. */
u64 w2m_object_new(u64 api, u32 kind, s64 count, u32 flags) { return publish(api, kind, count, 0, flags); }
u32 w2m_object_kind(u64 h, s64 *count) {
    struct object *o = object_of(h);
    if (!o) return 0;
    *count = changes(o) ? 0 : o->count;       /* a semaphore's, an event's, a mutex's: no caller reads it */
    u32 kind = o->kind;
    object_done(o);
    return kind;
}
/* For file.c's GetHandleInformation and SetHandleInformation on a pipe's handle (Sprint 055): h's flags before the
 * change, and the mask's bits made `values`' (mask 0: read alone), both under the table's lock; 0 when h names no
 * object. Until Sprint 355 it answered a pointer into `handle_flags` that file.c read and wrote after the lookup
 * returned, with the slot's state no longer held. */
u32 w2m_object_flags(u64 h, u32 mask, u32 values) {
    W2M_HELD(handles);
    int i = slot_of(h);
    u32 before = i < 0 ? 0 : handle_flags[i];
    if (i >= 0) handle_flags[i] = (before & ~mask) | (values & mask);
    return before;
}
/* For processes.c's CreateProcessW: every inheritable slot, read under the table's lock, which a form not in the rows
 * lets go before it stops the run. */
int w2m_object_inheritable(u64 (*out)[3], int room) {
    int n = 0;
    W2M_HELD(handles);
    for (int i = 0; i < OBJECTS; i++)
        if (handle_object[i] && (handle_flags[i] & 1)) {
            if (n == room) {
                W2M_LET_GO_EARLY(handles);
                w2m_unimplemented(W2M_API_CreateProcessW, room);   /* more than the list holds */
            }
            struct object *o = &objects[handle_object[i] - 1];
            if (o->kind != SHARED_EVENT && o->kind != PIPE_READ && o->kind != PIPE_WRITE && o->kind != PROCESS) {
                W2M_LET_GO_EARLY(handles);
                w2m_unimplemented(W2M_API_CreateProcessW, o->kind);    /* inheriting such a handle: not in the rows */
            }
            out[n][0] = OBJECT_BASE + 4 * (u64)i;
            out[n][1] = o->kind;
            out[n][2] = (u64)o->count;
            n++;
        }
    return n;
}

/* The parent's handles installed at their values, once. The table held, by its caller's guard (`slot_of`'s and
 * `handle_for`'s callers'); a form not in the rows, or a full table, lets it go before it stops the run. */
static void inherit_init(void) {
    if (inherited_ready) return;
    inherited_ready = 1;
    const u64 *t = (const u64 *)W2M_INHERIT_VA;              /* count, the three standard handles, then entries */
    for (u64 k = 0; k < t[0]; k++) {
        u64 h = t[4 + 3 * k], kind = t[5 + 3 * k], id = t[6 + 3 * k];
        int i = (int)((h - OBJECT_BASE) / 4);
        if (h < OBJECT_BASE || (h - OBJECT_BASE) % 4 || i >= OBJECTS) {
            W2M_LET_GO_BEFORE_STOP(handles);
            w2m_unimplemented(W2M_API_CreateProcessW, h);
        }
        int o = new_object((u32)kind, (s64)id, 0);
        if (o < 0) {
            W2M_LET_GO_BEFORE_STOP(handles);
            w2m_unimplemented(W2M_API_CreateProcessW, OBJECTS);
        }
        handle_object[i] = o + 1;
        handle_flags[i] = 1;                                    /* inherited handles stay inheritable */
        objects[o].refs = objects[o].handles = 1;
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
u64 w2m_object_thread(u64 api, u32 tid) { return publish(api, THREAD, tid, 0, 0); }
int w2m_object_thread_of(u64 h, u32 *tid) {
    struct object *o = object_of(h);
    int thread = o && o->kind == THREAD;
    if (thread) *tid = (u32)o->count;
    object_done(o);
    return thread;
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
        h = source == CURRENT_THREAD ? publish(W2M_API_DuplicateHandle, THREAD, current_tid(), 0, 0)
                                     : publish(W2M_API_DuplicateHandle, PROCESS, current_pid(), 0, 0);
    } else {
        /* the source's lookup and the copy's slot under one holding, so no close between them frees its object */
        int i;
        {
            W2M_HELD(handles);
            i = slot_of(source);
            h = 0;
            if (i >= 0) h = options & DUPLICATE_CLOSE_SOURCE ? source     /* the copy keeps the slot (row dup.close_source) */
                                                             : handle_for(handle_object[i] - 1);
        }
        if (i < 0) {
            /* the file table and the standard handles are the runtime word's, which this call's bridge does not hold */
            int other;
            {
                W2M_HELD(runtime_word);
                other = w2m_is_file_handle(source) || w2m_is_std_handle(source);
            }
            if (other) w2m_unimplemented(W2M_API_DuplicateHandle, source);   /* a file's or the console's: not in the rows */
            *(u64 *)out = 0;
            set_error(ERROR_INVALID_HANDLE);
            return w2m_logged(W2M_API_DuplicateHandle, source, 0);
        }
        if (!h) w2m_unimplemented(W2M_API_DuplicateHandle, OBJECTS);
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
    return w2m_logged(W2M_API_OpenThread, tid, publish(W2M_API_OpenThread, THREAD, current_tid(), 0, 0));
}

u64 k32_GetThreadId(u64 h) {
    if (h == CURRENT_THREAD) return w2m_logged(W2M_API_GetThreadId, h, current_tid());
    struct object *o = must(W2M_API_GetThreadId, h, object_of(h), THREAD, THREAD);   /* else not in the rows */
    u64 tid = (u64)o->count;
    object_done(o);
    return w2m_logged(W2M_API_GetThreadId, h, tid);
}

u64 k32_GetProcessId(u64 h) {
    if (h == CURRENT_PROCESS) return w2m_logged(W2M_API_GetProcessId, h, current_pid());
    struct object *o = must(W2M_API_GetProcessId, h, object_of(h), PROCESS, PROCESS);   /* else not in the rows */
    u64 pid = (u64)o->count;
    object_done(o);
    return w2m_logged(W2M_API_GetProcessId, h, pid);
}

/* Named from Sprint 369 (probes/named_sync_objects, row semaphore): the name's object, made or found (`named`). */
u64 k32_CreateSemaphoreW(u64 attributes, u64 initial, u64 maximum, u64 name) {
    if (attributes || (s32)maximum <= 0 || (s32)initial < 0) w2m_unimplemented(W2M_API_CreateSemaphoreW, attributes);
    if ((s32)initial > (s32)maximum) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateSemaphoreW, initial, 0);
    }
    if (name)
        return w2m_logged(W2M_API_CreateSemaphoreW, SEMAPHORE,
                          named(W2M_API_CreateSemaphoreW, SEMAPHORE, name, 1, (s32)initial, (s32)maximum));
    return make(W2M_API_CreateSemaphoreW, SEMAPHORE, (s32)initial, (s32)maximum);
}

