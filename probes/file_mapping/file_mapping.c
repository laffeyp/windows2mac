// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* file_mapping.c — CreateFileMappingW and MapViewOfFile in mono's form (Sprint 049, family "file mapping";
 * specs/work_list.json).
 *
 * No C runtime. The capture (store/game-runs/38113342549, 38113343972) holds the one form mono uses for each of five
 * assemblies: a file opened GENERIC_READ, share read|write; CreateFileMappingW(h, NULL, PAGE_READONLY, 0, 0, NULL);
 * MapViewOfFile(map, FILE_MAP_READ, 0, 0, <the file's size>); the file handle closed at once; the view never unmapped.
 * Rows: return and last error (after SetLastError(0xDEAD)); for a view, whether its bytes equal the file's, the bytes
 * after the file to the page's end, the view's offset in its 64 KB granule, and VirtualQuery of the view and of the
 * page after it. Addresses print as offsets from the view.
 */
#include <windows.h>

#define SENTINEL 0xDEADu

static char out[16384];
static unsigned n;
static char data[0x14c00];

static void put(const char *s) { while (*s && n < sizeof out - 64) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
/* Each row is written when it ends, so a fault leaves the rows before it. */
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}

static void make(const WCHAR *name, DWORD count) {
    HANDLE h = CreateFileW(name, GENERIC_READ | GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    DWORD done;
    if (count) WriteFile(h, data, count, &done, NULL);
    CloseHandle(h);
}
static HANDLE open_read(const WCHAR *name) {
    return CreateFileW(name, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, NULL, OPEN_EXISTING,
                       FILE_ATTRIBUTE_NORMAL, NULL);
}

static HANDLE mapping(const char *name, HANDLE file, DWORD protect, DWORD high, DWORD low) {
    SetLastError(SENTINEL);
    HANDLE m = CreateFileMappingW(file, NULL, protect, high, low, NULL);
    DWORD err = GetLastError();
    row(name); field("ret_nonzero", m != NULL); field("err", err); end();
    return m;
}

/* VirtualQuery at view + offset: fields relative to the view. */
static void query(const char *name, const unsigned char *view, SIZE_T offset) {
    MEMORY_BASIC_INFORMATION mbi;
    SIZE_T r = VirtualQuery(view + offset, &mbi, sizeof mbi);
    row(name); field("ret", r);
    field("base_offset", (unsigned long long)((const unsigned char *)mbi.BaseAddress - view));
    field("allocation_base_is_view", mbi.AllocationBase == view);
    field("allocation_protect", mbi.AllocationProtect); field("region_size", mbi.RegionSize);
    field("state", mbi.State); field("protect", mbi.Protect); field("type", mbi.Type); end();
}

static const unsigned char *view_of(const char *name, HANDLE m, DWORD access, SIZE_T bytes, DWORD size) {
    SetLastError(SENTINEL);
    const unsigned char *v = MapViewOfFile(m, access, 0, 0, bytes);
    DWORD err = GetLastError();
    row(name); field("ret_nonzero", v != NULL); field("err", err);
    if (v) {
        int same = 1;
        for (DWORD i = 0; i < size; i++) if (v[i] != (unsigned char)data[i]) same = 0;
        int zero_tail = 1;
        for (DWORD i = size; i % 4096; i++) if (v[i]) zero_tail = 0;
        field("bytes_equal", same); field("tail_zero", zero_tail);
        field("granule_offset", (unsigned long long)((ULONG_PTR)v & 0xFFFF));
    }
    end();
    return v;
}

void start(void) {
    for (unsigned i = 0; i < sizeof data; i++) data[i] = (char)(i * 7 + (i >> 8));
    make(L"fmap_game.dat", sizeof data);               /* 0x14c00 bytes: one of the five sizes mono maps */
    make(L"fmap_small.dat", 5);
    make(L"fmap_empty.dat", 0);

    /* mono's form */
    HANDLE f = open_read(L"fmap_game.dat");
    HANDLE m = mapping("map.game_form", f, PAGE_READONLY, 0, 0);
    const unsigned char *v = view_of("view.game_form", m, FILE_MAP_READ, sizeof data, sizeof data);
    SetLastError(SENTINEL);
    BOOL c1 = CloseHandle(f);
    BOOL c2 = CloseHandle(m);
    row("close.file_then_mapping"); field("file", c1 != 0); field("mapping", c2 != 0); field("err", GetLastError()); end();
    if (v) {
        int same = 1;
        for (DWORD i = 0; i < sizeof data; i++) if (v[i] != (unsigned char)data[i]) same = 0;
        row("view.after_close"); field("bytes_equal", same); end();
        query("query.view", v, 0);
        query("query.view_middle", v, 0x8000);
        query("query.after_view", v, 0x15000);         /* the page after the view's last */
        query("query.granule_end", v, 0x1F000);
    }

    /* size 0: the whole file */
    f = open_read(L"fmap_game.dat");
    m = mapping("map.again", f, PAGE_READONLY, 0, 0);
    v = view_of("view.whole_file", m, FILE_MAP_READ, 0, sizeof data);
    if (v) query("query.whole_file", v, 0);
    view_of("view.past_end", m, FILE_MAP_READ, sizeof data + 0x1000, sizeof data);
    view_of("view.write_access", m, FILE_MAP_WRITE, 0, sizeof data);
    view_of("view.copy_access", m, FILE_MAP_COPY, 0, sizeof data);
    CloseHandle(m);
    CloseHandle(f);

    /* a five-byte file: one page, zero after the data */
    f = open_read(L"fmap_small.dat");
    m = mapping("map.small", f, PAGE_READONLY, 0, 0);
    v = view_of("view.small", m, FILE_MAP_READ, 5, 5);
    if (v) query("query.small", v, 0);
    CloseHandle(m);
    CloseHandle(f);

    /* an empty file, a handle without read access, a mapping larger than the file */
    f = open_read(L"fmap_empty.dat");
    m = mapping("map.empty_file", f, PAGE_READONLY, 0, 0);
    if (m) CloseHandle(m);
    CloseHandle(f);
    f = CreateFileW(L"fmap_small.dat", 0, FILE_SHARE_READ | FILE_SHARE_WRITE, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL,
                    NULL);
    m = mapping("map.no_access", f, PAGE_READONLY, 0, 0);
    if (m) CloseHandle(m);
    CloseHandle(f);
    f = open_read(L"fmap_small.dat");
    m = mapping("map.larger_than_file", f, PAGE_READONLY, 0, 0x2000);
    if (m) CloseHandle(m);
    m = mapping("map.readwrite_on_read_handle", f, PAGE_READWRITE, 0, 0);
    if (m) CloseHandle(m);
    CloseHandle(f);
    m = mapping("map.closed_handle", f, PAGE_READONLY, 0, 0);
    if (m) CloseHandle(m);

    ExitProcess(0);
}
