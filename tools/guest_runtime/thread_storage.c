// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* thread_storage.c — TLS and FLS (Sprint 044, family "thread storage"; FLS per thread, Sprint 061).
 *
 * TlsAlloc, TlsGetValue, TlsSetValue, TlsFree, FlsAlloc, FlsGetValue, FlsSetValue, FlsFree, as
 * contracts/windows.thread_storage.single_thread/0.1.0.json records Windows answering on twelve
 * runs (Server 2025 and 2022; AMD EPYC 7763, 9V45; Intel Xeon 8370C):
 *   - TLS indices are the lowest free ones; a fresh process already holds index 0 and TAKEN_TLS
 *   - a TLS value lives in TEB.TlsSlots (+0x1480) below 64 and in the 1024-entry array that
 *     TEB.TlsExpansionSlots (+0x1780) points to above (Mono reads gs:[0x1480 + key*8] itself)
 *   - TlsGetValue clears the last error on success; an index past 1088: NULL and
 *     ERROR_INVALID_PARAMETER; TlsSetValue and TlsFree leave the last error on success
 *   - FLS indices start at 1; FlsGetValue before the thread's first FlsSetValue answers NULL and
 *     ERROR_INVALID_PARAMETER even for an allocated index (the thread's FLS data is made on the
 *     first set), afterwards it clears the last error
 *   - FlsFree runs the index's callback with the value; w2m cannot yet call back into guest code,
 *     so a free that would run one stops the run
 * A thread's FLS values are its own (Sprint 061): a block the thread's first FlsSetValue makes, behind TEB.FlsData
 * (+0x17C8, Geoff Chappell's TEB layout), so a new thread's FlsGetValue answers NULL and 87 as the rows say; the
 * blocks are listed so FlsFree clears the index on every thread. Before, one set of values served every thread, and
 * the game's new threads read the C runtime's per-thread data of the first (game run 38203364352, Sprint 060).
 * TlsGetValue is the game's second-hottest import (23,291,261 calls in 180 s): it never leaves the guest.
 *
 * The slots' own lock (Sprint 345; Change 1 of docs/design-rust-host-2026-10-24.md, "the TLS and FLS slots' lock"):
 * TlsAlloc, TlsFree, FlsAlloc, FlsFree, FlsSetValue, FlsGetValue and FlsGetValue2 take no runtime word, so
 * `hold_slots` and `let_slots_go` serialize the process's slot state alone: the bitmaps `tls_used` and `fls_used`, the
 * callbacks, the list of every thread's FLS block and each block's values, and the TLS set-up, which runs under the
 * lock. The heap's lock is taken under it (a thread's first FlsSetValue, `w2m_heap_take`), slots then heap, as the
 * design's order has it; the lock is let go before every w2m_unimplemented and w2m_logged, so it never waits on the
 * runtime word. TlsGetValue, TlsSetValue and TlsGetValue2 read and write the calling thread's TEB and take no lock.
 * TlsAlloc and TlsFree make their index 0 in every live thread's TEB or expansion array, under the slots' lock (Sprint
 * 347, `tls_clear_everywhere`); before, they cleared the caller's alone, and another thread read its old value.
 */
#include "runtime.h"

#define ERROR_INVALID_PARAMETER 87u
#define TLS_OUT_OF_INDEXES 0xFFFFFFFFull
#define TLS_SLOTS 64u
#define TLS_EXPANSION 1024u
#define TLS_INDICES (TLS_SLOTS + TLS_EXPANSION)
#define TEB_TLS_SLOTS 0x1480u
#define TEB_TLS_EXPANSION 0x1780u
#define FLS_INDICES 128u

static u8 tls_used[TLS_INDICES];
#define TEB_FLS_DATA 0x17C8u
struct fls_block { struct fls_block *next; u64 value[FLS_INDICES]; };
static struct fls_block *fls_blocks;              /* every thread's block, for FlsFree */
static u64 fls_callback[FLS_INDICES];
static u8 fls_used[FLS_INDICES];
static u32 slots_lock;                            /* 0 free, 1 held (Sprint 345); in the image's zeroed data */
static int tls_ready;

/* The TLS set-up, once: the two indices a fresh process holds. Run only with the slots held, so no thread finds
 * `tls_ready` set before the indices are. */
static void tls_init(void) {
    if (tls_ready) return;
    tls_ready = 1;
    tls_used[0] = 1;                              /* held in a fresh process (row tls.alloc.first) */
    tls_used[W2M_TLS_TAKEN] = 1;                  /* the other one (row tls.alloc.many.indices) */
}

/* The slots held: compare-and-swap of 0 for 1, acquire, as heap.c's `hold` and lock_runtime (CASA), a waiter reading
 * the word by plain loads until it reads 0 before it tries the swap; then the TLS set-up. Nothing reenters it. */
static void hold_slots(void) {
    for (;;) {
        while (__atomic_load_n(&slots_lock, __ATOMIC_RELAXED)) {}
        u32 free = 0;
        if (__atomic_compare_exchange_n(&slots_lock, &free, 1, 0, __ATOMIC_ACQUIRE, __ATOMIC_RELAXED)) break;
    }
    tls_init();
}

