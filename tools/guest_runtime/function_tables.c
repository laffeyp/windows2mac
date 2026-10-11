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
 * An address in a loaded image is looked up in its .pdata (exceptions.c; Sprint 053).
 */
#include "runtime.h"

#define TABLES 64

u64 w2m_image_function(u64 pc, u64 *base);       /* exceptions.c: an image's .pdata entry */

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

/* The one search, for RtlLookupFunctionEntry and exceptions.c: 1 when a growable table's range holds pc, its base in
 * *base and the entry holding pc in *entry (0: none, the table sorted by BeginAddress, the page's rule); 0 when no
 * table holds pc. */
int w2m_table_function(u64 pc, u64 *base, u64 *entry) {
    for (int i = 0; i < TABLES; i++) {
        struct table *tb = &tables[i];
        if (!tb->used || pc < tb->base || pc >= tb->end) continue;
        const struct runtime_function *f = (const struct runtime_function *)tb->entries;
        u64 lo = 0, hi = tb->count;
        *entry = 0;
        while (lo < hi) {
            u64 mid = (lo + hi) / 2;
            if (pc < tb->base + f[mid].begin) hi = mid;
            else if (pc >= tb->base + f[mid].end) lo = mid + 1;
            else { *entry = (u64)&f[mid]; break; }
        }
        *base = tb->base;
        return 1;
    }
    return 0;
}

/* RtlLookupFunctionEntry(ControlPc, ImageBase, HistoryTable): a table's entry, else the image's .pdata entry. */
u64 ntdll_RtlLookupFunctionEntry(u64 pc, u64 image, u64 history) {
    (void)history;
    if (!image) w2m_unimplemented(W2M_API_RtlLookupFunctionEntry, 0);                 /* not in the rows */
    u64 base = 0, entry = 0;
    if (w2m_table_function(pc, &base, &entry)) {
        if (entry) *(u64 *)image = base;               /* no entry: the image base left as it was */
        return w2m_logged(W2M_API_RtlLookupFunctionEntry, pc, entry);
    }
    return w2m_logged(W2M_API_RtlLookupFunctionEntry, pc, w2m_image_function(pc, (u64 *)image));
}
