// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* heap_query.c — the process heap as VirtualQuery shows it (Sprint 049, family "heap query").
 *
 * No C runtime. mono-2.0-bdwgc.dll's collector calls VirtualQuery on a process-heap block (frontier 2026-10-11,
 * 3,010,477 instructions). Addresses differ between runs, so rows are relations to GetProcessHeap() (the heap's
 * handle, its first segment's base on Windows) and to the block: the region of a 64-byte block, of the heap handle,
 * of a 1 MB block (above the size HeapAlloc serves from its segments), and the page after the small block's region
 * ends; each with its last error after SetLastError(0xDEAD). Rows are written as they end.
 */
#include <windows.h>

#define SENTINEL 0xDEADu

static char out[4096];
static unsigned n;
static ULONG_PTR heap;

static void put(const char *s) { while (*s) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}

static MEMORY_BASIC_INFORMATION query(const char *name, ULONG_PTR at, ULONG_PTR block) {
    MEMORY_BASIC_INFORMATION m;
    SetLastError(SENTINEL);
    SIZE_T r = VirtualQuery((void *)at, &m, sizeof m);
    DWORD err = GetLastError();
    row(name); field("ret_is_size", r == sizeof m); field("err", err);
    field("base_is_page", (ULONG_PTR)m.BaseAddress == (at & ~(ULONG_PTR)0xFFF));
    field("allocation_base_is_heap", (ULONG_PTR)m.AllocationBase == heap);
    field("allocation_base_below_block", (ULONG_PTR)m.AllocationBase <= block && (ULONG_PTR)m.AllocationBase + 0x10000 > block);
    field("allocation_base_64k", (ULONG_PTR)m.AllocationBase % 0x10000 == 0);
    field("allocation_protect", m.AllocationProtect); field("state", m.State); field("protect", m.Protect);
    field("type", m.Type);
    end();
    return m;
}

void start(void) {
    heap = (ULONG_PTR)GetProcessHeap();
    BYTE *block64 = HeapAlloc(GetProcessHeap(), 0, 64);
    BYTE *big = HeapAlloc(GetProcessHeap(), 0, 1 << 20);
    row("blocks"); field("small_above_heap", (ULONG_PTR)block64 > heap); field("big_non_null", big != NULL); end();
    MEMORY_BASIC_INFORMATION m = query("query.small", (ULONG_PTR)block64, (ULONG_PTR)block64);
    query("query.heap", heap, heap);
    query("query.big", (ULONG_PTR)big, (ULONG_PTR)big);
    ULONG_PTR after = (ULONG_PTR)m.BaseAddress + m.RegionSize;
    query("query.after_small_region", after, after);
    ExitProcess(0);
}
