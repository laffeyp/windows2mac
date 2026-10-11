// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* exceptions.c — structured exception dispatch and unwinding (Sprint 053, family "exceptions").
 *
 * No C runtime. The probe carries its own __C_specific_handler, as mono-2.0-bdwgc.dll and UnityPlayer.dll carry
 * their statically linked C runtime's: the dispatcher calls it for each frame whose UNWIND_INFO names it, it calls
 * the __except filters, and it unwinds to the taken __except with RtlUnwindEx (mono's one RtlUnwindEx call in the
 * game's Windows run). Its rules follow winnt.h's SCOPE_TABLE_AMD64: a scope with JumpTarget 0 is a __finally, a
 * HandlerAddress of 1 is the filter EXCEPTION_EXECUTE_HANDLER.
 *
 * Rows, each written as it ends: a thread-naming RaiseException taken by __except; CONTINUE_EXECUTION; the search
 * order through two frames with a __finally between; a noncontinuable exception continued; a vectored handler
 * before the frames; a raise three calls deep through a frame pointer (a realigned frame), a large allocation and saved XMM registers,
 * each frame's __finally reading its own local; access violations; the Rtl* queries on the probe's own code; last,
 * an exception no frame takes, which reaches the top-level filter.
 */
#include <windows.h>

#define SENTINEL 0xDEADu
#define CODE 0xE0577053u
#define THREAD_NAMING 0x406D1388u

int _fltused;                                            /* the linker's mark of floating point, as a C runtime defines */

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
static void list(const char *k, const int *v, int count) {
    put(",\""); put(k); put("\":[");
    for (int i = 0; i < count; i++) { if (i) put(","); putu((unsigned long long)v[i]); }
    put("]");
}
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}

/* ---- the C language-specific handler ------------------------------------------------------------------------ */

typedef LONG (*filter_fn)(EXCEPTION_POINTERS *, void *);
typedef void (*finally_fn)(BOOLEAN, void *);

EXCEPTION_DISPOSITION __C_specific_handler(EXCEPTION_RECORD *rec, void *frame, CONTEXT *ctx,
                                           DISPATCHER_CONTEXT *dc) {
    SCOPE_TABLE_AMD64 *table = (SCOPE_TABLE_AMD64 *)dc->HandlerData;
    ULONG64 base = dc->ImageBase, pc = dc->ControlPc - base;
    if (!(rec->ExceptionFlags & (EXCEPTION_UNWINDING | EXCEPTION_EXIT_UNWIND))) {
        EXCEPTION_POINTERS pointers = {rec, ctx};
        for (ULONG i = dc->ScopeIndex; i < table->Count; i++) {
            if (pc < table->ScopeRecord[i].BeginAddress || pc >= table->ScopeRecord[i].EndAddress
                || !table->ScopeRecord[i].JumpTarget)
                continue;
            ULONG h = table->ScopeRecord[i].HandlerAddress;
            LONG v = h == 1 ? EXCEPTION_EXECUTE_HANDLER : ((filter_fn)(base + h))(&pointers, frame);
            if (v < 0) return ExceptionContinueExecution;
            if (v > 0)
                RtlUnwindEx(frame, (void *)(base + table->ScopeRecord[i].JumpTarget), rec,
                            (void *)(ULONG_PTR)rec->ExceptionCode, dc->ContextRecord, dc->HistoryTable);
        }
    } else {
        ULONG64 target = (ULONG64)dc->TargetIp - base;
        for (ULONG i = dc->ScopeIndex; i < table->Count; i++) {
            if (pc < table->ScopeRecord[i].BeginAddress || pc >= table->ScopeRecord[i].EndAddress) continue;
            if ((rec->ExceptionFlags & EXCEPTION_TARGET_UNWIND) && table->ScopeRecord[i].JumpTarget == target)
                return ExceptionContinueSearch;
            if (!table->ScopeRecord[i].JumpTarget) {
                dc->ScopeIndex = i + 1;
                ((finally_fn)(base + table->ScopeRecord[i].HandlerAddress))(TRUE, frame);
            }
        }
    }
    return ExceptionContinueSearch;
}

/* ---- what the filters saw ------------------------------------------------------------------------------------ */

static int order[16], turns;
static void mark(int v) { order[turns++] = v; }

static DWORD seen_code, seen_flags, seen_params, chained_code;
static ULONG_PTR seen_info[4];
static int seen_chained, filter_calls;

static LONG seen(EXCEPTION_POINTERS *e, LONG answer) {
    EXCEPTION_RECORD *r = e->ExceptionRecord;
    filter_calls++;
    seen_code = r->ExceptionCode;
    seen_flags = r->ExceptionFlags;
    seen_params = r->NumberParameters;
    for (DWORD i = 0; i < 4 && i < r->NumberParameters; i++) seen_info[i] = r->ExceptionInformation[i];
    seen_chained = r->ExceptionRecord != NULL;
    chained_code = r->ExceptionRecord ? r->ExceptionRecord->ExceptionCode : 0;
    return answer;
}

