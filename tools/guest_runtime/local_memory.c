// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* local_memory.c — KERNEL32's LocalAlloc and LocalFree on the process heap (Sprint 048, families "local free" and
 * "integrity level").
 *
 * As contracts/windows.token.integrity_level records Windows answering (Server 2025 and 2022): LocalAlloc with
 * LMEM_FIXED or LPTR (LMEM_FIXED | LMEM_ZEROINIT) returns a 16-aligned block of the process heap whose HeapSize is
 * the size asked; LPTR zeroes it; the last error is left. LMEM_MOVEABLE's handles are not on the game's path and
 * stop the run.
 */
#include "runtime.h"

#define LMEM_ZEROINIT 0x40u

/* LocalAlloc(uFlags, uBytes) */
u64 k32_LocalAlloc(u64 flags, u64 size) {
    if ((u32)flags & ~LMEM_ZEROINIT) w2m_unimplemented(W2M_API_LocalAlloc, flags);    /* LMEM_MOVEABLE and the rest */
    u64 block = w2m_heap_take(size);
    if (!block) w2m_unimplemented(W2M_API_LocalAlloc, size);                         /* Windows would grant it */
    if ((u32)flags & LMEM_ZEROINIT)
        for (u64 i = 0; i < size; i++) ((u8 *)block)[i] = 0;
    return w2m_logged(W2M_API_LocalAlloc, size, block);
}

/* LocalFree (contracts/windows.memory.local_free, Sprint 048): NULL and a block's start answer NULL (the block is
 * freed); a pointer 8 bytes past a 16-byte boundary, as UnityPlayer passes argv + 1, answers itself with
 * ERROR_INVALID_HANDLE and frees nothing; Windows ends the process for a 16-aligned pointer inside a block, which
 * is not on the game's path and stops the run here (heap.c checks the block). */
u64 k32_LocalFree(u64 block) {
    if ((block & 15) == 8) {
        w2m_set_last_error(6);
        return w2m_logged(W2M_API_LocalFree, block, block);
    }
    if (block) w2m_heap_release(W2M_API_LocalFree, block);
    return w2m_logged(W2M_API_LocalFree, block, 0);
}
