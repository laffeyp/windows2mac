// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* last_error_values.c — contract windows.thread.last_error.set_get 0.2.0 (Sprint 032). No C runtime,
 * as probes/skeleton. The probe reads QueryPerformanceCounter once, before any SetLastError, and
 * mixes the count into three indices of VALUE (values.h); consecutive indices differ. It then
 * makes three pairs, SetLastError(v) then GetLastError(), with only string-building code between
 * them, and writes its own guest-trace body to standard output with WriteFile: GUEST_THREAD_STARTED,
 * six GUEST_CALL_RETURNED, GUEST_THREAD_EXITED. Each SetLastError line carries the table's
 * ARGS_SHA256 entry for the value passed, so the judge can check "GetLastError returned what
 * SetLastError was given" without the value appearing in the trace or the contract.
 * GetLastError's args_sha256 is sha256 of "GetLastError()". */
#include <windows.h>
#include "values.h"

#define PAIRS 3

static char out[2048];
static unsigned n;

static void put(const char *s) {
    while (*s) out[n++] = *s++;
}

static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do {
        t[k++] = (char)('0' + v % 10);
        v /= 10;
    } while (v);
    while (k) out[n++] = t[--k];
}

static void head(const char *tag, DWORD tid, unsigned seq) {
    put("{\"tag\":\"");
    put(tag);
    put("\",\"thread_id\":");
    putu(tid);
    put(",\"thread_seq\":");
    putu(seq);
    put(",\"hb_refs\":[],");
}

void start(void) {
    DWORD tid = GetCurrentThreadId();
    LARGE_INTEGER count;
    QueryPerformanceCounter(&count);
    unsigned long long x = (unsigned long long)count.QuadPart;
    x ^= x >> 8;
    x ^= x >> 16;
    x ^= x >> 32;

    head("GUEST_THREAD_STARTED", tid, 0);
    put("\"parent_thread_id\":0,\"start_address\":");
    putu((unsigned long long)(ULONG_PTR)start);
    put(",\"suspended\":false}\n");

    unsigned idx = (unsigned)(x & 0xFF);
    unsigned seq = 1;
    unsigned mismatches = 0;
    for (int p = 0; p < PAIRS; p++) {
        SetLastError(VALUE[idx]);
        head("GUEST_CALL_RETURNED", tid, seq++);
        put("\"api\":\"kernel32!SetLastError\",\"args_sha256\":\"");
        put(ARGS_SHA256[idx]);
        put("\",\"return_value\":0,\"last_error\":0}\n");

        DWORD e = GetLastError();
        head("GUEST_CALL_RETURNED", tid, seq++);
        put("\"api\":\"kernel32!GetLastError\",\"args_sha256\":"
            "\"sha256:d767c2ccfb34d58d8fbdc63817ba854589de1374265ca5f5ee8860fae59590ed\","
            "\"return_value\":");
        putu(e);
        put(",\"last_error\":0}\n");
        if (e != VALUE[idx]) mismatches++;

        x >>= 7;
        idx = (idx + 1 + (unsigned)(x & 0x7F)) & 0xFF;   /* offset 1..128: never the same index */
    }

    UINT code = mismatches ? 1 : 0;
    head("GUEST_THREAD_EXITED", tid, seq);
    put("\"exit_code\":");
    putu(code);
    put("}\n");

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(code);
}
