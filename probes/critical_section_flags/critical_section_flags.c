// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* critical_section_flags.c — InitializeCriticalSectionEx's flag forms (Sprint 049, family "critical section flags").
 *
 * No C runtime. mono-2.0-bdwgc.dll's start-up calls InitializeCriticalSectionEx(cs, 0,
 * CRITICAL_SECTION_NO_DEBUG_INFO) (look-ahead 2026-10-11, 2,742,280 instructions); Sprint 044's rows held only
 * spin counts with and without that flag. Rows: the return, the last error (after SetLastError(0xDEAD)) and the
 * structure's fields after the call, for spin 0 and 4000 with each flag a caller may pass.
 */
#include <windows.h>

#define SENTINEL 0xDEADu

static char out[4096];
static unsigned n;

static void put(const char *s) { while (*s) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void puts_(long long v) { if (v < 0) { out[n++] = '-'; v = -v; } putu((unsigned long long)v); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }

static void one(const char *name, DWORD spin, DWORD flags) {
    CRITICAL_SECTION cs;
    for (unsigned i = 0; i < sizeof cs; i++) ((volatile BYTE *)&cs)[i] = 0xA5;
    SetLastError(SENTINEL);
    BOOL ok = InitializeCriticalSectionEx(&cs, spin, flags);
    DWORD err = GetLastError();
    put("{\"s\":\""); put(name); put("\"");
    field("ret", ok != 0); field("err", err);
    put(",\"debug_info\":"); puts_((long long)(LONG_PTR)cs.DebugInfo);
    put(",\"lock_count\":"); puts_(cs.LockCount);
    put(",\"recursion\":"); puts_(cs.RecursionCount);
    field("owner", (ULONG_PTR)cs.OwningThread); field("semaphore", (ULONG_PTR)cs.LockSemaphore);
    field("spin", cs.SpinCount);
    put("}\n");
    DWORD written;                                       /* each row written as it ends (KIT_DIARY #37) */
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
    if (ok && !(flags & 0x04000000)) DeleteCriticalSection(&cs);   /* STATIC_INIT leaves the bytes as they were
                                                                        * (run 38113744018): nothing to delete */
}

void start(void) {
    one("spin0.no_debug_info", 0, CRITICAL_SECTION_NO_DEBUG_INFO);
    one("spin4000.no_debug_info", 4000, CRITICAL_SECTION_NO_DEBUG_INFO);
    one("spin0.none", 0, 0);
    one("spin4000.none", 4000, 0);
    one("spin0.dynamic_spin", 0, 0x02000000);            /* RTL_CRITICAL_SECTION_FLAG_DYNAMIC_SPIN */
    one("spin0.static_init", 0, 0x04000000);             /* RTL_CRITICAL_SECTION_FLAG_STATIC_INIT */
    one("spin0.no_debug_dynamic", 0, CRITICAL_SECTION_NO_DEBUG_INFO | 0x02000000);
    ExitProcess(0);
}
