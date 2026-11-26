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
            PIPE_WRITE = W2M_OBJECT_PIPE_WRITE, REMOTE_THREAD = W2M_OBJECT_REMOTE_THREAD };
            /* Sprint 055 (processes.c): a named event, a pipe's ends and another process's thread, each the host's
             * object (src/w2m/kernel.py) whose id `count` holds; a PROCESS's `count` is its id, this one or another */

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
static struct object { u32 kind; s64 count, max; u32 refs, handles, abandoned; } objects[OBJECTS];
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
 * word by plain loads until it reads 0 before it tries the swap. Nothing reenters it. */
static void hold_handles(void) {
    for (;;) {
        while (__atomic_load_n(&handles_lock, __ATOMIC_RELAXED)) {}
        u32 free = 0;
        if (__atomic_compare_exchange_n(&handles_lock, &free, 1, 0, __ATOMIC_ACQUIRE, __ATOMIC_RELAXED)) return;
    }
}

/* The table let go: a store of 0, release (STLR), as `let_slots_go`. */
static void let_handles_go(void) { __atomic_store_n(&handles_lock, 0, __ATOMIC_RELEASE); }

/* runtime.c (Sprint 355): the runtime word taken by a call whose bridge holds none, 1 when this call took it; and let
 * go when it did. A named event's EVENT_OPEN and DuplicateHandle's look at the file table and the standard handles take
 * it so, with no other lock held, and the waits and releases to park, to ready and for the host's objects (Sprint
 * 356). */
int w2m_take_runtime_word(void);
void w2m_let_runtime_word_go(int taken);

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }
static u32 current_tid(void);

static void inherit_init(void);

/* A kind whose state this file keeps (Sprint 356): a semaphore's, an event's and a mutex's, which change under
 * `sync_lock`, and a token's and a mapping's, fixed at creation (`signaled` reads a mapping's descriptor as a count, as
 * before). The rest are the host's (a thread, a process, a shared event) or answer no wait (a pipe's ends, another
 * process's thread). */
static int sync_kind(const struct object *o) {
    return o->kind == SEMAPHORE || o->kind == EVENT_MANUAL || o->kind == EVENT_AUTO || o->kind == MUTEX ||
           o->kind == TOKEN || o->kind == MAPPING;
}

/* A kind whose `count` and `max` change after creation, under `sync_lock`: a semaphore, an event, a mutex. Every other
 * kind's are fixed, a mapping's descriptor and size among them, and read with no lock by a holder of a reference. */
static int changes(const struct object *o) {
    return o->kind == SEMAPHORE || o->kind == EVENT_MANUAL || o->kind == EVENT_AUTO || o->kind == MUTEX;
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
    hold_handles();
    int o = new_object(kind, count, max);
    u64 h = o < 0 ? 0 : handle_for(o);
    if (h) handle_flags[(h - OBJECT_BASE) / 4] = flags;
    else if (o >= 0) objects[o].kind = FREE;
    let_handles_go();
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
    hold_handles();
    int i = slot_of(h);
    struct object *o = i < 0 ? 0 : &objects[handle_object[i] - 1];
    if (o) o->refs++;
    let_handles_go();
    return o;
}

static void object_done(struct object *o) {
    if (!o) return;
    hold_handles();
    unref(o);
    let_handles_go();
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
    hold_handles();
    int i = slot_of(h);
    if (i < 0) {
        let_handles_go();
        return 0;
    }
    struct object *o = &objects[handle_object[i] - 1];
    handle_object[i] = 0;
    int last = --o->handles == 0;
    u32 kind = o->kind;
    s64 id = changes(o) ? 0 : o->count;       /* a semaphore's, an event's, a mutex's: `sync_lock`'s, and unused here */
    unref(o);
    let_handles_go();
    if (last) {
        if (kind == MAPPING) w2m_host(W2M_FILE_CLOSE, (u64)id, 0, 0, 0, 0);       /* the views keep their copy */
        if (kind == SHARED_EVENT || kind == PIPE_READ || kind == PIPE_WRITE ||
            (kind == PROCESS && (u32)id != (u32)w2m_teb()->client_process))
            w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_CLOSE, (u64)id, 0, 0, 0);   /* the host's handle count */
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
    int took = w2m_take_runtime_word();             /* the transport page is every vCPU's; no other lock held */
    u64 id = w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_EVENT_OPEN, name, chars, (manual ? 1 : 0) | (initial ? 2 : 0), &existed);
    w2m_let_runtime_word_go(took);
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
 * another vCPU holds it. Nothing reenters it. */
