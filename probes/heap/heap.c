/* heap.c — the process heap as Windows answers (Sprint 044, family "heap").
 *
 * No C runtime. The launcher's start-up reaches GetProcessHeap after its TLS and critical-section
 * set-up (frontier 2026-10-10, 1,734 instructions). This family is GetProcessHeap, HeapAlloc,
 * HeapFree, HeapSize and HeapReAlloc on the process heap. Addresses differ between runs, so rows
 * print relations (equal to PEB.ProcessHeap, the alignment, contents kept) and sizes, returns and
 * last errors (after SetLastError(0xDEAD)).
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
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void end(void) { put("}\n"); }

static unsigned long long gs(unsigned off) {
    unsigned long long v;
    __asm__ volatile("movq %%gs:(%1), %0" : "=r"(v) : "r"((unsigned long long)off));
    return v;
}

static DWORD err;

static int all_bytes(const unsigned char *p, SIZE_T len, unsigned char b) {
    for (SIZE_T i = 0; i < len; i++) if (p[i] != b) return 0;
    return 1;
}

static void fill(unsigned char *p, SIZE_T len, unsigned char b) { for (SIZE_T i = 0; i < len; i++) p[i] = b; }

void start(void) {
    SetLastError(SENTINEL);
    HANDLE h = GetProcessHeap();
    err = GetLastError();
    row("process_heap"); flag("nonzero", h != 0); flag("equals_peb_process_heap", (unsigned long long)h == *(unsigned long long *)(gs(0x60) + 0x30));
    flag("same_twice", GetProcessHeap() == h); field("err", err); end();

    static const SIZE_T sizes[] = {1, 7, 8, 15, 16, 17, 24, 100, 1000, 4096, 65536, 1048576};
    int aligned16 = 1, distinct = 1;
    void *ptrs[12];
    for (int i = 0; i < 12; i++) {
        SetLastError(SENTINEL);
        ptrs[i] = HeapAlloc(h, 0, sizes[i]);
        err = GetLastError();
        aligned16 &= ((ULONG_PTR)ptrs[i] % 16) == 0;
        for (int j = 0; j < i; j++) distinct &= ptrs[j] != ptrs[i];
        if (err != SENTINEL) break;
    }
    row("alloc.sizes"); flag("all_nonzero", ptrs[11] != 0); flag("aligned_16", aligned16); flag("distinct", distinct);
    field("err", err); end();
    int sizes_match = 1;
    for (int i = 0; i < 12; i++) sizes_match &= HeapSize(h, 0, ptrs[i]) == sizes[i];
    row("size.matches_request"); flag("all", sizes_match); end();

    unsigned char *z = HeapAlloc(h, HEAP_ZERO_MEMORY, 4096);
    row("alloc.zero_memory"); flag("zeroed", z && all_bytes(z, 4096, 0)); end();
    /* zeroing on memory that held data: a fresh heap is zero anyway, so only a block made dirty and
       freed shows whether HEAP_ZERO_MEMORY did anything (a planted fault was invisible without it) */
    unsigned char *dirty = HeapAlloc(h, 0, 4096);
    fill(dirty, 4096, 0xFF);
    HeapFree(h, 0, dirty);
    unsigned char *z2 = HeapAlloc(h, HEAP_ZERO_MEMORY, 4096);
    row("alloc.zero_memory_after_dirty_free"); flag("zeroed", z2 && all_bytes(z2, 4096, 0)); end();

    SetLastError(SENTINEL);
    void *e = HeapAlloc(h, 0, 0);
    err = GetLastError();
    row("alloc.zero_bytes"); flag("nonzero", e != 0); field("size", e ? HeapSize(h, 0, e) : 999); field("err", err); end();

    SetLastError(SENTINEL);
    void *huge = HeapAlloc(h, 0, (SIZE_T)1 << 46);
    err = GetLastError();
    row("alloc.too_large"); flag("null", huge == 0); field("err", err); end();

    unsigned char *r = HeapAlloc(h, 0, 64);
    fill(r, 64, 0x5A);
    SetLastError(SENTINEL);
    unsigned char *r2 = HeapReAlloc(h, 0, r, 100000);
    err = GetLastError();
    row("realloc.grow"); flag("nonzero", r2 != 0); flag("kept_contents", r2 && all_bytes(r2, 64, 0x5A));
    field("size", r2 ? HeapSize(h, 0, r2) : 0); field("err", err); end();
    SetLastError(SENTINEL);
    unsigned char *r3 = HeapReAlloc(h, HEAP_ZERO_MEMORY, r2, 200000);
    err = GetLastError();
    row("realloc.grow_zero"); flag("kept_contents", r3 && all_bytes(r3, 64, 0x5A));
    flag("new_bytes_zeroed", r3 && all_bytes(r3 + 100000, 100000, 0)); field("err", err); end();
    SetLastError(SENTINEL);
    unsigned char *r4 = HeapReAlloc(h, 0, r3, 16);
    err = GetLastError();
    row("realloc.shrink"); flag("kept_contents", r4 && all_bytes(r4, 16, 0x5A)); field("size", r4 ? HeapSize(h, 0, r4) : 0);
    field("err", err); end();
    SetLastError(SENTINEL);
    void *r5 = HeapReAlloc(h, HEAP_REALLOC_IN_PLACE_ONLY, r4, 1 << 24);
    err = GetLastError();
    row("realloc.in_place_only_grow"); flag("null", r5 == 0); field("err", err);
    field("old_size_kept", HeapSize(h, 0, r4)); end();

    SetLastError(SENTINEL);
    BOOL f = HeapFree(h, 0, r4);
    err = GetLastError();
    row("free"); field("ret", f != 0); field("err", err); end();
    SetLastError(SENTINEL);
    f = HeapFree(h, 0, NULL);
    err = GetLastError();
    row("free.null"); field("ret", f != 0); field("err", err); end();

    for (int i = 0; i < 12; i++) HeapFree(h, 0, ptrs[i]);
    void *again = HeapAlloc(h, 0, 16);
    row("alloc.after_frees"); flag("aligned_16", ((ULONG_PTR)again % 16) == 0); end();

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
