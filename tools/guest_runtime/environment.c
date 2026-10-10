// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* environment.c — the environment block (Sprint 044, family "environment").
 *
 * GetEnvironmentStringsW, FreeEnvironmentStringsW and lstrlenW, as
 * contracts/windows.environment.{set,empty}_block record Windows answering a process started with an
 * explicit block and no handles (12 runners, Server 2025 and 2022, AMD and Intel; 24 identical outputs):
 *   - the block is the one the start passed, entries in its order, a drive variable ("=C:=...") kept,
 *     nothing added
 *   - each call returns a new copy from the process heap of exactly the block's size: the entries, each
 *     with its NUL, then a NUL (69 units, 138 bytes, for five entries); an empty block is two NULs
 *     (HeapSize 4); a write into one copy does not show in the next
 *   - FreeEnvironmentStringsW frees the copy and answers TRUE; both calls leave the last error
 * The block is the host's: UTF-16 entries at W2M_ENVIRONMENT_VA (src/w2m/vm_backend.py, `environment`).
 */
#include "runtime.h"

typedef unsigned short u16;

u64 k32_GetEnvironmentStringsW(void) {
    const u16 *host = (const u16 *)W2M_ENVIRONMENT_VA;
    u64 units = 0;
    if (!host[0]) units = 2;                         /* empty: two NULs (row empty.heap) */
    else {
        while (host[units] || host[units + 1]) units++;
        units += 2;
    }
    u64 copy = w2m_heap_take(units * 2);
    if (!copy) w2m_unimplemented(W2M_API_GetEnvironmentStringsW, units);
    u16 *to = (u16 *)copy;
    for (u64 i = 0; i < units; i++) to[i] = host[i];
    return w2m_logged(W2M_API_GetEnvironmentStringsW, units, copy);
}

u64 k32_FreeEnvironmentStringsW(u64 block) {
    if (!block) w2m_unimplemented(W2M_API_FreeEnvironmentStringsW, 0);     /* not in the rows */
    w2m_heap_release(W2M_API_FreeEnvironmentStringsW, block);
    return w2m_logged(W2M_API_FreeEnvironmentStringsW, block, 1);
}

/* lstrlenW: the units before the NUL (the probe's child counts entries with it). */
u64 k32_lstrlenW(u64 s) {
    if (!s) w2m_unimplemented(W2M_API_lstrlenW, 0);                         /* not in the rows */
    const u16 *w = (const u16 *)s;
    u64 n = 0;
    while (w[n]) n++;
    return w2m_logged(W2M_API_lstrlenW, s, n);
}