static void reset(void) {
    turns = filter_calls = 0;
    seen_code = seen_flags = seen_params = chained_code = 0;
    seen_chained = 0;
    for (int i = 0; i < 4; i++) seen_info[i] = 0;
}

/* ---- rows ---------------------------------------------------------------------------------------------------- */

static void thread_naming(void) {
    static const char name[] = "w2m";
    ULONG_PTR args[3] = {0x1000, (ULONG_PTR)name, 0xFFFFFFFF};     /* THREADNAME_INFO: type, name, thread id */
    int reached = 0;
    DWORD code = 0;
    reset();
    SetLastError(SENTINEL);
    __try {
        RaiseException(THREAD_NAMING, 0, 3, args);
    } __except (seen(GetExceptionInformation(), EXCEPTION_EXECUTE_HANDLER)) {
        reached = 1;
        code = GetExceptionCode();
    }
    DWORD err = GetLastError();
    row("except.thread_naming"); field("reached", reached); field("code", code); field("filter_calls", filter_calls);
    field("seen_code", seen_code); field("flags", seen_flags); field("params", seen_params);
    field("args_equal", seen_info[0] == args[0] && seen_info[1] == args[1] && seen_info[2] == args[2]);
    field("chained", seen_chained); field("err", err); end();
}

static void continue_execution(void) {
    int returned = 0;
    reset();
    SetLastError(SENTINEL);
    __try {
        RaiseException(CODE, 0, 1, (ULONG_PTR[]){7});
        returned = 1;
    } __except (seen(GetExceptionInformation(), EXCEPTION_CONTINUE_EXECUTION)) {
        returned = 2;
    }
    DWORD err = GetLastError();
    row("except.continue"); field("returned", returned); field("filter_calls", filter_calls);
    field("params", seen_params); field("info0", seen_info[0]); field("err", err); end();
}

static int finally_abnormal;

__declspec(noinline) static void inner_search(void) {
    __try {
        __try {
            RaiseException(CODE, 0, 0, NULL);
        } __finally {
            mark(2);
            finally_abnormal = AbnormalTermination();
        }
    } __except (mark(1), EXCEPTION_CONTINUE_SEARCH) {
        mark(9);                                         /* never: the filter declines */
    }
}

static void search_order(void) {
    reset();
    __try {
        inner_search();
    } __except (mark(3), EXCEPTION_EXECUTE_HANDLER) {
        mark(4);
    }
    row("except.search_order"); list("order", order, turns); field("abnormal", finally_abnormal); end();
}

static LONG noncontinuable_inner(EXCEPTION_POINTERS *e) {
    mark(e->ExceptionRecord->ExceptionCode == CODE ? 1 : 2);
    return e->ExceptionRecord->ExceptionCode == CODE ? EXCEPTION_CONTINUE_EXECUTION : EXCEPTION_CONTINUE_SEARCH;
}

__declspec(noinline) static void raise_noncontinuable(void) {
    __try {
        RaiseException(CODE, EXCEPTION_NONCONTINUABLE, 0, NULL);
        mark(8);                                         /* never: a noncontinuable exception does not return */
    } __except (noncontinuable_inner(GetExceptionInformation())) {
        mark(9);
    }
}

static void noncontinuable(void) {
    int reached = 0;
    reset();
    __try {
        raise_noncontinuable();
    } __except (seen(GetExceptionInformation(), EXCEPTION_EXECUTE_HANDLER)) {
        reached = 1;
    }
    row("except.noncontinuable"); field("reached", reached); list("order", order, turns);
    field("seen_code", seen_code); field("flags", seen_flags); field("params", seen_params);
    field("chained", seen_chained); field("chained_code", chained_code); end();
}

static LONG WINAPI vectored(EXCEPTION_POINTERS *e) {
    mark(e->ExceptionRecord->ExceptionCode == CODE ? 10 : 19);
    return EXCEPTION_CONTINUE_SEARCH;
}

static void vectored_first(void) {
    reset();
    PVOID h = AddVectoredExceptionHandler(1, vectored);
    __try {
        RaiseException(CODE, 0, 0, NULL);
    } __except (mark(11), EXCEPTION_EXECUTE_HANDLER) {
        mark(12);
    }
    RemoveVectoredExceptionHandler(h);
    row("except.vectored_first"); list("order", order, turns); end();
}

/* Three frames of three prolog shapes; each __finally reads its own local through the establisher frame. */
static volatile int depth_seed = 3;
static int saw_fp, saw_big, saw_xmm;

__declspec(noinline) static void deepest(void) { RaiseException(CODE, 0, 0, NULL); }

