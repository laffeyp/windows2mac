// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* heap.c — the process heap (Sprint 044, family "heap").
 *
 * GetProcessHeap, HeapAlloc, HeapFree, HeapSize, HeapReAlloc on the process heap, as
 * contracts/windows.heap.process/0.1.0.json records Windows answering on four runners (Server 2025
 * and 2022; AMD EPYC 7763, 9V74; Intel Xeon 8370C; store/probe-runs/380408*):
 *   - the handle equals PEB.ProcessHeap and is the same every call
 *   - blocks are 16-byte aligned; HeapSize is exactly the size asked for, 0 included
 *   - HEAP_ZERO_MEMORY zeroes a new block and a grown block's new bytes
 *   - failure (a request Windows refuses, HEAP_REALLOC_IN_PLACE_ONLY that cannot grow) answers NULL
 *     and sets the last error to ERROR_NOT_ENOUGH_MEMORY; success leaves it untouched
 *   - HeapFree(NULL) is TRUE
 *   - HEAP_REALLOC_IN_PLACE_ONLY grows a block to at most the larger of its size and 16 bytes
 *     (contracts/windows.heap.in_place, Sprint 065: game run 38216081440 seq 3265-3268, where w2m granted
 *     growths from a reused block's larger capacity); the block keeps that reach while it stays in place
 * The arena is W2M_HEAP_SIZE bytes at W2M_HEAP_VA (src/w2m/vm_backend.py), mapped sparse. A request
 * Windows would grant that the arena cannot hold stops the run instead of claiming no memory.
 *
 * The heap's own lock (Sprint 343; Change 1 of docs/design-rust-host-2026-10-24.md, "a lock per heap": "the heap
 * serializes access within the calling process", Microsoft Learn, HeapCreate): its five services take no runtime word,
 * so `hold_heap` and `let_heap_go` serialize the heap's shared state alone, the free list, the bump pointer and the
 * one-time set-up, in `take` and `give_back`. A block's header and payload belong to its holder, as Windows leaves a
 * block to its caller. Nothing is called while the lock is held, w2m_unimplemented and w2m_logged least of all, so the
 * lock never waits on another lock, and a service on the runtime word calls w2m_heap_take and w2m_heap_release without
 * an inversion. From Sprint 359 each take is a guard, W2M_HELD(heap) (runtime.h), whose scope's end lets it go.
 */
#include "runtime.h"

#define HEAP_NO_SERIALIZE 0x1u
#define HEAP_GENERATE_EXCEPTIONS 0x4u
#define HEAP_ZERO_MEMORY 0x8u
#define HEAP_REALLOC_IN_PLACE_ONLY 0x10u
#define ERROR_NOT_ENOUGH_MEMORY 8u
#define WINDOWS_REFUSES (1ull << 46)          /* row alloc.too_large: 2^46 bytes fail on every runner */
#define BLOCK_MAGIC 0x6B6F6C6250414548ull     /* "HEAPblok" */

struct header {                               /* 32 bytes before each payload */
    u64 size_and_magic_check;                 /* the size asked for */
    u64 capacity;                             /* payload bytes available, a multiple of 16 */
    u64 reach;                                /* how far HEAP_REALLOC_IN_PLACE_ONLY grows it (family "heap_in_place") */
    u64 unused;                               /* keeps the payload 16-byte aligned */
};

struct heap {                                 /* the handle points here, the arena's first page */
    u64 magic;
    u64 next;                                 /* bump pointer: the next unused byte */
    u64 free_list;                            /* first free block's header, linked through payload[0] */
    u32 lock;                                 /* 0 free, 1 held (Sprint 343); zero as the arena is mapped */
};

static int ready;

/* The heap held: its lock taken by compare-and-swap of 0 for 1, acquire, as lock_runtime takes the runtime word
 * (CASA), and its one-time set-up run under it, so two vCPUs never both run it and neither takes a block before it is
 * done. As lock_runtime does (monitor_code.rs 128-131), a waiter reads the word by plain loads until it reads 0 and
 * only then tries the swap, so it does not take the line from the holder writing `next` and `free_list` beside it
 * (reviewer-4's fourteenth review, Low 1). The lock word reads 1 for any holder: nothing reenters it, and the host
 * never reads it. Its answer, 1, is its guard's (W2M_HELD, runtime.h; Sprint 359). */
static int hold_heap(void) {
    struct heap *h = (struct heap *)W2M_HEAP_VA;
#ifdef W2M_TIME_LOCKS
    w2m_lock_take(&h->lock, W2M_LOCK_HEAP);              /* timed (Sprint 358, runtime.h) */
#else
    for (;;) {
        while (__atomic_load_n(&h->lock, __ATOMIC_RELAXED)) {}
        u32 free = 0;
        if (__atomic_compare_exchange_n(&h->lock, &free, 1, 0, __ATOMIC_ACQUIRE, __ATOMIC_RELAXED)) break;
    }
#endif
    if (!ready) {
        ready = 1;
        h->magic = BLOCK_MAGIC;
        h->next = W2M_HEAP_VA + 0x1000;
        const u8 *params = w2m_parameters();                       /* ProcessParameters, in the arena's first pages */
        if ((u64)params >= W2M_HEAP_VA && (u64)params < W2M_HEAP_VA + W2M_HEAP_SIZE)   /* (Sprint 059) */
            h->next = ((u64)params + *(const u32 *)params + 15) & ~15ull;
        h->free_list = 0;
        *(volatile u64 *)(W2M_PEB_VA + 0x30) = W2M_HEAP_VA;          /* PEB.ProcessHeap */
    }
    return 1;
}

/* The heap let go: a store of 0, release, as unlock_runtime lets the runtime word go (STLR). */
#ifdef W2M_TIME_LOCKS
static void let_heap_go(void) { w2m_lock_let_go(&((struct heap *)W2M_HEAP_VA)->lock, W2M_LOCK_HEAP); }

/* The four leaf locks' timing areas (Sprint 358): a symbol only the timed image defines, by which the host knows an
 * image's takes are timed (`guest::LOCKS_TIMED_SYMBOL`). */
u64 w2m_lock_timing_areas(void) { return W2M_LOCK_AREAS_VA; }
#else
static void let_heap_go(void) { __atomic_store_n(&((struct heap *)W2M_HEAP_VA)->lock, 0, __ATOMIC_RELEASE); }
#endif

/* The heap's guard's end (W2M_HELD, runtime.h; Sprint 359): its lock let go unless let go early. */
static void heap_guard_end(int *held) {
    if (*held) let_heap_go();
}

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

static void zero(u8 *p, u64 n) { for (u64 i = 0; i < n; i++) p[i] = 0; }

static void copy(u8 *to, const u8 *from, u64 n) { for (u64 i = 0; i < n; i++) to[i] = from[i]; }

static u64 round16(u64 n) { return (n + 15) & ~15ull; }

static void check_flags(u64 api, u64 flags, u64 allowed) {
    if (flags & ~allowed) w2m_unimplemented(api, flags);
}

static void check_handle(u64 api, u64 heap) {
    if (heap != W2M_HEAP_VA) w2m_unimplemented(api, heap);                /* HeapCreate's heaps: not yet */
}

/* A block of at least `size` bytes, or 0 when the arena is full; the heap held from the free list's walk to the bump
 * pointer's move and the new block's capacity, which no one else reaches until it is returned. Each return lets the
 * lock go by the guard's end (Sprint 359). */
static struct header *take(u64 size) {
    W2M_HELD(heap);
    struct heap *h = (struct heap *)W2M_HEAP_VA;
    u64 want = round16(size ? size : 1);
    u64 *link = &h->free_list;
    while (*link) {
        struct header *b = (struct header *)*link;
        if (b->capacity >= want) {
            *link = *(u64 *)(b + 1);
            return b;
        }
        link = (u64 *)(b + 1);
    }
    if (h->next + sizeof(struct header) + want > W2M_HEAP_VA + W2M_HEAP_SIZE) return 0;
    struct header *b = (struct header *)h->next;
    h->next += sizeof(struct header) + want;
    b->capacity = want;
    return b;
}

/* `b` linked at the free list's head, under the heap's lock. */
static void give_back(struct header *b) {
    W2M_HELD(heap);
    struct heap *h = (struct heap *)W2M_HEAP_VA;
    *(u64 *)(b + 1) = h->free_list;
    h->free_list = (u64)b;
}

static struct header *header_of(u64 api, u64 p) {
    if (p < W2M_HEAP_VA + 0x1000 || p >= W2M_HEAP_VA + W2M_HEAP_SIZE || p % 16)
        w2m_unimplemented(api, p);                                        /* not a block of this heap */
    return (struct header *)p - 1;
}

/* For the runtime's own calls that Windows serves from the process heap (GetEnvironmentStringsW's copy,
 * Sprint 044): a block of exactly `size` bytes, as HeapAlloc makes one, without a ring entry; 0 when the
 * arena is full. */
u64 w2m_heap_take(u64 size) {
    struct header *b = take(size);
    if (!b) return 0;
    b->size_and_magic_check = size;
    b->reach = size > 16 ? size : 16;
    return (u64)(b + 1);
}

void w2m_heap_release(u64 api, u64 p) { give_back(header_of(api, p)); }

u64 k32_GetProcessHeap(void) {
    {
        W2M_HELD(heap);                                                  /* the set-up, once */
    }
    return w2m_logged(W2M_API_GetProcessHeap, 0, W2M_HEAP_VA);
}

u64 k32_HeapAlloc(u64 heap, u64 flags, u64 size) {
    check_handle(W2M_API_HeapAlloc, heap);
    check_flags(W2M_API_HeapAlloc, (u32)flags, HEAP_NO_SERIALIZE | HEAP_ZERO_MEMORY);
    if (size >= WINDOWS_REFUSES) {
        set_error(ERROR_NOT_ENOUGH_MEMORY);
        return w2m_logged(W2M_API_HeapAlloc, size, 0);
    }
    struct header *b = take(size);
    if (!b) w2m_unimplemented(W2M_API_HeapAlloc, size);                  /* Windows would grant it */
    b->size_and_magic_check = size;
    b->reach = size > 16 ? size : 16;
    if (flags & HEAP_ZERO_MEMORY) zero((u8 *)(b + 1), b->capacity);
    return w2m_logged(W2M_API_HeapAlloc, size, (u64)(b + 1));
}

u64 k32_HeapFree(u64 heap, u64 flags, u64 p) {
    check_handle(W2M_API_HeapFree, heap);
    check_flags(W2M_API_HeapFree, (u32)flags, HEAP_NO_SERIALIZE);
    if (p) give_back(header_of(W2M_API_HeapFree, p));
    return w2m_logged(W2M_API_HeapFree, p, 1);
}

u64 k32_HeapSize(u64 heap, u64 flags, u64 p) {
    check_handle(W2M_API_HeapSize, heap);
    check_flags(W2M_API_HeapSize, (u32)flags, HEAP_NO_SERIALIZE);
    return w2m_logged(W2M_API_HeapSize, p, header_of(W2M_API_HeapSize, p)->size_and_magic_check);
}

u64 k32_HeapReAlloc(u64 heap, u64 flags, u64 p, u64 size) {
    check_handle(W2M_API_HeapReAlloc, heap);
    check_flags(W2M_API_HeapReAlloc, (u32)flags, HEAP_NO_SERIALIZE | HEAP_ZERO_MEMORY | HEAP_REALLOC_IN_PLACE_ONLY);
    struct header *b = header_of(W2M_API_HeapReAlloc, p);
    u64 old = b->size_and_magic_check;
    if (size >= WINDOWS_REFUSES) {
        set_error(ERROR_NOT_ENOUGH_MEMORY);
        return w2m_logged(W2M_API_HeapReAlloc, size, 0);
    }
    if ((flags & HEAP_REALLOC_IN_PLACE_ONLY) && size > b->reach) {      /* Windows refuses past the reach */
        set_error(ERROR_NOT_ENOUGH_MEMORY);
        return w2m_logged(W2M_API_HeapReAlloc, size, 0);
    }
    if (round16(size ? size : 1) <= b->capacity) {                       /* fits where it is */
        if ((flags & HEAP_ZERO_MEMORY) && size > old) zero((u8 *)p + old, size - old);
        b->size_and_magic_check = size;
        if (size > b->reach) b->reach = size;
        return w2m_logged(W2M_API_HeapReAlloc, size, p);
    }
    if (flags & HEAP_REALLOC_IN_PLACE_ONLY) {
        set_error(ERROR_NOT_ENOUGH_MEMORY);
        return w2m_logged(W2M_API_HeapReAlloc, size, 0);
    }
    struct header *n = take(size);
    if (!n) w2m_unimplemented(W2M_API_HeapReAlloc, size);
    n->size_and_magic_check = size;
    n->reach = size > 16 ? size : 16;
    copy((u8 *)(n + 1), (const u8 *)p, old);
    if (flags & HEAP_ZERO_MEMORY) zero((u8 *)(n + 1) + old, n->capacity - old);
    give_back(b);
    return w2m_logged(W2M_API_HeapReAlloc, size, (u64)(n + 1));
}
