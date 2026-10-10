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
 * The arena is W2M_HEAP_SIZE bytes at W2M_HEAP_VA (src/w2m/vm_backend.py), mapped sparse. A request
 * Windows would grant that the arena cannot hold stops the run instead of claiming no memory.
 */
#include "runtime.h"

#define HEAP_NO_SERIALIZE 0x1u
#define HEAP_GENERATE_EXCEPTIONS 0x4u
#define HEAP_ZERO_MEMORY 0x8u
#define HEAP_REALLOC_IN_PLACE_ONLY 0x10u
#define ERROR_NOT_ENOUGH_MEMORY 8u
#define WINDOWS_REFUSES (1ull << 46)          /* row alloc.too_large: 2^46 bytes fail on every runner */
#define BLOCK_MAGIC 0x6B6F6C6250414548ull     /* "HEAPblok" */

struct header {                               /* 16 bytes before each payload */
    u64 size_and_magic_check;                 /* the size asked for */
    u64 capacity;                             /* payload bytes available, a multiple of 16 */
};

struct heap {                                 /* the handle points here, the arena's first page */
    u64 magic;
    u64 next;                                 /* bump pointer: the next unused byte */
    u64 free_list;                            /* first free block's header, linked through payload[0] */
};

static int ready;

static struct heap *the_heap(void) {
    struct heap *h = (struct heap *)W2M_HEAP_VA;
    if (!ready) {
        ready = 1;
        h->magic = BLOCK_MAGIC;
        h->next = W2M_HEAP_VA + 0x1000;
        h->free_list = 0;
        *(volatile u64 *)(W2M_PEB_VA + 0x30) = W2M_HEAP_VA;          /* PEB.ProcessHeap */
    }
    return h;
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

/* A block of at least `size` bytes, or 0 when the arena is full. */
static struct header *take(u64 size) {
    struct heap *h = the_heap();
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

static void give_back(struct header *b) {
    struct heap *h = the_heap();
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
    return (u64)(b + 1);
}

void w2m_heap_release(u64 api, u64 p) { give_back(header_of(api, p)); }

u64 k32_GetProcessHeap(void) {
    return w2m_logged(W2M_API_GetProcessHeap, 0, (u64)the_heap());
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
    if (round16(size ? size : 1) <= b->capacity) {                       /* fits where it is */
        if ((flags & HEAP_ZERO_MEMORY) && size > old) zero((u8 *)p + old, size - old);
        b->size_and_magic_check = size;
        return w2m_logged(W2M_API_HeapReAlloc, size, p);
    }
    if (flags & HEAP_REALLOC_IN_PLACE_ONLY) {
        set_error(ERROR_NOT_ENOUGH_MEMORY);
        return w2m_logged(W2M_API_HeapReAlloc, size, 0);
    }
    struct header *n = take(size);
    if (!n) w2m_unimplemented(W2M_API_HeapReAlloc, size);
    n->size_and_magic_check = size;
    copy((u8 *)(n + 1), (const u8 *)p, old);
    if (flags & HEAP_ZERO_MEMORY) zero((u8 *)(n + 1) + old, n->capacity - old);
    give_back(b);
    return w2m_logged(W2M_API_HeapReAlloc, size, (u64)(n + 1));
}