/* The slots let go: a store of 0, release (STLR), as heap.c's `let_go`. */
static void let_slots_go(void) { __atomic_store_n(&slots_lock, 0, __ATOMIC_RELEASE); }

/* The calling thread's FLS block; `make`: create it (the thread's first FlsSetValue). Called with the slots held; a
 * full arena lets them go before it stops the run. */
static struct fls_block *fls_mine(int make) {
    struct fls_block **slot = (struct fls_block **)((u8 *)w2m_teb() + TEB_FLS_DATA);
    if (!*slot && make) {
        struct fls_block *b = (struct fls_block *)w2m_heap_take(sizeof *b);
        if (!b) {
            let_slots_go();
            w2m_unimplemented(W2M_API_FlsSetValue, sizeof *b);   /* the heap's arena is full */
        }
        u8 *bytes = (u8 *)b;
        for (u64 i = 0; i < sizeof *b; i++) bytes[i] = 0;
        b->next = fls_blocks;
        fls_blocks = b;
        *slot = b;
    }
    return *slot;
}

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

/* The slot of `index` in the TEB at `teb`: TlsSlots below 64, above them the array TlsExpansionSlots points to; none
 * while it points to none. */
static u64 *tls_slot_in(u8 *teb, u64 index) {
    if (index < TLS_SLOTS) return (u64 *)(teb + TEB_TLS_SLOTS) + index;
    u64 *expansion = *(u64 **)(teb + TEB_TLS_EXPANSION);
    return expansion ? expansion + (index - TLS_SLOTS) : 0;
}

static u64 *tls_slot(u64 index) { return tls_slot_in((u8 *)w2m_teb(), index); }

/* The slot of `index` in the TEB at `teb`, its expansion array given first when it has none (Sprint 347's amendment;
 * Microsoft Learn, TlsAlloc: "After a TLS index has been allocated, each thread of the process can use it to access
 * its own TLS storage slot"). Every thread but the first has its own array from its creation, "`(0x1780, teb +
 * 0x2000),`" (crates/w2m_vm/src/threads.rs 1249); the first thread's TEB, W2M_TEB_VA (run.rs 958), has none from the
 * host (process.rs 372-383), and it takes the process's array, W2M_TLS_EXPANSION_VA, unused till then and so zero.
 * Before, only its own TlsAlloc past 63 gave it, and its TlsSetValue of an index another thread took stopped the run
 * (probes/tls_expansion_first). Called with the slots held, for every live thread whenever an index is allocated or
 * freed, so once any index past 63 is taken every live thread has an array. */
static u64 *tls_slot_given(u8 *teb, u64 index) {
    u64 *expansion = (u64 *)(teb + TEB_TLS_EXPANSION);
    if (index >= TLS_SLOTS && !*expansion && teb == (u8 *)W2M_TEB_VA) *expansion = W2M_TLS_EXPANSION_VA;
    return tls_slot_in(teb, index);
}

/* The run table's head and each entry's first eight words, as threads.c's `struct w2m_run_table` and
 * `struct w2m_run_entry` lay them out (crates/w2m_vm/src/threads.rs RT_COUNT 0x00, E_STATUS 0x08, E_TEB 0x20): the
 * count of threads the process has made, and each one's status and TEB. */
struct run_head { u64 tid, status, suspend, deadline, teb, vslot, instructions, pauses; };
_Static_assert(sizeof(struct run_head) == W2M_RUN_X_OFF, "a run entry's words before its registers");
#define RUN_COUNT (*(volatile u64 *)W2M_RUN_TABLE_VA)
#define RUN_ENDED 2u                              /* threads.c's ENDED */

/* `index` made 0 for every live thread (Sprint 347): Windows reads 0 at another thread's slot after TlsFree of the
 * index and after TlsAlloc takes it again (run 45780352907, at a TEB slot and at an expansion slot; Microsoft Learn,
 * TlsAlloc: "When a TLS index is allocated, its storage slots are initialized to NULL"). Called with the slots held.
 * The caller's slot first, since a process that has made no thread has no run table entry (its count 0); then each
 * thread the run table lists, reached by the TEB its entry holds, as threads.c's `scratch_of` reaches another
 * thread's TEB ("`e->teb + W2M_THREAD_SCRATCH_OFF`"); not an ended thread's, nor an entry whose TEB the host has yet
 * to write. Each slot is found by `tls_slot_given`, which gives the first thread its expansion array when it has none.
 * Each store is one aligned 8-byte store, single-copy atomic, made with no lock the thread's own TlsGetValue or
 * TlsSetValue takes: a thread reading or writing the index at the same moment sees its value or 0, which is that
 * application's race on a freed or newly taken index, as it is on Windows. */
static void tls_clear_everywhere(u64 index) {
    u64 *mine = tls_slot_given((u8 *)w2m_teb(), index);
    if (mine) *mine = 0;
    u64 n = RUN_COUNT;
    for (u64 i = 0; i < n; i++) {
        const volatile struct run_head *e =
            (const volatile struct run_head *)(W2M_RUN_TABLE_VA + W2M_RUN_ENTRIES_OFF + i * W2M_RUN_ENTRY);
        u64 teb = e->teb;
        if (!teb || e->status == RUN_ENDED) continue;
        u64 *slot = tls_slot_given((u8 *)teb, index);
        if (slot) *(volatile u64 *)slot = 0;
    }
}

