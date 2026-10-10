// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* thread_storage.c — TLS and FLS on one thread (Sprint 044, family "thread storage").
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
 * TlsGetValue is the game's second-hottest import (23,291,261 calls in 180 s): it never leaves the guest.
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
static u64 fls_value[FLS_INDICES];
static u64 fls_callback[FLS_INDICES];
static u8 fls_used[FLS_INDICES];
static int fls_thread_data;                       /* the thread's FLS data exists (first FlsSetValue) */
static int tls_ready;

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

static u64 *tls_slot(u64 index) {
    u8 *teb = (u8 *)w2m_teb();
    if (index < TLS_SLOTS) return (u64 *)(teb + TEB_TLS_SLOTS) + index;
    u64 *expansion = *(u64 **)(teb + TEB_TLS_EXPANSION);
    return expansion ? expansion + (index - TLS_SLOTS) : 0;
}

static void tls_init(void) {
    if (tls_ready) return;
    tls_ready = 1;
    tls_used[0] = 1;                              /* held in a fresh process (row tls.alloc.first) */
    tls_used[W2M_TLS_TAKEN] = 1;                  /* the other one (row tls.alloc.many.indices) */
}

u64 k32_TlsAlloc(void) {
    tls_init();
    for (u64 i = 0; i < TLS_INDICES; i++) {
        if (tls_used[i]) continue;
        if (i >= TLS_SLOTS && *(u64 *)((u8 *)w2m_teb() + TEB_TLS_EXPANSION) == 0)
            *(u64 *)((u8 *)w2m_teb() + TEB_TLS_EXPANSION) = W2M_TLS_EXPANSION_VA;
        tls_used[i] = 1;
        u64 *slot = tls_slot(i);
        *slot = 0;
        return w2m_logged(W2M_API_TlsAlloc, 0, i);
    }
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
    if (!slot) w2m_unimplemented(W2M_API_TlsSetValue, index);   /* an expansion index never allocated */
    *slot = value;
    return w2m_logged(W2M_API_TlsSetValue, index, 1);
}

u64 k32_TlsFree(u64 index) {
    tls_init();
    index = (u32)index;
    if (index >= TLS_INDICES || !tls_used[index]) w2m_unimplemented(W2M_API_TlsFree, index);
    tls_used[index] = 0;
    u64 *slot = tls_slot(index);
    if (slot) *slot = 0;
    return w2m_logged(W2M_API_TlsFree, index, 1);
}

u64 k32_FlsAlloc(u64 callback) {
    for (u64 i = 1; i < FLS_INDICES; i++) {
        if (fls_used[i]) continue;
        fls_used[i] = 1;
        fls_value[i] = 0;
        fls_callback[i] = callback;
        return w2m_logged(W2M_API_FlsAlloc, callback, i);
    }
    w2m_unimplemented(W2M_API_FlsAlloc, FLS_INDICES);
    return TLS_OUT_OF_INDEXES;
}

u64 k32_FlsGetValue(u64 index) {
    index = (u32)index;
    if (!fls_thread_data || index >= FLS_INDICES) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_FlsGetValue, index, 0);
    }
    set_error(0);                                  /* a freed index too: NULL, error 0 (row fls2.freed, Server 2025) */
    return w2m_logged(W2M_API_FlsGetValue, index, fls_value[index]);
}

/* FlsGetValue2 and TlsGetValue2 (Server 2025 exports them, 2022 does not): the same values as FlsGetValue
 * and TlsGetValue, the last error left in every case (contracts/windows.thread_storage.get2). */
u64 k32_FlsGetValue2(u64 index) {
    index = (u32)index;
    u64 v = fls_thread_data && index < FLS_INDICES ? fls_value[index] : 0;
    return w2m_logged(W2M_API_FlsGetValue2, index, v);
}

u64 k32_TlsGetValue2(u64 index) {
    index = (u32)index;
    u64 *slot = index < TLS_INDICES ? tls_slot(index) : 0;
    return w2m_logged(W2M_API_TlsGetValue2, index, slot ? *slot : 0);
}

u64 k32_FlsSetValue(u64 index, u64 value) {
    index = (u32)index;
    if (index >= FLS_INDICES || !fls_used[index]) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_FlsSetValue, index, 0);
    }
    fls_thread_data = 1;
    fls_value[index] = value;
    return w2m_logged(W2M_API_FlsSetValue, index, 1);
}

u64 k32_FlsFree(u64 index) {
    index = (u32)index;
    if (index >= FLS_INDICES || !fls_used[index]) w2m_unimplemented(W2M_API_FlsFree, index);
    if (fls_callback[index] && fls_value[index])
        w2m_unimplemented(W2M_API_FlsFree, index);        /* the callback would run in guest code */
    fls_used[index] = 0;
    fls_value[index] = 0;
    return w2m_logged(W2M_API_FlsFree, index, 1);
}
