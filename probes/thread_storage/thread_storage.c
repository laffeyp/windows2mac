/* thread_storage.c — TLS and FLS as Windows answers on one thread (Sprint 044, family "thread storage").
 *
 * No C runtime. The launcher's start-up reaches FlsAlloc through an API set (frontier 2026-10-10,
 * 378 instructions); Mono reads TLS slots straight from the TEB (gs:[0x1480 + key*8], Sprint 036),
 * and TlsGetValue is the game's second-hottest import (23,291,261 calls in 180 s). This family is
 * TlsAlloc, TlsGetValue, TlsSetValue, TlsFree, FlsAlloc, FlsGetValue, FlsSetValue, FlsFree. Rows give
 * the indices a fresh process receives, values, returns, last errors (after SetLastError(0xDEAD)),
 * and where TlsSetValue's value lives: TEB.TlsSlots (+0x1480) for indices below 64, the array at
 * TEB.TlsExpansionSlots (+0x1780) above.
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
static volatile LONG callbacks;
static PVOID callback_value;
static void WINAPI fls_callback(PVOID value) {
    callbacks++;
    callback_value = value;
}

void start(void) {
    /* TLS */
    SetLastError(SENTINEL);
    DWORD t1 = TlsAlloc();
    err = GetLastError();
    row("tls.alloc.first"); field("index", t1); field("err", err); end();
    DWORD t2 = TlsAlloc();
    row("tls.alloc.second"); field("index", t2); end();

    SetLastError(SENTINEL);
    LPVOID v = TlsGetValue(t1);
    err = GetLastError();
    row("tls.get.fresh"); flag("null", v == 0); field("err", err); end();

    SetLastError(SENTINEL);
    BOOL ok = TlsSetValue(t1, (LPVOID)0x1234567890ull);
    err = GetLastError();
    row("tls.set"); field("ret", ok != 0); field("err", err);
    flag("teb_slot_holds_value", gs(0x1480 + 8 * t1) == 0x1234567890ull); end();

    SetLastError(SENTINEL);
    v = TlsGetValue(t1);
    err = GetLastError();
    row("tls.get.after_set"); flag("value_returned", v == (LPVOID)0x1234567890ull); field("err", err); end();

    SetLastError(SENTINEL);
    v = TlsGetValue(5000);
    err = GetLastError();
    row("tls.get.out_of_range"); flag("null", v == 0); field("err", err); end();

    SetLastError(SENTINEL);
    ok = TlsSetValue(5000, (LPVOID)1);
    err = GetLastError();
    row("tls.set.out_of_range"); field("ret", ok != 0); field("err", err); end();

    /* beyond the 64 TEB slots: the expansion array */
    static DWORD more[80];
    DWORD last = 0;
    int below = 0, above = 0;
    for (int i = 0; i < 80; i++) {
        more[i] = TlsAlloc();
        if (more[i] == TLS_OUT_OF_INDEXES) break;
        if (more[i] < 64) below++; else above++;
        last = more[i];
    }
    row("tls.alloc.many"); field("below_64", below); field("above_64", above); field("last", last); end();
    row("tls.alloc.many.indices");                       /* every index received, in order */
    put(",\"indices\":[");
    for (int i = 0; i < 80 && more[i] != TLS_OUT_OF_INDEXES; i++) { if (i) put(","); putu(more[i]); }
    put("]"); end();
    SetLastError(SENTINEL);
    ok = TlsSetValue(last, (LPVOID)0xABCDEFull);
    err = GetLastError();
    unsigned long long *expansion = (unsigned long long *)gs(0x1780);
    row("tls.set.expansion"); field("ret", ok != 0); field("err", err); flag("expansion_present", expansion != 0);
    flag("expansion_holds_value", expansion != 0 && last >= 64 && expansion[last - 64] == 0xABCDEFull);
    flag("value_returned", TlsGetValue(last) == (LPVOID)0xABCDEFull); end();

    SetLastError(SENTINEL);
    ok = TlsFree(t2);
    err = GetLastError();
    row("tls.free"); field("ret", ok != 0); field("err", err); end();
    DWORD again = TlsAlloc();
    row("tls.alloc.after_free"); flag("reuses_freed", again == t2); end();
    SetLastError(SENTINEL);
    v = TlsGetValue(again);
    err = GetLastError();
    row("tls.get.reused"); flag("null", v == 0); field("err", err); end();

    /* FLS */
    SetLastError(SENTINEL);
    DWORD f1 = FlsAlloc(fls_callback);
    err = GetLastError();
    row("fls.alloc.first"); field("index", f1); field("err", err); end();
    DWORD f2 = FlsAlloc(NULL);
    row("fls.alloc.second"); field("index", f2); end();

    SetLastError(SENTINEL);
    PVOID fv = FlsGetValue(f1);
    err = GetLastError();
    row("fls.get.fresh"); flag("null", fv == 0); field("err", err); end();

    SetLastError(SENTINEL);
    ok = FlsSetValue(f1, (PVOID)0x5555);
    err = GetLastError();
    row("fls.set"); field("ret", ok != 0); field("err", err); end();

    SetLastError(SENTINEL);
    fv = FlsGetValue(f1);
    err = GetLastError();
    row("fls.get.after_set"); flag("value_returned", fv == (PVOID)0x5555); field("err", err); end();

    SetLastError(SENTINEL);
    fv = FlsGetValue(5000);
    err = GetLastError();
    row("fls.get.out_of_range"); flag("null", fv == 0); field("err", err); end();

    SetLastError(SENTINEL);
    ok = FlsFree(f2);
    err = GetLastError();
    row("fls.free.no_callback"); field("ret", ok != 0); field("err", err); field("callbacks", callbacks); end();

    {   /* the callback runs in the program's code: everything so far is written first, so an
           implementation that cannot run it yet loses only the last row */
        DWORD w;
        WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &w, NULL);
        n = 0;
    }
    SetLastError(SENTINEL);
    ok = FlsFree(f1);
    err = GetLastError();
    row("fls.free.with_callback"); field("ret", ok != 0); field("err", err); field("callbacks", callbacks);
    flag("callback_got_value", callback_value == (PVOID)0x5555); end();

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
