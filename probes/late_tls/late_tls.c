// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* late_tls.c — static TLS in a DLL loaded with LoadLibrary after the process started (Sprint 065, family "late TLS").
 *
 * No C runtime, as static_tls.c. The frontier of 2026-10-12 stops at LoadLibrary of steam_api64.dll (an 8-byte TLS
 * template, no zero fill, no callbacks) after 50,784,721,550 instructions. This program, without TLS, starts a worker
 * that waits, loads late_tls_dll.dll (which it does not import) and lets the worker go, then starts a third thread.
 * Rows: the DLL's TLS index; whether TEB.ThreadLocalStoragePointer was set before the load, is set after it, and kept
 * its value; for each thread (main, the worker started before the load, the thread started after it) the marker it
 * reads (DllMain adds 1 on the loading thread), whether its zero fill is zero and whether its marker lies in the block
 * its slot holds; whether the three blocks are distinct.
 */
#include <windows.h>

typedef ULONG (*index_fn)(void);
typedef unsigned long long *(*marker_fn)(void);
typedef unsigned char *(*zeros_fn)(void);

static char out[4096];
static unsigned n;
static marker_fn marker;
static zeros_fn zeros;
static ULONG index;
static HANDLE go;
static struct seen { unsigned long long value; int zero, in_slot; unsigned long long *at; } seen[3];

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

static void **tls_array(void) {
    void **a;
    __asm__ volatile("movq %%gs:0x58, %0" : "=r"(a));
    return a;
}

static void look(int who) {
    unsigned long long *m = marker();
    unsigned char *z = zeros();
    int zero = 1;
    for (int i = 0; i < 32; i++) zero &= z[i] == 0;
    void **a = tls_array();
    unsigned char *block = a ? (unsigned char *)a[index] : 0;
    seen[who].value = *m;
    seen[who].zero = zero;
    seen[who].in_slot = block && (unsigned char *)m >= block && (unsigned char *)m < block + 4096;
    seen[who].at = m;
}

static DWORD WINAPI worker(void *arg) {
    (void)arg;
    WaitForSingleObject(go, INFINITE);
    look(1);
    return 0;
}

static DWORD WINAPI later(void *arg) {
    (void)arg;
    look(2);
    return 0;
}

void start(void) {
    go = CreateEventW(NULL, TRUE, FALSE, NULL);
    HANDLE w = CreateThread(NULL, 0, worker, NULL, 0, NULL);
    void **before = tls_array();
    HMODULE m = LoadLibraryW(L"late_tls_dll.dll");
    void **after = tls_array();
    if (m) {
        index = ((index_fn)GetProcAddress(m, "late_index"))();
        marker = (marker_fn)GetProcAddress(m, "late_marker");
        zeros = (zeros_fn)GetProcAddress(m, "late_zeros");
        look(0);
        SetEvent(go);
        WaitForSingleObject(w, INFINITE);
        HANDLE l = CreateThread(NULL, 0, later, NULL, 0, NULL);
        WaitForSingleObject(l, INFINITE);
    }
    row("late.loaded"); flag("value", m != 0); end();
    row("late.index"); field("value", index); end();
    row("late.array"); flag("set_before", before != 0); flag("set_after", after != 0);
    flag("kept", before == after); end();
    static const char *names[] = {"late.main", "late.before_load", "late.after_load"};
    for (int i = 0; i < 3; i++) {
        row(names[i]); field("marker", seen[i].value); flag("zeros_zero", seen[i].zero); flag("in_slot", seen[i].in_slot);
        end();
    }
    row("late.blocks"); flag("distinct", seen[0].at != seen[1].at && seen[1].at != seen[2].at && seen[0].at != seen[2].at);
    end();
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
