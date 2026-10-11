// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* thread_stack.c — the main thread's stack as VirtualQuery and the TEB show it (Sprint 049, family "thread stack").
 *
 * No C runtime. mono-2.0-bdwgc.dll's collector calls VirtualQuery on a page of the thread's stack (frontier
 * 2026-10-11, 2,977,437 instructions: 0xfffef000). Addresses differ between runs, so every row is a relation to
 * the TEB's NtTib.StackBase (+0x08), NtTib.StackLimit (+0x10) and DeallocationStack (+0x1478) and to the image's
 * SizeOfStackReserve: the region holding the stack pointer, the one just below StackLimit, the one at
 * DeallocationStack; GetCurrentThreadStackLimits; each with its last error after SetLastError(0xDEAD).
 */
#include <windows.h>

#define SENTINEL 0xDEADu

extern IMAGE_DOS_HEADER __ImageBase;

static char out[4096];
static unsigned n;
static ULONG_PTR base, limit, dealloc;

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

/* An address as a name: one of the TEB's fields, or its distance in pages from StackLimit. */
static void where(const char *k, ULONG_PTR v) {
    put(",\""); put(k); put("\":");
    if (v == base) put("\"base\"");
    else if (v == limit) put("\"limit\"");
    else if (v == dealloc) put("\"dealloc\"");
    else if (v < limit) { put("\"limit-"); putu((limit - v) / 4096); put("p\""); }
    else { put("\"limit+"); putu((v - limit) / 4096); put("p\""); }
}

static void query(const char *name, ULONG_PTR at) {
    MEMORY_BASIC_INFORMATION m;
    SetLastError(SENTINEL);
    SIZE_T r = VirtualQuery((void *)at, &m, sizeof m);
    DWORD err = GetLastError();
    row(name); field("ret_is_size", r == sizeof m); field("err", err);
    where("base_address", (ULONG_PTR)m.BaseAddress); where("allocation_base", (ULONG_PTR)m.AllocationBase);
    where("end", (ULONG_PTR)m.BaseAddress + m.RegionSize);
    put(",\"allocation_protect\":"); putu(m.AllocationProtect); field("state", m.State); field("protect", m.Protect);
    field("type", m.Type);
    end();
}

void start(void) {
    BYTE *teb = (BYTE *)NtCurrentTeb();
    base = *(ULONG_PTR *)(teb + 0x08);
    limit = *(ULONG_PTR *)(teb + 0x10);
    dealloc = *(ULONG_PTR *)(teb + 0x1478);
    IMAGE_NT_HEADERS64 *nt = (IMAGE_NT_HEADERS64 *)((BYTE *)&__ImageBase + __ImageBase.e_lfanew);
    ULONG_PTR sp = (ULONG_PTR)&teb;
    row("teb"); field("reserve_is_header", base - dealloc == nt->OptionalHeader.SizeOfStackReserve);
    field("sp_inside", limit <= sp && sp < base); field("aligned", (base | limit | dealloc) % 4096 == 0);
    field("limit_above_dealloc", limit > dealloc); end();

    query("query.sp", sp);
    query("query.below_limit", limit - 1);
    query("query.dealloc", dealloc);
    query("query.base_minus_one", base - 1);

    ULONG_PTR low = 0, high = 0;
    SetLastError(SENTINEL);
    GetCurrentThreadStackLimits(&low, &high);
    DWORD err = GetLastError();
    row("limits"); where("low", low); where("high", high); field("err", err); end();
    ExitProcess(0);
}