__declspec(noinline) static double level_xmm(double x) {
    volatile double a = x * 1.5, b = x * 2.5;
    double c = a * b, d = a + b, e = a - b, f = a / b;     /* live across the call: XMM6 and up are saved */
    int local = 33;
    __try {
        deepest();
    } __finally {
        saw_xmm = local;
    }
    return c + d + e + f;
}

__declspec(noinline) static int level_big(int k) {
    volatile char big[0x800];                            /* UWOP_ALLOC_LARGE, under one page: no stack probe */
    big[k] = (char)k;
    int local = 22;
    __try {
        level_xmm((double)k);
    } __finally {
        saw_big = local;
    }
    return big[k];
}

__declspec(noinline) static int level_fp(int k) {
    _Alignas(64) volatile char p[64];                    /* realigned: a frame pointer (a variable alloca would
                                                            call __chkstk, which a program without the C runtime
                                                            lacks) */
    p[0] = (char)k;
    int local = 11;
    __try {
        level_big(k);
    } __finally {
        saw_fp = local;
    }
    return p[0];
}

static void deep(void) {
    int reached = 0;
    reset();
    __try {
        level_fp(depth_seed);
    } __except (seen(GetExceptionInformation(), EXCEPTION_EXECUTE_HANDLER)) {
        reached = 1;
    }
    row("except.deep"); field("reached", reached); field("code", seen_code);
    field("fp", saw_fp); field("big", saw_big); field("xmm", saw_xmm); end();
}

static void access_violations(void) {
    reset();
    __try {
        *(volatile int *)0 = 1;
    } __except (seen(GetExceptionInformation(), EXCEPTION_EXECUTE_HANDLER)) {
    }
    row("fault.write_null"); field("code", seen_code); field("flags", seen_flags); field("params", seen_params);
    field("info0", seen_info[0]); field("info1", seen_info[1]); end();
    reset();
    int v = 0;
    __try {
        v = *(volatile int *)0x10;
    } __except (seen(GetExceptionInformation(), EXCEPTION_EXECUTE_HANDLER)) {
    }
    row("fault.read_low"); field("code", seen_code); field("flags", seen_flags); field("params", seen_params);
    field("info0", seen_info[0]); field("info1", seen_info[1]); field("v", (unsigned)v); end();
}

__declspec(noinline) static void queries(void) {
    CONTEXT c;
    RtlCaptureContext(&c);
    ULONG64 self = (ULONG64)queries, module = (ULONG64)GetModuleHandleW(NULL);
    int rip_in_self = c.Rip > self && c.Rip < self + 0x400;
    DWORD64 base = 0;
    PRUNTIME_FUNCTION f = RtlLookupFunctionEntry(c.Rip, &base, NULL);
    int found = f != NULL, begin = f && base + f->BeginAddress == self, base_is_module = base == module;
    PVOID data = NULL;
    DWORD64 frame = 0;
    PEXCEPTION_ROUTINE handler = f ? RtlVirtualUnwind(UNW_FLAG_NHANDLER, base, c.Rip, f, &c, &data, &frame, NULL) : NULL;
    int to_caller = c.Rip == (DWORD64)__builtin_return_address(0);
    DWORD64 stack_base = 0;
    PRUNTIME_FUNCTION none = RtlLookupFunctionEntry((DWORD64)&c, &stack_base, NULL);
    PVOID image = NULL, stack_image = (PVOID)1;
    PVOID header = RtlPcToFileHeader((PVOID)self, &image);
    PVOID stack_header = RtlPcToFileHeader((PVOID)&c, &stack_image);
    row("rtl.queries"); field("rip_in_self", rip_in_self); field("found", found); field("begin", begin);
    field("base_is_module", base_is_module); field("handler_null", handler == NULL); field("to_caller", to_caller);
    field("frame_below_caller", frame != 0 && frame < c.Rsp); field("stack_entry_null", none == NULL);
    field("stack_base_left", stack_base == 0);
    field("header", header == (PVOID)module && image == (PVOID)module);
    field("stack_header_null", stack_header == NULL && stack_image == NULL); end();
}

static LONG WINAPI top_level(EXCEPTION_POINTERS *e) {
    row("unhandled.top_filter"); field("code", e->ExceptionRecord->ExceptionCode);
    field("flags", e->ExceptionRecord->ExceptionFlags); field("params", e->ExceptionRecord->NumberParameters); end();
    ExitProcess(0);
    return EXCEPTION_EXECUTE_HANDLER;
}

void start(void) {
    thread_naming();
    continue_execution();
    search_order();
    noncontinuable();
    vectored_first();
    deep();
    access_violations();
    queries();
    SetUnhandledExceptionFilter(top_level);
    RaiseException(CODE, 0, 0, NULL);
    row("unhandled.returned"); end();                    /* never: the filter ends the process */
    ExitProcess(1);
}