static void hold_sync(void) {
    for (;;) {
        while (__atomic_load_n(&sync_lock, __ATOMIC_RELAXED)) {}
        u32 free = 0;
        if (__atomic_compare_exchange_n(&sync_lock, &free, 1, 0, __ATOMIC_ACQUIRE, __ATOMIC_RELAXED)) return;
    }
}

/* The state let go: a store of 0, release (STLR). */
static void let_sync_go(void) { __atomic_store_n(&sync_lock, 0, __ATOMIC_RELEASE); }

/* The runtime word as a wait or a release holds it, taken with no leaf held: `held` while this call holds it, `taken`
 * when this call took it (a traced call's bridge holds it already, as does a run on the host's clock, `services.rs`
 * `lock_in`, and this call then keeps it). */
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
    int took = w2m_take_runtime_word();
    w2m_block(deadline, key);
    w2m_let_runtime_word_go(took);
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
        hold_handles();
        u32 open = o->handles;
        let_handles_go();
        if (!open) return 0;
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
    hold_handles();
    for (int i = 0; i < OBJECTS; i++)
        if (objects[i].kind == MUTEX && objects[i].refs) {
            objects[i].refs++;
            held[n++] = i;
        }
    let_handles_go();
    struct readied r;                                  /* its tids read only below n: not zeroed */
    r.n = 0;
    hold_sync();
    for (int k = 0; k < n; k++) {
        struct object *o = &objects[held[k]];
        if (!o->count || (u32)o->max != tid) continue;
        o->count = 0;
        o->max = 0;
        o->abandoned = 1;
        grant(o, &r);
        grant_all(o, &r);
    }
    let_sync_go();
    struct word word = {0, 0};
    ready_all(&r, &word);
    let_word_go(&word);
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
u64 k32_SetEvent(u64 h) {
    struct object *o = object_of(h);
    struct word word = {0, 0};
    if (o && o->kind == SHARED_EVENT) {
        take_word(&word);
        w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_EVENT_SET, (u64)o->count, 0, 0, 0);
        w2m_signal(0);
    } else {
        must(W2M_API_SetEvent, h, o, EVENT_MANUAL, EVENT_AUTO);
        struct readied r;                              /* its tids read only below n: not zeroed */
        r.n = 0;
        hold_sync();
        o->count = 1;
        grant(o, &r);
        grant_all(o, &r);
        let_sync_go();
        ready_all(&r, &word);                        /* an auto-reset event's grants ready its waiters (095) */
        if (o->kind == EVENT_MANUAL) {
            take_word(&word);
            w2m_signal((u64)o);                      /* a manual-reset event releases every waiter */
        }
    }
    let_word_go(&word);
    object_done(o);
    return w2m_logged(W2M_API_SetEvent, h, 1);
}
u64 k32_ResetEvent(u64 h) {
    struct object *o = object_of(h);
    if (o && o->kind == SHARED_EVENT) {
        int took = w2m_take_runtime_word();
        w2m_host(W2M_HOST_KERNEL, W2M_KERNEL_EVENT_RESET, (u64)o->count, 0, 0, 0);
        w2m_let_runtime_word_go(took);
    } else {
        must(W2M_API_ResetEvent, h, o, EVENT_MANUAL, EVENT_AUTO);
        hold_sync();
        o->count = 0;
        let_sync_go();
    }
    object_done(o);
    return w2m_logged(W2M_API_ResetEvent, h, 1);
}

