// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* vectored_handlers.c — vectored exception handlers (Sprint 049, family "vectored handlers").
 *
 * No C runtime. mono-2.0-bdwgc.dll's start-up calls AddVectoredExceptionHandler (frontier 2026-10-11, 2,898,399
 * instructions); mono's handler is how it turns faults in managed code into exceptions. Rows, each written as it
 * ends: registration (Add first and last, Remove, Remove again; handles compared, never printed; last error after
 * SetLastError(0xDEAD)), then dispatch: RaiseException with three handlers registered, each recording its turn,
 * the code and flags it saw and the record's address field, and answering CONTINUE_SEARCH or CONTINUE_EXECUTION.
 * Dispatch comes last: w2m registers handlers but does not dispatch yet.
 */
#include <windows.h>

#define SENTINEL 0xDEADu
#define CODE 0xE0577001u

static char out[4096];
static unsigned n;
static int turn, order[8], answers[8];
static DWORD seen_code[8], seen_flags[8];

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

static LONG handle(int who, EXCEPTION_POINTERS *e) {
    order[turn] = who;
    seen_code[turn] = e->ExceptionRecord->ExceptionCode;
    seen_flags[turn] = e->ExceptionRecord->ExceptionFlags;
    turn++;
    return answers[who];
}
static LONG WINAPI handler_a(EXCEPTION_POINTERS *e) { return handle(1, e); }
static LONG WINAPI handler_b(EXCEPTION_POINTERS *e) { return handle(2, e); }
static LONG WINAPI handler_c(EXCEPTION_POINTERS *e) { return handle(3, e); }

void start(void) {
    SetLastError(SENTINEL);
    PVOID a = AddVectoredExceptionHandler(0, handler_a);
    DWORD err = GetLastError();
    row("add.last"); field("non_null", a != NULL); field("err", err); end();
    SetLastError(SENTINEL);
    PVOID b = AddVectoredExceptionHandler(1, handler_b);
    err = GetLastError();
    row("add.first"); field("non_null", b != NULL); field("distinct", b != a); field("err", err); end();
    PVOID again = AddVectoredExceptionHandler(0, handler_a);
    row("add.same_handler"); field("distinct", again != a); end();
    SetLastError(SENTINEL);
    ULONG r = RemoveVectoredExceptionHandler(again);
    err = GetLastError();
    row("remove"); field("ret", r); field("err", err); end();
    SetLastError(SENTINEL);
    r = RemoveVectoredExceptionHandler(again);
    err = GetLastError();
    row("remove.again"); field("ret", r); field("err", err); end();
    SetLastError(SENTINEL);
    r = RemoveVectoredExceptionHandler((PVOID)0x12345670);
    err = GetLastError();
    row("remove.unknown"); field("ret", r); field("err", err); end();

    /* Dispatch: b (first), then a (last), then c (last, added now); a answers CONTINUE_EXECUTION. */
    PVOID c = AddVectoredExceptionHandler(0, handler_c);
    answers[1] = EXCEPTION_CONTINUE_EXECUTION;
    answers[2] = EXCEPTION_CONTINUE_SEARCH;
    answers[3] = EXCEPTION_CONTINUE_SEARCH;
    turn = 0;
    SetLastError(SENTINEL);
    RaiseException(CODE, 0, 0, NULL);
    err = GetLastError();
    row("dispatch.continue"); field("turns", turn);
    for (int i = 0; i < turn; i++) { put(i ? "," : ",\"order\":["); putu(order[i]); }
    if (turn) put("]");
    field("code_seen", turn && seen_code[0] == CODE); field("flags_seen", turn ? seen_flags[0] : 99);
    field("err", err); end();

    RemoveVectoredExceptionHandler(a);
    answers[2] = EXCEPTION_CONTINUE_EXECUTION;
    turn = 0;
    RaiseException(CODE, 0, 0, NULL);
    row("dispatch.after_remove"); field("turns", turn);
    for (int i = 0; i < turn; i++) { put(i ? "," : ",\"order\":["); putu(order[i]); }
    if (turn) put("]");
    end();
    RemoveVectoredExceptionHandler(b);
    RemoveVectoredExceptionHandler(c);
    ExitProcess(0);
}
