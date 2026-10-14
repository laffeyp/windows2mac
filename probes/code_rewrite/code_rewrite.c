// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* code_rewrite.c — code written, run, rewritten and run again (Sprint 065 item 9e).
 *
 * mono writes its methods into PAGE_EXECUTE_READWRITE memory and patches them in place, with plain stores and no
 * FlushInstructionCache (game run 38216081440 calls none); a scratch frontier found 11,365 of w2m's translated blocks
 * whose guest bytes had changed by the stop. The rows: what each call returns after each kind of rewrite. No C
 * runtime.
 */
#include <windows.h>

static char out[4096];
static unsigned n;

static void put(const char *s) { while (*s && n < sizeof out - 64) out[n++] = *s++; }
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

typedef unsigned (*fn)(void);

/* mov eax, imm32; ret */
static void emit_return(volatile unsigned char *p, unsigned v) {
    p[0] = 0xB8;
    p[1] = (unsigned char)v; p[2] = (unsigned char)(v >> 8); p[3] = (unsigned char)(v >> 16); p[4] = (unsigned char)(v >> 24);
    p[5] = 0xC3;
}

void start(void) {
    volatile unsigned char *page = VirtualAlloc(NULL, 0x2000, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
    fn f = (fn)(void *)page;

    emit_return(page, 1);
    unsigned first = f();
    page[1] = 2;                                   /* the immediate only */
    unsigned second = f();
    row("immediate"); field("first", first); field("second", second); end();

    emit_return(page, 3);                          /* the whole instruction again */
    unsigned third = f();
    page[0x800] = 0x5A;                            /* data on the same page, the code untouched */
    unsigned fourth = f();
    row("rewrite"); field("third", third); field("after_data_write", fourth); end();

    /* a routine on the allocation's second page, rewritten */
    emit_return(page + 0x1000, 7);
    unsigned seven = ((fn)(void *)(page + 0x1000))();
    page[0x1001] = 8;
    unsigned eight = ((fn)(void *)(page + 0x1000))();
    row("second_page"); field("before", seven); field("after", eight); end();

    /* VirtualProtect to read-execute and back, then a rewrite */
    DWORD old = 0;
    BOOL p1 = VirtualProtect((LPVOID)page, 0x1000, PAGE_EXECUTE_READ, &old);
    unsigned ninth = f();
    DWORD old2 = 0;
    BOOL p2 = VirtualProtect((LPVOID)page, 0x1000, PAGE_EXECUTE_READWRITE, &old2);
    page[1] = 10;
    unsigned tenth = f();
    row("protect"); field("to_rx", p1); field("old", old); field("value", ninth); field("back", p2); field("old_back", old2);
    field("after_rewrite", tenth); end();

    /* a direct call from the second page into the first, then the first page rewritten (Sprint 068 item K): code
     * that reaches rewritten code by a direct branch, as a translator that chains blocks must unchain */
    volatile unsigned char *tramp = page + 0x1100;
    long rel = (long)((const unsigned char *)page - (const unsigned char *)(tramp + 5));
    tramp[0] = 0xE8;                               /* call rel32 */
    tramp[1] = (unsigned char)rel; tramp[2] = (unsigned char)(rel >> 8);
    tramp[3] = (unsigned char)(rel >> 16); tramp[4] = (unsigned char)(rel >> 24);
    tramp[5] = 0xC3;                               /* ret */
    emit_return(page, 11);
    unsigned eleventh = ((fn)(void *)tramp)();
    unsigned again = ((fn)(void *)tramp)();
    page[1] = 12;
    unsigned twelfth = ((fn)(void *)tramp)();
    row("chained"); field("first", eleventh); field("again", again); field("after_rewrite", twelfth); end();
    ExitProcess(0);
}
