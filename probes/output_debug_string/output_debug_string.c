// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* output_debug_string.c — OutputDebugStringA and OutputDebugStringW with no debugger (Sprint 065, family "debug
 * string").
 *
 * No C runtime. steam_api64.dll, which the game loads after 50.78 billion instructions (frontier 2026-10-12), imports
 * OutputDebugStringA. A vectored handler records each exception it sees (code, flags, parameter count and the first
 * two parameters, the second as an offset from the string when it points into it) and continues the search. Rows:
 * per call (A with a short string, A with an empty one, W with a short string), the exceptions seen in order and the
 * last error after SetLastError(0xDEAD).
 */
#include <windows.h>

static char out[4096];
static unsigned n;
static struct seen { DWORD code, flags, count; ULONG_PTR p0, p1; } seen[8];
static unsigned seen_n;

static void put(const char *s) { while (*s) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void end(void) { put("}\n"); }

static LONG CALLBACK handler(EXCEPTION_POINTERS *ep) {
    EXCEPTION_RECORD *r = ep->ExceptionRecord;
    if (seen_n < 8) {
        seen[seen_n].code = r->ExceptionCode;
        seen[seen_n].flags = r->ExceptionFlags;
        seen[seen_n].count = r->NumberParameters;
        seen[seen_n].p0 = r->NumberParameters > 0 ? r->ExceptionInformation[0] : 0;
        seen[seen_n].p1 = r->NumberParameters > 1 ? r->ExceptionInformation[1] : 0;
        seen_n++;
    }
    return EXCEPTION_CONTINUE_SEARCH;
}

static void report(const char *name, const void *text, DWORD err) {
    row(name); field("error", err); field("exceptions", seen_n); end();
    for (unsigned i = 0; i < seen_n; i++) {
        row(name); field("exception", i); field("code", seen[i].code); field("flags", seen[i].flags);
        field("parameters", seen[i].count); field("p0", seen[i].p0);
        put(",\"p1_is_text\":"); put(seen[i].p1 == (ULONG_PTR)text ? "true" : "false"); end();
    }
    seen_n = 0;
}

void start(void) {
    AddVectoredExceptionHandler(1, handler);
    static const char a[] = "w2m debug string\n";
    static const WCHAR w[] = L"w2m wide string\n";
    SetLastError(0xDEAD);
    OutputDebugStringA(a);
    report("a.short", a, GetLastError());
    SetLastError(0xDEAD);
    OutputDebugStringA("");
    report("a.empty", "", GetLastError());
    SetLastError(0xDEAD);
    OutputDebugStringW(w);
    report("w.short", w, GetLastError());
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
