// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* thread_handles.c — real handles from pseudo-handles, duplicates, semaphores, OpenThread (Sprint 049, family
 * "thread handles").
 *
 * No C runtime. mono-2.0-bdwgc.dll's start-up calls DuplicateHandle(GetCurrentProcess(), GetCurrentThread(),
 * GetCurrentProcess(), &h, 0, FALSE, DUPLICATE_SAME_ACCESS) (frontier 2026-10-11, 2,977,521 instructions; the
 * game's trace, run 38113342549, seq 809 and 874), then CreateSemaphoreW(NULL, 0 or 1, 0x7FFFFFFF, NULL) (seq 852)
 * and OpenThread(THREAD_ALL_ACCESS, FALSE, tid) (seq 1141). Handles differ between runs, so rows print relations:
 * what GetThreadId, GetProcessId and waits say of a handle, whether two handles differ, multiples of 4; each with
 * its last error after SetLastError(0xDEAD). Rows are written as they end.
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
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}
static int real(HANDLE h) { return h && h != INVALID_HANDLE_VALUE && ((ULONG_PTR)h & 3) == 0 && (LONG_PTR)h > 0; }

void start(void) {
    HANDLE me = GetCurrentProcess(), self = GetCurrentThread();
    row("pseudo"); field("process_is_minus_1", me == (HANDLE)-1); field("thread_is_minus_2", self == (HANDLE)-2); end();

    HANDLE t = NULL;
    SetLastError(SENTINEL);
    BOOL ok = DuplicateHandle(me, self, me, &t, 0, FALSE, DUPLICATE_SAME_ACCESS);
    DWORD err = GetLastError();
    row("dup.thread"); field("ret", ok != 0); field("real", real(t)); field("err", err);
    field("thread_id_is_current", GetThreadId(t) == GetCurrentThreadId()); end();
    HANDLE t2 = NULL;
    DuplicateHandle(me, self, me, &t2, 0, FALSE, DUPLICATE_SAME_ACCESS);
    row("dup.thread_again"); field("distinct", t2 != t); field("thread_id_is_current", GetThreadId(t2) == GetCurrentThreadId()); end();
    SetLastError(SENTINEL);
    DWORD w = WaitForSingleObject(t, 0);
    err = GetLastError();
    row("wait.own_thread"); field("ret", w); field("err", err); end();

    HANDLE p = NULL;
    SetLastError(SENTINEL);
    ok = DuplicateHandle(me, me, me, &p, 0, FALSE, DUPLICATE_SAME_ACCESS);
    err = GetLastError();
    row("dup.process"); field("ret", ok != 0); field("real", real(p)); field("err", err);
    field("process_id_is_current", GetProcessId(p) == GetCurrentProcessId()); end();

    HANDLE e = CreateEventW(NULL, TRUE, FALSE, NULL), e2 = NULL;
    SetLastError(SENTINEL);
    ok = DuplicateHandle(me, e, me, &e2, 0, FALSE, DUPLICATE_SAME_ACCESS);
    err = GetLastError();
    SetEvent(e2);
    row("dup.event"); field("ret", ok != 0); field("distinct", e2 != e); field("err", err);
    field("set_through_copy_seen", WaitForSingleObject(e, 0) == WAIT_OBJECT_0); end();
    HANDLE e3 = NULL;
    SetLastError(SENTINEL);
    ok = DuplicateHandle(me, e2, me, &e3, 0, FALSE, DUPLICATE_SAME_ACCESS | DUPLICATE_CLOSE_SOURCE);
    err = GetLastError();
    SetLastError(SENTINEL);
    BOOL closed_again = CloseHandle(e2);
    DWORD err2 = GetLastError();
    row("dup.close_source"); field("ret", ok != 0); field("err", err); field("source_close_ret", closed_again != 0);
    field("source_close_err", err2); field("copy_signaled", WaitForSingleObject(e3, 0) == WAIT_OBJECT_0); end();

    HANDLE bad = (HANDLE)0x1234, out_h = (HANDLE)0x5A5A;
    SetLastError(SENTINEL);
    ok = DuplicateHandle(me, bad, me, &out_h, 0, FALSE, DUPLICATE_SAME_ACCESS);
    err = GetLastError();
    row("dup.bad_source"); field("ret", ok != 0); field("err", err); field("out_untouched", out_h == (HANDLE)0x5A5A);
    field("out_null", out_h == NULL); end();

    SetLastError(SENTINEL);
    BOOL c = CloseHandle(t);
    err = GetLastError();
    row("close.thread"); field("ret", c != 0); field("err", err); end();
    CloseHandle(t2); CloseHandle(p); CloseHandle(e); CloseHandle(e3);

    for (int initial = 0; initial <= 1; initial++) {
        SetLastError(SENTINEL);
        HANDLE s = CreateSemaphoreW(NULL, initial, 0x7FFFFFFF, NULL);
        err = GetLastError();
        row(initial ? "semaphore.one" : "semaphore.zero"); field("real", real(s)); field("err", err);
        field("wait", WaitForSingleObject(s, 0)); end();
        CloseHandle(s);
    }
    SetLastError(SENTINEL);
    HANDLE bad_sem = CreateSemaphoreW(NULL, 2, 1, NULL);
    err = GetLastError();
    row("semaphore.initial_above_max"); field("null", bad_sem == NULL); field("err", err); end();

    SetLastError(SENTINEL);
    HANDLE o = OpenThread(THREAD_ALL_ACCESS, FALSE, GetCurrentThreadId());
    err = GetLastError();
    row("open.current"); field("real", real(o)); field("err", err);
    field("thread_id_is_current", GetThreadId(o) == GetCurrentThreadId()); end();
    CloseHandle(o);
    SetLastError(SENTINEL);
    o = OpenThread(THREAD_ALL_ACCESS, FALSE, 0xFFFFFFF0u);
    err = GetLastError();
    row("open.missing"); field("null", o == NULL); field("err", err); end();
    ExitProcess(0);
}
