// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* virtual_memory.c — reserve, commit, protect, query, decommit, release (Sprint 046, family "virtual
 * memory").
 *
 * No C runtime; one thread. UnityPlayer.dll's DllMain reserves 0x1FFFF000 bytes MEM_RESERVE PAGE_NOACCESS
 * (frontier 2026-10-10, 175,529 instructions), and the static C runtime's DLL start-up calls VirtualProtect.
 * Rows: each call's return (or its relation: non-NULL, 64 KB aligned, equal to an earlier address) and last
 * error after SetLastError(0xDEAD), and VirtualQuery's answer (state, protection, allocation protection,
 * type, region size, the base and allocation base against the reservation). What a fault on a protected
 * page does is not here: w2m enforces protections in a later sprint.
 */
#include <windows.h>

#define SENTINEL 0xDEADu
#define RESERVE_SIZE 0x1FFFF000ull

static char out[16384];
static unsigned n;

static void put(const char *s) { while (*s) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void end(void) { put("}\n"); }

static DWORD err;

static void query(const char *name, void *at, void *reservation) {
    MEMORY_BASIC_INFORMATION m;
    unsigned char *b = (unsigned char *)&m;
    for (unsigned i = 0; i < sizeof m; i++) b[i] = 0xA5;
    SetLastError(SENTINEL);
    SIZE_T r = VirtualQuery(at, &m, sizeof m);
    err = GetLastError();
    row(name); field("ret", r); field("err", err);
    if (r) {
        field("state", m.State); field("protect", m.Protect); field("allocation_protect", m.AllocationProtect);
        field("type", m.Type); field("region_size", m.RegionSize);
        flag("base_is_page_of_address", m.BaseAddress == (void *)((ULONG_PTR)at & ~0xFFFull));
        flag("allocation_base_is_reservation", m.AllocationBase == reservation);
    }
    end();
}

static void *alloc(const char *name, void *at, SIZE_T size, DWORD type, DWORD protect, void *expect) {
    SetLastError(SENTINEL);
    void *p = VirtualAlloc(at, size, type, protect);
    err = GetLastError();
    row(name); flag("non_null", p != 0); field("err", err);
    if (p) { flag("aligned_64k", ((ULONG_PTR)p & 0xFFFF) == 0); flag("above_4gb", (ULONG_PTR)p > 0xFFFFFFFFull); flag("is_expected", p == expect); }
    end();
    return p;
}

static void protect(const char *name, void *at, SIZE_T size, DWORD prot) {
    DWORD old = 0xA5A5A5A5u;
    SetLastError(SENTINEL);
    BOOL ok = VirtualProtect(at, size, prot, &old);
    err = GetLastError();
    row(name); field("ret", ok != 0); field("err", err); field("old", old); end();
}

static void release(const char *name, void *at, SIZE_T size, DWORD type) {
    SetLastError(SENTINEL);
    BOOL ok = VirtualFree(at, size, type);
    err = GetLastError();
    row(name); field("ret", ok != 0); field("err", err); end();
}

void start(void) {
    unsigned char *r = alloc("reserve", NULL, RESERVE_SIZE, MEM_RESERVE, PAGE_NOACCESS, NULL);
    query("query.reserved", r, r);
    unsigned char *c = alloc("commit_inside", r, 0x10000, MEM_COMMIT, PAGE_READWRITE, r);
    int zero = 1;
    for (int i = 0; i < 0x10000; i++) zero &= c[i] == 0;
    c[5] = 7;
    row("commit_inside.zeroed"); flag("value", zero); end();
    query("query.committed", c + 0x1234, r);
    query("query.after_committed", c + 0x10000, r);
    unsigned char *c2 = alloc("commit_unaligned_inside", r + 0x20123, 0x10, MEM_COMMIT, PAGE_READWRITE, r + 0x20000);
    query("query.unaligned_commit", r + 0x20000, r);
    protect("protect.readonly", c, 0x1000, PAGE_READONLY);
    query("query.readonly", c, r);
    protect("protect.back", c, 0x1000, PAGE_READWRITE);
    protect("protect.two_protections", c, 0x2000, PAGE_EXECUTE_READ);
    query("query.after_two", c, r);
    protect("protect.reserved_page", r + 0x100000, 0x1000, PAGE_READWRITE);
    alloc("alloc.size_zero", NULL, 0, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE, NULL);
    alloc("alloc.reserve_taken", r, 0x10000, MEM_RESERVE, PAGE_READWRITE, NULL);
    alloc("alloc.bad_protect", NULL, 0x1000, MEM_RESERVE | MEM_COMMIT, 0x12345, NULL);
    unsigned char *little_block = alloc("alloc.little_block", NULL, 100, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE, NULL);
    query("query.little_block", little_block, little_block);
    release("free.decommit", c, 0x10000, MEM_DECOMMIT);
    query("query.decommitted", c, r);
    release("free.release_with_size", r, 0x10000, MEM_RELEASE);
    release("free.release_inside", r + 0x10000, 0, MEM_RELEASE);
    release("free.release", r, 0, MEM_RELEASE);
    query("query.released", r, NULL);
    release("free.release_again", r, 0, MEM_RELEASE);
    release("free.little_block", little_block, 0, MEM_RELEASE);
    (void)c2;

    HMODULE self = GetModuleHandleW(NULL);
    unsigned char *base = (unsigned char *)self;
    IMAGE_NT_HEADERS64 *nt = (IMAGE_NT_HEADERS64 *)(base + ((IMAGE_DOS_HEADER *)base)->e_lfanew);
    IMAGE_SECTION_HEADER *sec = IMAGE_FIRST_SECTION(nt);
    query("image.headers", base, base);
    for (int i = 0; i < nt->FileHeader.NumberOfSections && i < 6; i++) {
        static char name[24] = "image.section.";
        int k = 14;
        for (int j = 0; j < 8 && sec[i].Name[j]; j++) name[k++] = sec[i].Name[j] == '.' ? '_' : (char)sec[i].Name[j];
        name[k] = 0;
        query(name, base + sec[i].VirtualAddress, base);
    }
    IMAGE_SECTION_HEADER *rdata = 0;
    for (int i = 0; i < nt->FileHeader.NumberOfSections; i++)
        if (sec[i].Name[0] == '.' && sec[i].Name[1] == 'r' && sec[i].Name[2] == 'd') rdata = &sec[i];
    if (rdata) {
        protect("image.protect_rdata_rw", base + rdata->VirtualAddress, 0x1000, PAGE_READWRITE);
        query("image.rdata_after", base + rdata->VirtualAddress, base);
        protect("image.protect_rdata_back", base + rdata->VirtualAddress, 0x1000, PAGE_READONLY);
    }

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
