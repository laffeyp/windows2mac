// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* function_tables.c — ntdll's growable function tables and RtlLookupFunctionEntry over them (Sprint 049, family
 * "function tables").
 *
 * As contracts/windows.exception.function_tables records Windows answering (Server 2025 and 2022):
 *   - RtlAddGrowableFunctionTable answers 0 and an opaque table reference, also for zero entries
 *   - RtlLookupFunctionEntry answers the table's entry whose [RangeBase + BeginAddress, RangeBase + EndAddress)
 *     holds the address, among the first EntryCount entries (RtlGrowFunctionTable raises the count), and writes
 *     RangeBase as the image base; no entry: NULL and the image base left as it was
 *   - RtlDeleteGrowableFunctionTable removes the table; no call touches the last error
 * The functions an image's own .pdata describes are not looked up yet: an address in a loaded image stops the run.
 */
#include "runtime.h"

#define TABLES 64

struct runtime_function { u32 begin, end, unwind; };
static struct table { u64 entries, count, max, base, end; int used; } tables[TABLES];

/* RtlAddGrowableFunctionTable(DynamicTable, FunctionTable, EntryCount, MaximumEntryCount, RangeBase, RangeEnd):
 * the last two on the guest's stack at RSP + 0x28 and + 0x30. */
u64 ntdll_RtlAddGrowableFunctionTable(u64 out, u64 entries, u64 count, u64 max, u64 rsp) {
    u64 base = *(u64 *)(rsp + 0x28), end = *(u64 *)(rsp + 0x30);
    if (!out || (u32)count > (u32)max || base >= end) w2m_unimplemented(W2M_API_RtlAddGrowableFunctionTable, count);
    for (int i = 0; i < TABLES; i++)
        if (!tables[i].used) {
            tables[i] = (struct table){entries, (u32)count, (u32)max, base, end, 1};
            *(u64 *)out = (u64)&tables[i];
            return w2m_logged(W2M_API_RtlAddGrowableFunctionTable, base, 0);
        }
    w2m_unimplemented(W2M_API_RtlAddGrowableFunctionTable, TABLES);
    return 0;
}

static struct table *table_of(u64 api, u64 t) {
    if (t < (u64)&tables[0] || t >= (u64)&tables[TABLES] || (t - (u64)&tables[0]) % sizeof tables[0])
        w2m_unimplemented(api, t);
    struct table *tb = (struct table *)t;
    if (!tb->used) w2m_unimplemented(api, t);
    return tb;
}

/* RtlGrowFunctionTable(DynamicTable, NewEntryCount) */
W2M_VOID u64 ntdll_RtlGrowFunctionTable(u64 t, u64 count) {
    struct table *tb = table_of(W2M_API_RtlGrowFunctionTable, t);
    if ((u32)count > tb->max || (u32)count < tb->count) w2m_unimplemented(W2M_API_RtlGrowFunctionTable, count);
    tb->count = (u32)count;
    return w2m_logged(W2M_API_RtlGrowFunctionTable, t, 0);
}

/* RtlDeleteGrowableFunctionTable(DynamicTable) */
W2M_VOID u64 ntdll_RtlDeleteGrowableFunctionTable(u64 t) {
    table_of(W2M_API_RtlDeleteGrowableFunctionTable, t)->used = 0;
    return w2m_logged(W2M_API_RtlDeleteGrowableFunctionTable, t, 0);
}

/* RtlLookupFunctionEntry(ControlPc, ImageBase, HistoryTable) */
u64 ntdll_RtlLookupFunctionEntry(u64 pc, u64 image, u64 history) {
    (void)history;
    if (!image) w2m_unimplemented(W2M_API_RtlLookupFunctionEntry, 0);                 /* not in the rows */
    for (int i = 0; i < TABLES; i++) {
        struct table *tb = &tables[i];
        if (!tb->used || pc < tb->base || pc >= tb->end) continue;
        const struct runtime_function *f = (const struct runtime_function *)tb->entries;
        u64 lo = 0, hi = tb->count;                    /* sorted by BeginAddress (the page's rule) */
        while (lo < hi) {
            u64 mid = (lo + hi) / 2;
            if (pc < tb->base + f[mid].begin) hi = mid;
            else if (pc >= tb->base + f[mid].end) lo = mid + 1;
            else {
                *(u64 *)image = tb->base;
                return w2m_logged(W2M_API_RtlLookupFunctionEntry, pc, (u64)&f[mid]);
            }
        }
        return w2m_logged(W2M_API_RtlLookupFunctionEntry, pc, 0);
    }
    if (w2m_host(W2M_HOST_MODULE_AT, pc, 0, 0, 0, 0))                  /* an image's .pdata: not yet */
        w2m_unimplemented(W2M_API_RtlLookupFunctionEntry, pc);
    return w2m_logged(W2M_API_RtlLookupFunctionEntry, pc, 0);
}