u64 k32_TlsAlloc(void) {
    hold_slots();
    for (u64 i = 0; i < TLS_INDICES; i++) {
        if (tls_used[i]) continue;
        tls_used[i] = 1;
        tls_clear_everywhere(i);
        let_slots_go();
        return w2m_logged(W2M_API_TlsAlloc, 0, i);
    }
    let_slots_go();
    w2m_unimplemented(W2M_API_TlsAlloc, TLS_INDICES);      /* out of indices: not observed */
    return TLS_OUT_OF_INDEXES;
}

u64 k32_TlsGetValue(u64 index) {
    index = (u32)index;
    if (index >= TLS_INDICES) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_TlsGetValue, index, 0);
    }
    u64 *slot = tls_slot(index);
    set_error(0);
    return w2m_logged(W2M_API_TlsGetValue, index, slot ? *slot : 0);
}

u64 k32_TlsSetValue(u64 index, u64 value) {
    index = (u32)index;
    if (index >= TLS_INDICES) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_TlsSetValue, index, 0);
    }
    u64 *slot = tls_slot(index);
    if (!slot) w2m_unimplemented(W2M_API_TlsSetValue, index);   /* no index past 63 ever allocated: not in the rows */
    *slot = value;
    return w2m_logged(W2M_API_TlsSetValue, index, 1);
}

u64 k32_TlsFree(u64 index) {
    hold_slots();
    index = (u32)index;
    if (index >= TLS_INDICES || !tls_used[index]) {
        let_slots_go();
        w2m_unimplemented(W2M_API_TlsFree, index);
    }
    tls_used[index] = 0;
    tls_clear_everywhere(index);
    let_slots_go();
    return w2m_logged(W2M_API_TlsFree, index, 1);
}

u64 k32_FlsAlloc(u64 callback) {
    hold_slots();
    for (u64 i = 1; i < FLS_INDICES; i++) {
        if (fls_used[i]) continue;
        fls_used[i] = 1;
        for (struct fls_block *b = fls_blocks; b; b = b->next) b->value[i] = 0;
        fls_callback[i] = callback;
        let_slots_go();
        return w2m_logged(W2M_API_FlsAlloc, callback, i);
    }
    let_slots_go();
    w2m_unimplemented(W2M_API_FlsAlloc, FLS_INDICES);
    return TLS_OUT_OF_INDEXES;
}

u64 k32_FlsGetValue(u64 index) {
    index = (u32)index;
    hold_slots();
    struct fls_block *b = fls_mine(0);
    u64 v = b && index < FLS_INDICES ? b->value[index] : 0;
    let_slots_go();
    if (!b || index >= FLS_INDICES) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_FlsGetValue, index, 0);
    }
    set_error(0);                                  /* a freed index too: NULL, error 0 (row fls2.freed, Server 2025) */
    return w2m_logged(W2M_API_FlsGetValue, index, v);
}

/* FlsGetValue2 and TlsGetValue2 (Server 2025 exports them, 2022 does not): the same values as FlsGetValue
 * and TlsGetValue, the last error left in every case (contracts/windows.thread_storage.get2). */
u64 k32_FlsGetValue2(u64 index) {
    index = (u32)index;
    hold_slots();
    struct fls_block *b = fls_mine(0);
    u64 v = b && index < FLS_INDICES ? b->value[index] : 0;
    let_slots_go();
    return w2m_logged(W2M_API_FlsGetValue2, index, v);
}

u64 k32_TlsGetValue2(u64 index) {
    index = (u32)index;
    u64 *slot = index < TLS_INDICES ? tls_slot(index) : 0;
    return w2m_logged(W2M_API_TlsGetValue2, index, slot ? *slot : 0);
}

u64 k32_FlsSetValue(u64 index, u64 value) {
    index = (u32)index;
    hold_slots();
    if (index >= FLS_INDICES || !fls_used[index]) {
        let_slots_go();
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_FlsSetValue, index, 0);
    }
    fls_mine(1)->value[index] = value;
    let_slots_go();
    return w2m_logged(W2M_API_FlsSetValue, index, 1);
}

u64 k32_FlsFree(u64 index) {
    index = (u32)index;
    hold_slots();
    if (index >= FLS_INDICES || !fls_used[index]) {
        let_slots_go();
        w2m_unimplemented(W2M_API_FlsFree, index);
    }
    for (struct fls_block *b = fls_blocks; b; b = b->next)
        if (fls_callback[index] && b->value[index]) {
            let_slots_go();
            w2m_unimplemented(W2M_API_FlsFree, index);    /* the callback would run in guest code */
        }
    fls_used[index] = 0;
    for (struct fls_block *b = fls_blocks; b; b = b->next) b->value[index] = 0;
    let_slots_go();
    return w2m_logged(W2M_API_FlsFree, index, 1);
}