u64 k32_ReleaseSemaphore(u64 h, u64 count, u64 previous) {
    struct object *o = must(W2M_API_ReleaseSemaphore, h, object_of(h), SEMAPHORE, SEMAPHORE);
    if ((s32)count <= 0) w2m_unimplemented(W2M_API_ReleaseSemaphore, count);
    struct readied r;                              /* its tids read only below n: not zeroed */
    r.n = 0;
    hold_sync();
    if (o->count + (s32)count > o->max) {
        let_sync_go();
        object_done(o);
        set_error(ERROR_TOO_MANY_POSTS);
        return w2m_logged(W2M_API_ReleaseSemaphore, h, 0);
    }
    s32 before = (s32)o->count;
    o->count += (s32)count;
    grant(o, &r);
    grant_all(o, &r);
    let_sync_go();
    if (previous) *(s32 *)previous = before;
    struct word word = {0, 0};
    ready_all(&r, &word);
    let_word_go(&word);
    object_done(o);
    return w2m_logged(W2M_API_ReleaseSemaphore, h, 1);
}

u64 k32_ReleaseMutex(u64 h) {
    struct object *o = must(W2M_API_ReleaseMutex, h, object_of(h), MUTEX, MUTEX);
    u32 me = current_tid();
    struct readied r;                              /* its tids read only below n: not zeroed */
    r.n = 0;
    hold_sync();
    if (o->count == 0 || (u32)o->max != me) {
        let_sync_go();
        object_done(o);
        set_error(ERROR_NOT_OWNER);
        return w2m_logged(W2M_API_ReleaseMutex, h, 0);
    }
    if (--o->count == 0) {
        o->max = 0;
        grant(o, &r);
        grant_all(o, &r);
    }
    let_sync_go();
    struct word word = {0, 0};
    ready_all(&r, &word);
    let_word_go(&word);
    object_done(o);
    return w2m_logged(W2M_API_ReleaseMutex, h, 1);
}

/* Not signaled: 1 when the wait is over (WAIT_TIMEOUT now, the deadline cleared); 0 when the thread parks until
 * `*deadline` and retries the call (runtime.h W2M_SCHEDULE_RETRY), other threads running meanwhile; -1 when the counter
 * is needed first (`now`, `have_now`), read with no leaf held, as it is a host request on the host's clock. ms is from
 * the first try, the deadline kept in the TEB, the waiting thread's own (only it and its vCPU's switch read it). */
static int timed_out_at(u32 ms, int have_now, u64 now, u64 *deadline) {
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    if (ms == 0) return 1;
    if (!sc->wait_deadline) {
        if (ms == INFINITE) {
            *deadline = ~0ull;
            return 0;
        }
        if (!have_now) return -1;
        *deadline = now + (u64)ms * 10000;
        return 0;
    }
    if (sc->wait_deadline != ~0ull) {
        if (!have_now) return -1;
        if (now >= sc->wait_deadline) {
            sc->wait_deadline = 0;
            return 1;
        }
    }
    *deadline = sc->wait_deadline;
    return 0;
}

