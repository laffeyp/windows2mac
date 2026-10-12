// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* slist.c — InterlockedPushEntrySList, InterlockedPopEntrySList, InterlockedFlushSList (Sprint 063, family
 * "interlocked lists"; InitializeSListHead is process.c's, family process_queries).
 *
 * As contracts/windows.interlocked.lists records Windows answering (Server 2025 and 2022, AMD EPYC 7763 and 9V74):
 *   - the header's high quadword is the first entry's address (its low 4 bits 0), the low quadword's low 16 bits
 *     the depth and the rest a sequence; each entry's first quadword links the next
 *   - a push links the entry to the old first, makes it first, adds one to the depth and advances the sequence,
 *     and returns the old first (NULL for an empty list); a pop returns the first, leaves its link, makes its next
 *     first and takes one from the depth; a flush returns the first and leaves depth 0 and no entry; neither
 *     changes the sequence; no call changes the last error
 *   - the sequence advances by 2 a push on Server 2025 and by 1 on 2022: w2m answers as 2025, the game's reference
 *     run's system (game run 38206263976)
 * Microsoft Learn: "All list items must be aligned on a MEMORY_ALLOCATION_ALIGNMENT boundary; otherwise, this
 * function will behave unpredictably." A header or entry off a 16-byte boundary stops the run.
 *
 * Atomic for the guest: w2m runs the guest's threads on one vCPU and switches between them only at a translated
 * block's entry (scheduler.py), and a service runs from the bridge to its return with no translated code in
 * between, so no other guest thread runs inside one of these calls. The game makes 40,403 pushes, 40,870 flushes
 * and 9,326 pops in 180 s (jit_capture.bin.imports.tsv): they stay in the guest.
 */
#include "runtime.h"

#define SEQUENCE_STEP (2ull << 16)      /* Server 2025's (row push.1: sequence 2) */
#define DEPTH_MASK 0xFFFFull

static void aligned(u64 api, u64 address) {
    if (address & 15) w2m_unimplemented(api, address);          /* "behave unpredictably": no row */
}

u64 k32_InterlockedPushEntrySList(u64 head, u64 entry) {
    aligned(W2M_API_InterlockedPushEntrySList, head);
    aligned(W2M_API_InterlockedPushEntrySList, entry);
    u64 *h = (u64 *)head;
    u64 first = h[1] & ~15ull;
    *(u64 *)entry = first;
    h[0] = ((h[0] & ~DEPTH_MASK) + SEQUENCE_STEP) | (((h[0] & DEPTH_MASK) + 1) & DEPTH_MASK);
    h[1] = entry;
    return w2m_logged(W2M_API_InterlockedPushEntrySList, head, first);
}

u64 k32_InterlockedPopEntrySList(u64 head) {
    aligned(W2M_API_InterlockedPopEntrySList, head);
    u64 *h = (u64 *)head;
    u64 first = h[1] & ~15ull;
    if (first) {
        h[1] = *(u64 *)first;
        h[0] = (h[0] & ~DEPTH_MASK) | (((h[0] & DEPTH_MASK) - 1) & DEPTH_MASK);
    }
    return w2m_logged(W2M_API_InterlockedPopEntrySList, head, first);
}

u64 k32_InterlockedFlushSList(u64 head) {
    aligned(W2M_API_InterlockedFlushSList, head);
    u64 *h = (u64 *)head;
    u64 first = h[1] & ~15ull;
    h[1] = 0;
    h[0] &= ~DEPTH_MASK;
    return w2m_logged(W2M_API_InterlockedFlushSList, head, first);
}
