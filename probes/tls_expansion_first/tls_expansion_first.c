// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* tls_expansion_first.c — the first thread uses a TLS index past the 64 TEB slots that another thread allocated
 * (Sprint 347; found by builder-347's experiment).
 *
 * No C runtime. A worker takes TLS indices until one lies past the 64 TEB slots, then ends. The first thread, which
 * has taken none past 63, sets its own value at that index and reads it back. Microsoft documents TlsAlloc: "After a
 * TLS index has been allocated, each thread of the process can use it to access its own TLS storage slot." w2m gave
 * the first thread its expansion array only on its own TlsAlloc past 63, so its TlsSetValue stopped the run. Rows: the
 * index's range, TlsSetValue's result, the value read back and the last error after the read.
 */
#include <windows.h>

static char out[256];
static unsigned n;

static void put(const char *s) { while (*s && n < sizeof out - 32) out[n++] = *s++; }
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

static DWORD high = TLS_OUT_OF_INDEXES;

static DWORD WINAPI worker(LPVOID param) {
    (void)param;
    for (int k = 0; k < 80; k++) {
        DWORD x = TlsAlloc();
        if (x == TLS_OUT_OF_INDEXES) break;
        if (x >= 64) { high = x; break; }
    }
    return 0;
}

void start(void) {
    DWORD tid;
    HANDLE h = CreateThread(NULL, 0, worker, NULL, 0, &tid);
    DWORD w = WaitForSingleObject(h, INFINITE);
    BOOL found = high != TLS_OUT_OF_INDEXES;
    BOOL set = found && TlsSetValue(high, (LPVOID)(ULONG_PTR)0x5E7);
    SetLastError(0x77);
    ULONG_PTR got = found ? (ULONG_PTR)TlsGetValue(high) : 0;
    DWORD error = GetLastError();
    row("expansion_first");
    field("wait", w);
    field("high_at_least_64", found && high >= 64);
    field("set", set != 0);
    field("got", got);
    field("error_after_get", error);
    end();
    ExitProcess(0);
}