/* `timed_out_at` with the counter read when it is needed, no leaf held. */
static int timed_out_now(u32 ms, u64 *deadline) {
    int t = timed_out_at(ms, 0, 0, deadline);
    return t < 0 ? timed_out_at(ms, 1, w2m_counter_now(), deadline) : t;
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
static u64 wait_one_held(u64 api, u64 h, struct object *o, u32 ms, int retry);

static u64 wait_one(u64 api, u64 h, u32 ms) {
    u32 me = current_tid();
    hold_sync();
    int w = waiter_of(me);
    struct object *o = w >= 0 ? &objects[waiters[w].object] : 0;  /* a retry: only this thread drops its entry */
    let_sync_go();
    int retry = o != 0;
    if (!retry) o = object_of(h);
    if (!o) {
        if (h < OBJECT_BASE) w2m_unimplemented(api, h);         /* a file, a process, a pseudo-handle: not in the rows */
        w2m_thread_scratch()->wait_deadline = 0;
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(api, h, WAIT_FAILED);
    }
    if (!sync_kind(o)) return wait_one_held(api, h, o, ms, retry);
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    u64 ret = WAIT_OBJECT_0, now = 0, deadline = 0;
    int have_now = 0;
    for (;;) {
        hold_sync();
        w = waiter_of(me);
        if (w >= 0 && waiters[w].granted) {                    /* a release handed it over while this thread waited */
            sc->wait_deadline = 0;
            if (waiters[w].granted == ABANDONED_GRANT) ret = WAIT_ABANDONED;   /* its owner ended (amendment) */
        } else if (!sync_signaled(o, me)) {
            int t = timed_out_at(ms, have_now, now, &deadline);
            if (t < 0) {
                let_sync_go();
                now = w2m_counter_now();
                have_now = 1;
                continue;
            }
            if (!t) {                                          /* retried: the bridge does not return */
                if (w < 0) {
                    if (nwaiters == WAITERS) {
                        let_sync_go();
                        w2m_unimplemented(api, WAITERS);
                    }
                    waiter_add(me, o, -1);                     /* the lookup's reference, kept */
                }
                w2m_wait_armed();
                let_sync_go();
                park(deadline, (u64)o);
                return 0;
            }
            ret = WAIT_TIMEOUT;
        } else {
            sc->wait_deadline = 0;
            if (sync_take(o, me)) ret = WAIT_ABANDONED;         /* its owner ended holding it (amendment) */
        }
        if (w >= 0) waiters_out(me);
        let_sync_go();
        break;
    }
    object_done(o);                                            /* the entry's reference, or the lookup's */
    return w2m_logged(api, h, ret);
}

/* A wait on one of the host's objects (Sprint 356): the runtime word held from its check, a host request, to its park,
 * so no readying of the host's, nor this process's signal of a shared event, falls between them; its entry, never
 * granted, made and taken out under `sync_lock`, taken under the word as the order allows. */
static u64 wait_one_held(u64 api, u64 h, struct object *o, u32 ms, int retry) {
    struct word word = {0, 0};
    take_word(&word);
    u64 ret = WAIT_OBJECT_0, deadline = 0;
    if (!host_signaled(o)) {
        if (!timed_out_now(ms, &deadline)) {                  /* retried: the bridge does not return */
            if (!retry) {
                hold_sync();
                if (nwaiters == WAITERS) {
                    let_sync_go();
                    w2m_unimplemented(api, WAITERS);
                }
                waiter_add(current_tid(), o, -1);              /* the lookup's reference, kept */
                let_sync_go();
            }
            w2m_block(deadline, (u64)o);
            let_word_go(&word);
            return 0;
        }
        ret = WAIT_TIMEOUT;
    } else {
        w2m_thread_scratch()->wait_deadline = 0;
        host_take(o);
    }
    if (retry) {
        hold_sync();
        waiters_out(current_tid());
        let_sync_go();
    }
    let_word_go(&word);
    object_done(o);
    return w2m_logged(api, h, ret);
}

u64 k32_WaitForSingleObject(u64 h, u64 ms) { return wait_one(W2M_API_WaitForSingleObject, h, (u32)ms); }

u64 k32_WaitForSingleObjectEx(u64 h, u64 ms, u64 alertable) {
    (void)alertable;                                           /* no APC is ever queued: one thread */
    return wait_one(W2M_API_WaitForSingleObjectEx, h, (u32)ms);
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
static u64 wait_any(u32 n, struct object **os, u32 ms, int retry) {
    u32 me = current_tid();
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    u64 ret, now = 0, deadline = 0;
    int have_now = 0;
    for (;;) {
        hold_sync();
        ret = granted_slot(me);                                /* a release handed one over while this thread waited */
        if (ret != WAIT_TIMEOUT) sc->wait_deadline = 0;
        for (u32 i = 0; ret == WAIT_TIMEOUT && i < n; i++)
            if (sync_signaled(os[i], me)) {
                sc->wait_deadline = 0;
                ret = (sync_take(os[i], me) ? WAIT_ABANDONED : WAIT_OBJECT_0) + i;   /* abandoned: the amendment */
            }
        if (ret == WAIT_TIMEOUT) {
            int t = timed_out_at(ms, have_now, now, &deadline);
            if (t < 0) {
                let_sync_go();
                now = w2m_counter_now();
                have_now = 1;
                continue;
            }
            if (!t) {                                          /* retried: the bridge does not return */
                if (!retry) {
                    if (nwaiters + (int)n > WAITERS) {
                        let_sync_go();
                        w2m_unimplemented(W2M_API_WaitForMultipleObjects, WAITERS);
                    }
                    for (u32 i = 0; i < n; i++) waiter_add(me, os[i], (int)i);   /* the lookups' references, kept */
                }
                w2m_wait_armed();
                let_sync_go();
                park(deadline, 0);
                return 0;
            }
        }
        if (retry) waiters_out(me);
        let_sync_go();
        break;
    }
    objects_done(os, n);
    return w2m_logged(W2M_API_WaitForMultipleObjects, n, ret);
}

/* A wait for any of n objects one of which is the host's (Sprint 356): the runtime word held from the check to the
 * park. The check runs in today's order, a grant first and then each object to the first signaled one, the host asked
 * only up to it, an object of this file's read in a holding of `sync_lock` of its own; then one holding decides: a
 * grant, else the first object signaled now of this file's or the host's one found so, taken, else the timeout or the
 * park, its entries made. A release between the check and that holding is seen by it; one after it finds the entries,
 * and its readying waits for the word, so the park is never passed over and no mark is needed. */
static u64 wait_any_held(u32 n, struct object **os, u32 ms, int retry) {
    u32 me = current_tid();
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    struct word word = {0, 0};
    take_word(&word);
    hold_sync();
    u64 granted = granted_slot(me);
    int host = -1;
    let_sync_go();
    for (u32 i = 0; granted == WAIT_TIMEOUT && i < n; i++) {
        if (sync_kind(os[i])) {
            hold_sync();
            int s = sync_signaled(os[i], me);
            let_sync_go();
            if (s) break;
        } else if (host_signaled(os[i])) {
            host = (int)i;
            break;
        }
    }
    u64 ret, now = 0, deadline = 0;
    int have_now = 0;
    for (;;) {
        hold_sync();
        ret = granted_slot(me);
        for (u32 i = 0; ret == WAIT_TIMEOUT && i < n; i++)
            if ((int)i == host || (sync_kind(os[i]) && sync_signaled(os[i], me)))
                ret = ((int)i != host && sync_take(os[i], me) ? WAIT_ABANDONED : WAIT_OBJECT_0) + i;
        if (ret == WAIT_TIMEOUT) {
            int t = timed_out_at(ms, have_now, now, &deadline);
            if (t < 0) {
                let_sync_go();
                now = w2m_counter_now();
                have_now = 1;
                continue;
            }
            if (!t) {                                          /* retried: the bridge does not return */
                if (!retry) {
                    if (nwaiters + (int)n > WAITERS) {
                        let_sync_go();
                        w2m_unimplemented(W2M_API_WaitForMultipleObjects, WAITERS);
                    }
                    for (u32 i = 0; i < n; i++) waiter_add(me, os[i], (int)i);   /* the lookups' references, kept */
                }
                let_sync_go();
                w2m_block(deadline, 0);
                let_word_go(&word);
                return 0;
            }
        } else {
            sc->wait_deadline = 0;
        }
        if (retry) waiters_out(me);
        let_sync_go();
        break;
    }
    if (host >= 0 && ret == WAIT_OBJECT_0 + (u32)host) host_take(os[host]);
    let_word_go(&word);
    objects_done(os, n);
    return w2m_logged(W2M_API_WaitForMultipleObjects, n, ret);
}

/* A wait for all of n objects of this file's (Sprint 356): its grant (a release took them for it, Sprint 095), every
 * object signaled and taken, its timeout, or its entry made and its mark armed, in one holding of `sync_lock`. */
static u64 wait_all(u32 n, struct object **os, u32 ms) {
    u32 me = current_tid();
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    u64 ret, now = 0, deadline = 0;
    int have_now = 0;
    for (;;) {
        hold_sync();
        int a = all_of(me);
        ret = WAIT_OBJECT_0;
        if (a >= 0 && all_waiters[a].granted) {
            sc->wait_deadline = 0;
            ret = (u64)(all_waiters[a].granted - 1);           /* take_every's answer, kept + 1 (grant_all) */
        } else {
            u32 i = 0;
            while (i < n && sync_signaled(os[i], me)) i++;
            if (i < n) {
                int t = timed_out_at(ms, have_now, now, &deadline);
                if (t < 0) {
                    let_sync_go();
                    now = w2m_counter_now();
                    have_now = 1;
                    continue;
                }
                if (!t) {                                      /* retried: the bridge does not return */
                    if (a < 0) {
                        if (nall == ALL_WAITERS) {
                            let_sync_go();
                            w2m_unimplemented(W2M_API_WaitForMultipleObjects, ALL_WAITERS);
                        }
                        all_waiters[nall] = (struct all_waiter){me, n, 0, 0, {0}};
                        for (u32 k = 0; k < n; k++)            /* the lookups' references, kept */
                            all_waiters[nall].objects[k] = (int)(os[k] - objects);
                        nall++;
                    }
                    w2m_wait_armed();
                    let_sync_go();
                    park(deadline, 0);
                    return 0;
                }
                ret = WAIT_TIMEOUT;
            } else {
                sc->wait_deadline = 0;
                ret = take_every(n, 0, os, me);
            }
        }
        if (a >= 0) all_out(a);
        let_sync_go();
        break;
    }
    objects_done(os, n);
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
static u64 wait_all_held(u32 n, struct object **os, u32 ms, int retry) {
    u32 me = current_tid();
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    struct word word = {0, 0};
    take_word(&word);
    u32 k = 0;
    for (; k < n; k++) {
        int s;
        if (sync_kind(os[k])) {
            hold_sync();
            s = sync_signaled(os[k], me);
            let_sync_go();
        } else {
            s = host_signaled(os[k]);
        }
        if (!s) break;
    }
    u64 ret, now = 0, deadline = 0;
    int have_now = 0;
    for (;;) {
        hold_sync();
        int a = all_of(me), every = k == n;
        for (u32 i = 0; every && i < n; i++)
            if (sync_kind(os[i]) && !sync_signaled(os[i], me)) every = 0;
        if (every) {
            ret = take_every(n, 0, os, me);
            sc->wait_deadline = 0;
        } else {
            int t = timed_out_at(ms, have_now, now, &deadline);
            if (t < 0) {
                let_sync_go();
                now = w2m_counter_now();
                have_now = 1;
                continue;
            }
            if (!t) {                                          /* retried: the bridge does not return */
                if (!retry) {
                    if (nall == ALL_WAITERS) {
                        let_sync_go();
                        w2m_unimplemented(W2M_API_WaitForMultipleObjects, ALL_WAITERS);
                    }
                    all_waiters[nall] = (struct all_waiter){me, n, 0, 1, {0}};
                    for (u32 i = 0; i < n; i++)                /* the lookups' references, kept */
                        all_waiters[nall].objects[i] = (int)(os[i] - objects);
                    nall++;
                }
                int again = k < n && sync_kind(os[k]) && sync_signaled(os[k], me);
                let_sync_go();
                if (again) w2m_wait_again();
                w2m_block(deadline, 0);
                let_word_go(&word);
                return 0;
            }
            ret = WAIT_TIMEOUT;
        }
        if (a >= 0) all_out(a);
        let_sync_go();
        break;
    }
    if (ret != WAIT_TIMEOUT)                                   /* WAIT_OBJECT_0, or WAIT_ABANDONED_0 + k */
        for (u32 i = 0; i < n; i++)
            if (!sync_kind(os[i])) host_take(os[i]);
    let_word_go(&word);
    objects_done(os, n);
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
u64 k32_WaitForMultipleObjects(u64 count, u64 handles, u64 all, u64 ms) {
    const u64 *hs = (const u64 *)handles;
    u32 n = (u32)count;
    if (n == 0 || n > 64) w2m_unimplemented(W2M_API_WaitForMultipleObjects, n);
    u32 me = current_tid();
    struct object *os[64];
    u32 got = n;
    hold_sync();                                               /* a retry: only this thread drops its entries */
    int a = (u32)all ? all_of(me) : -1;
    int retry = (u32)all ? a >= 0 : waiter_of(me) >= 0;
    if (retry && (u32)all)
        for (u32 i = 0; i < n; i++) os[i] = &objects[all_waiters[a].objects[i]];
    else if (retry)
        got = entries_of(me, os);
    let_sync_go();
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
    if ((u32)all) return held ? wait_all_held(n, os, (u32)ms, retry) : wait_all(n, os, (u32)ms);
    return held ? wait_any_held(n, os, (u32)ms, retry) : wait_any(n, os, (u32)ms, retry);
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
    hold_handles();
    int i = slot_of(h);
    u32 before = i < 0 ? 0 : handle_flags[i];
    if (i >= 0) handle_flags[i] = (before & ~mask) | (values & mask);
    let_handles_go();
    return before;
}
/* For processes.c's CreateProcessW: every inheritable slot, read under the table's lock, which a form not in the rows
 * lets go before it stops the run. */
int w2m_object_inheritable(u64 (*out)[3], int room) {
    int n = 0;
    hold_handles();
    for (int i = 0; i < OBJECTS; i++)
        if (handle_object[i] && (handle_flags[i] & 1)) {
            if (n == room) {
                let_handles_go();
                w2m_unimplemented(W2M_API_CreateProcessW, room);   /* more than the list holds */
            }
            struct object *o = &objects[handle_object[i] - 1];
            if (o->kind != SHARED_EVENT && o->kind != PIPE_READ && o->kind != PIPE_WRITE && o->kind != PROCESS) {
                let_handles_go();
                w2m_unimplemented(W2M_API_CreateProcessW, o->kind);    /* inheriting such a handle: not in the rows */
            }
            out[n][0] = OBJECT_BASE + 4 * (u64)i;
            out[n][1] = o->kind;
            out[n][2] = (u64)o->count;
            n++;
        }
    let_handles_go();
    return n;
}

/* The parent's handles installed at their values, once. The table held; a form not in the rows, or a full table, lets
 * it go before it stops the run. */
static void inherit_init(void) {
    if (inherited_ready) return;
    inherited_ready = 1;
    const u64 *t = (const u64 *)W2M_INHERIT_VA;              /* count, the three standard handles, then entries */
    for (u64 k = 0; k < t[0]; k++) {
        u64 h = t[4 + 3 * k], kind = t[5 + 3 * k], id = t[6 + 3 * k];
        int i = (int)((h - OBJECT_BASE) / 4);
        if (h < OBJECT_BASE || (h - OBJECT_BASE) % 4 || i >= OBJECTS) {
            let_handles_go();
            w2m_unimplemented(W2M_API_CreateProcessW, h);
        }
        int o = new_object((u32)kind, (s64)id, 0);
        if (o < 0) {
            let_handles_go();
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
        hold_handles();
        int i = slot_of(source);
        h = 0;
        if (i >= 0) h = options & DUPLICATE_CLOSE_SOURCE ? source         /* the copy keeps the slot (row dup.close_source) */
                                                         : handle_for(handle_object[i] - 1);
        let_handles_go();
        if (i < 0) {
            /* the file table and the standard handles are the runtime word's, which this call's bridge does not hold */
            int took = w2m_take_runtime_word();
            int other = w2m_is_file_handle(source) || w2m_is_std_handle(source);
            w2m_let_runtime_word_go(took);
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

u64 k32_CreateSemaphoreW(u64 attributes, u64 initial, u64 maximum, u64 name) {
    if (attributes || name || (s32)maximum <= 0 || (s32)initial < 0) w2m_unimplemented(W2M_API_CreateSemaphoreW, name);
    if ((s32)initial > (s32)maximum) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateSemaphoreW, initial, 0);
    }
    return make(W2M_API_CreateSemaphoreW, SEMAPHORE, (s32)initial, (s32)maximum);
}

