// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* heap_in_place.c — how far HeapReAlloc(HEAP_REALLOC_IN_PLACE_ONLY) grows a process-heap block (Sprint 065).
 *
 * No C runtime. Game run 38216081440 seq 3265-3268: UnityPlayer grows blocks in place (flags 0x10) to 0x817, 0x1017
 * and 0x2017; Windows refuses all four (NULL, ERROR_NOT_ENOUGH_MEMORY), and the caller takes a new block. w2m's heap
 * granted three (frontier 643a6283510c-1791802134771149000 seq 1171399-1173530): its block, HeapAlloc(0x417), came off
 * the free list with a larger capacity. The family "heap" has one in-place row (growth to 16 MB fails). Addresses
 * differ between runs, so these rows print sizes and outcomes:
 *   - limit.<state>.<size>: the largest size a block of `size` grows to in place, found by growing it one byte at a
 *     time until a refusal (to at most size + 0x2000), and the refusal's last error; the step between two blocks
 *     allocated one after the other. State "fresh": the block, then a second one of the same size, so the first's
 *     neighbour is busy when they are adjacent. State "warm": 40 blocks of the size first (the low-fragmentation heap
 *     takes a size after repeated requests), then the same pair.
 *   - neighbour_freed: three 0x417 blocks; the middle one freed; the first grown in place to 0x817.
 */
#include <windows.h>

#define SENTINEL 0xDEADu

static char out[16384];
static unsigned n;

static void put(const char *s) { while (*s) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void puts16(unsigned long long v) {
    static const char hex[] = "0123456789abcdef";
    char t[24];
    int k = 0;
    do { t[k++] = hex[v & 15]; v >>= 4; } while (v);
    while (k) out[n++] = t[--k];
}
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void flush(void) {
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}
static void end(void) { put("}\n"); flush(); }

static HANDLE h;

static void limit(const char *state, SIZE_T size) {
    unsigned char *p = HeapAlloc(h, 0, size);
    unsigned char *q = HeapAlloc(h, 0, size);
    SIZE_T grown = size;
    DWORD err = 0;
    while (grown < size + 0x2000) {
        SetLastError(SENTINEL);
        if (!HeapReAlloc(h, HEAP_REALLOC_IN_PLACE_ONLY, p, grown + 1)) { err = GetLastError(); break; }
        grown++;
    }
    put("{\"s\":\"limit."); put(state); put("."); puts16(size); put("\"");
    field("limit", grown); field("err", err); field("size_after", HeapSize(h, 0, p));
    field("step_forward", q > p ? (unsigned long long)(q - p) : 0);
    field("step_back", p > q ? (unsigned long long)(p - q) : 0); end();
    HeapFree(h, 0, p);
    HeapFree(h, 0, q);
}

void start(void) {
    h = GetProcessHeap();
    static const SIZE_T sizes[] = {1, 0x10, 0x18, 0x100, 0x417, 0x800, 0x1000, 0x4000, 0x10000};
    for (unsigned i = 0; i < sizeof sizes / sizeof sizes[0]; i++) limit("fresh", sizes[i]);
    for (unsigned i = 0; i < sizeof sizes / sizeof sizes[0]; i++) {
        void *warm[40];
        for (int k = 0; k < 40; k++) warm[k] = HeapAlloc(h, 0, sizes[i]);
        limit("warm", sizes[i]);
        for (int k = 0; k < 40; k++) HeapFree(h, 0, warm[k]);
    }

    unsigned char *a = HeapAlloc(h, 0, 0x417), *b = HeapAlloc(h, 0, 0x417), *c = HeapAlloc(h, 0, 0x417);
    HeapFree(h, 0, b);
    SetLastError(SENTINEL);
    void *r = HeapReAlloc(h, HEAP_REALLOC_IN_PLACE_ONLY, a, 0x817);
    DWORD err = GetLastError();
    put("{\"s\":\"neighbour_freed\""); field("granted", r != 0); field("same", r == a); field("err", err);
    field("b_after_a", b > a ? (unsigned long long)(b - a) : 0); end();
    HeapFree(h, 0, a);
    HeapFree(h, 0, c);
    ExitProcess(0);
}
