// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* function_tables.c — growable function tables for generated code (Sprint 049, family "function tables").
 *
 * No C runtime. mono-2.0-bdwgc.dll's start-up gets RtlAddGrowableFunctionTable from ntdll by GetProcAddress and
 * calls it (frontier 2026-10-11, 2,752,580 instructions); its JIT registers the unwind data of the code it writes
 * that way. This probe gets ntdll's RtlAddGrowableFunctionTable, RtlGrowFunctionTable,
 * RtlDeleteGrowableFunctionTable and RtlLookupFunctionEntry the same way and records what a table makes visible:
 * which entry RtlLookupFunctionEntry answers for an address (as its index in the table, or none), and the image
 * base it reports (as "range" when it is the table's RangeBase), before and after the table grows and after it is
 * deleted; each status and last error (after SetLastError(0xDEAD)).
 */
#include <windows.h>

#define SENTINEL 0xDEADu

typedef DWORD (WINAPI *add_fn)(PVOID *, PRUNTIME_FUNCTION, DWORD, DWORD, ULONG_PTR, ULONG_PTR);
typedef void (WINAPI *grow_fn)(PVOID, DWORD);
typedef void (WINAPI *delete_fn)(PVOID);
typedef PRUNTIME_FUNCTION (WINAPI *lookup_fn)(DWORD64, PDWORD64, PVOID);

static char out[4096];
static unsigned n;
static lookup_fn lookup;
static RUNTIME_FUNCTION *entries;
static ULONG_PTR base;

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

static void find(const char *name, ULONG_PTR pc) {
    DWORD64 image = 0x5A5A;
    SetLastError(SENTINEL);
    PRUNTIME_FUNCTION f = lookup(pc, &image, NULL);
    DWORD err = GetLastError();
    row(name);
    if (f) { field("entry", (unsigned long long)(f - entries)); field("in_table", f >= entries && f < entries + 4); }
    else put(",\"entry\":null");
    put(",\"image\":");
    if (image == base) put("\"range\""); else if (image == 0x5A5A) put("\"untouched\""); else putu(image);
    field("err", err);
    end();
}

void start(void) {
    HMODULE ntdll = GetModuleHandleW(L"ntdll.dll");
    add_fn add = (add_fn)GetProcAddress(ntdll, "RtlAddGrowableFunctionTable");
    grow_fn grow = (grow_fn)GetProcAddress(ntdll, "RtlGrowFunctionTable");
    delete_fn del = (delete_fn)GetProcAddress(ntdll, "RtlDeleteGrowableFunctionTable");
    lookup = (lookup_fn)GetProcAddress(ntdll, "RtlLookupFunctionEntry");
    row("procs"); field("all", add && grow && del && lookup); end();

    /* A 64 KB region of "code" and, after it, the table and one UNWIND_INFO (version 1, no codes). */
    BYTE *region = VirtualAlloc(NULL, 0x20000, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    base = (ULONG_PTR)region;
    entries = (RUNTIME_FUNCTION *)(region + 0x10000);
    BYTE *unwind = region + 0x11000;
    unwind[0] = 1;
    for (int i = 0; i < 4; i++) {
        entries[i].BeginAddress = 0x100 * (i + 1);
        entries[i].EndAddress = 0x100 * (i + 1) + 0x80;
        entries[i].UnwindData = 0x11000;
    }

    PVOID table = (PVOID)0x5A5A;
    SetLastError(SENTINEL);
    DWORD status = add(&table, entries, 1, 4, base, base + 0x10000);
    DWORD err = GetLastError();
    row("add"); field("status", status); field("table_set", table != (PVOID)0x5A5A && table != NULL); field("err", err); end();
    find("find.first", base + 0x110);
    find("find.first_end", base + 0x180);           /* EndAddress is past the function */
    find("find.second_before_grow", base + 0x210);
    find("find.gap", base + 0x1A0);
    find("find.past_range", base + 0x10010);        /* the table itself: outside RangeEnd */

    SetLastError(SENTINEL);
    grow(table, 3);
    err = GetLastError();
    row("grow"); field("err", err); end();
    find("find.second", base + 0x210);
    find("find.third", base + 0x37F);
    find("find.fourth_beyond_count", base + 0x410);

    SetLastError(SENTINEL);
    del(table);
    err = GetLastError();
    row("delete"); field("err", err); end();
    find("find.after_delete", base + 0x110);

    PVOID empty = (PVOID)0x5A5A;
    SetLastError(SENTINEL);
    status = add(&empty, entries, 0, 4, base, base + 0x10000);
    err = GetLastError();
    row("add.empty"); field("status", status); field("table_set", empty != (PVOID)0x5A5A && empty != NULL); field("err", err); end();
    find("find.empty", base + 0x110);
    del(empty);
    ExitProcess(0);
}
