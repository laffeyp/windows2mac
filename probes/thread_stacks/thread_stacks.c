// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* thread_stacks.c — a created thread's stack, and how it grows (Sprint 066 item 4, plan row 066).
 *
 * No C runtime. contracts/windows.memory.thread_stack holds the main thread's stack only; the game's 28 threads are
 * CreateThread(NULL, 0, ...) (captures 38113342549, 38113343972). Each subject thread reads its TEB's
 * NtTib.StackBase (+0x08), NtTib.StackLimit (+0x10) and DeallocationStack (+0x1478) at its start, then waits; the
 * main thread queries the subject's stack pages with VirtualQuery while it waits, as mono's collector queries
 * another thread's stack. Addresses differ between runs, so every address prints as a relation to the subject's
 * own TEB fields (where()) and sizes print in 4 KB pages.
 *
 * Subjects: "default" CreateThread(NULL, 0, ..., 0) (the game's form); "reserve" 0x40000 with
 * STACK_SIZE_PARAM_IS_A_RESERVATION; "commit" 0x20000 without it. "default" then grows its stack: it touches the
 * pages below its StackLimit one at a time, top down, as __chkstk does, and records StackLimit after each touch;
 * the main thread queries again. "skip" touches a page 16 below its StackLimit without touching the ones between,
 * under a vectored handler that records the exception and ends the thread.
 *
 * Rows (each address as where() names it):
 *   <subject>.teb          reserve_pages (StackBase - DeallocationStack), header_reserve_pages and
 *                          header_commit_pages (the image's SizeOfStackReserve and SizeOfStackCommit), committed_pages
 *                          (StackBase - StackLimit), sp_inside, aligned, limit_above_dealloc
 *   <subject>.limits       GetCurrentThreadStackLimits from the subject: low, high, err
 *   <subject>.query.<at>   VirtualQuery from the main thread at sp (no base_address: it depends on the frames), base-1,
 *                          limit, limit-1, limit-2p .. limit-5p, dealloc: ret_is_size, err, base_address,
 *                          allocation_base, end, allocation_protect, state, protect, type
 *   default.grow           limit_unchanged (StackLimit before the touches equals the one read at the start), then
 *                          moved_1 .. moved_8: pages StackLimit has moved down after touching 1 .. 8 pages
 *   default.grown.<at>     the queries again, relative to the moved StackLimit
 *   skip                   code (the exception the handler saw, 0 if none), write (ExceptionInformation[0]),
 *                          at_touched (ExceptionInformation[1] is the touched address), moved (pages StackLimit moved),
 *                          exit_code
 * Each row after SetLastError(0xDEAD) where it names err.
 */
#include <windows.h>

#define SENTINEL 0xDEADu
#define PAGE 4096u
#define GROW 8
#define SKIP 16

extern IMAGE_DOS_HEADER __ImageBase;

static char out[4096];
static unsigned n;
static ULONG_PTR base, limit, dealloc;          /* the subject's, for where() */

static void put(const char *s) { while (*s && n < sizeof out - 64) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *subject, const char *s) {
    put("{\"s\":\""); put(subject); if (*s) { put("."); put(s); } put("\"");
}
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}

/* An address as a name: one of the subject's TEB fields, or its distance in pages from StackLimit. */
static void where(const char *k, ULONG_PTR v) {
    put(",\""); put(k); put("\":");
    if (v == base) put("\"base\"");
    else if (v == limit) put("\"limit\"");
    else if (v == dealloc) put("\"dealloc\"");
    else if (v < limit) { put("\"limit-"); putu((limit - v) / PAGE); put("p\""); }
    else { put("\"limit+"); putu((v - limit) / PAGE); put("p\""); }
}

static void query(const char *subject, const char *name, ULONG_PTR at, int with_base) {
    MEMORY_BASIC_INFORMATION m;
    SetLastError(SENTINEL);
    SIZE_T r = VirtualQuery((void *)at, &m, sizeof m);
    DWORD err = GetLastError();
    row(subject, name); field("ret_is_size", r == sizeof m); field("err", err);
    if (with_base) where("base_address", (ULONG_PTR)m.BaseAddress);
    where("allocation_base", (ULONG_PTR)m.AllocationBase); where("end", (ULONG_PTR)m.BaseAddress + m.RegionSize);
    put(",\"allocation_protect\":"); putu(m.AllocationProtect); field("state", m.State); field("protect", m.Protect);
    field("type", m.Type);
    end();
}

static ULONG_PTR teb(void) { ULONG_PTR t; __asm__ volatile("movq %%gs:0x30, %0" : "=r"(t)); return t; }
static ULONG_PTR teb_field(unsigned off) { return *(volatile ULONG_PTR *)(teb() + off); }

struct subject {
    const char *name;
    ULONG_PTR base, limit, dealloc, sp;
    ULONG_PTR low, high;
    DWORD limits_err;
    int grow;
    ULONG_PTR before, grown[GROW];
};

static HANDLE ready, go;

static DWORD WINAPI subject_main(LPVOID param) {
    struct subject *s = (struct subject *)param;
    s->base = teb_field(0x08);
    s->limit = teb_field(0x10);
    s->dealloc = teb_field(0x1478);
    s->sp = (ULONG_PTR)&s;
    SetLastError(SENTINEL);
    GetCurrentThreadStackLimits(&s->low, &s->high);
    s->limits_err = GetLastError();
    SetEvent(ready);
    WaitForSingleObject(go, INFINITE);
    if (s->grow) {
        /* no call between reading StackLimit and the last touch: only the touches move it */
        ULONG_PTR from = teb_field(0x10);
        s->before = from;
        for (int i = 1; i <= GROW; i++) {
            *(volatile char *)(from - (ULONG_PTR)i * PAGE) = 1;
            s->grown[i - 1] = teb_field(0x10);
        }
        SetEvent(ready);
        WaitForSingleObject(go, INFINITE);
    }
    return 0;
}

/* "<prefix>.<at>" in name */
static const char *named(char *name, const char *prefix, const char *at) {
    unsigned j = 0;
    for (const char *p = prefix; *p; p++) name[j++] = *p;
    name[j++] = '.';
    for (const char *p = at; *p; p++) name[j++] = *p;
    name[j] = 0;
    return name;
}

static void queries(const char *subject, const char *prefix, ULONG_PTR sp) {
    char name[32];
    static const char *const below[] = {"limit-1", "limit-2p", "limit-3p", "limit-4p", "limit-5p"};
    query(subject, named(name, prefix, "sp"), sp, 0);
    query(subject, named(name, prefix, "base-1"), base - 1, 0);
    query(subject, named(name, prefix, "limit"), limit, 1);
    for (unsigned k = 0; k < 5; k++)
        query(subject, named(name, prefix, below[k]), k == 0 ? limit - 1 : limit - (ULONG_PTR)(k + 1) * PAGE, 1);
    query(subject, named(name, prefix, "dealloc"), dealloc, 1);
}

static struct subject subjects[3];

static void run_subject(int i, const char *name, SIZE_T size, DWORD flags, int grow) {
    struct subject *s = &subjects[i];
    s->name = name;
    s->grow = grow;
    HANDLE h = CreateThread(NULL, size, subject_main, s, flags, NULL);
    WaitForSingleObject(ready, INFINITE);
    base = s->base; limit = s->limit; dealloc = s->dealloc;
    IMAGE_NT_HEADERS64 *nt = (IMAGE_NT_HEADERS64 *)((BYTE *)&__ImageBase + __ImageBase.e_lfanew);
    row(name, "teb"); field("reserve_pages", (base - dealloc) / PAGE);
    field("header_reserve_pages", nt->OptionalHeader.SizeOfStackReserve / PAGE);
    field("header_commit_pages", nt->OptionalHeader.SizeOfStackCommit / PAGE);
    field("committed_pages", (base - limit) / PAGE); field("sp_inside", limit <= s->sp && s->sp < base);
    field("aligned", (base | limit | dealloc) % PAGE == 0); field("limit_above_dealloc", limit > dealloc); end();
    row(name, "limits"); where("low", s->low); where("high", s->high); field("err", s->limits_err); end();
    queries(name, "query", s->sp);
    if (grow) {
        SetEvent(go);
        WaitForSingleObject(ready, INFINITE);
        row(name, "grow"); field("limit_unchanged", s->before == s->limit);
        static const char *const moved[GROW] = {"moved_1", "moved_2", "moved_3", "moved_4", "moved_5", "moved_6",
                                                "moved_7", "moved_8"};
        for (int k = 0; k < GROW; k++) field(moved[k], (s->before - s->grown[k]) / PAGE);
        end();
        limit = s->grown[GROW - 1];
        queries(name, "grown", s->sp);
    }
    SetEvent(go);
    WaitForSingleObject(h, INFINITE);
    CloseHandle(h);
}

/* skip: one touch 16 pages below StackLimit; the handler records it and sends the thread to skip_landed */
static volatile DWORD skip_tid, skip_code, skip_write, skip_at_touched;
static volatile ULONG_PTR skip_limit, skip_touched, skip_limit_after;

static void skip_landed(void) {
    skip_limit_after = teb_field(0x10);
    ExitThread(9);
}

static LONG WINAPI skip_handler(EXCEPTION_POINTERS *e) {
    if (GetCurrentThreadId() != skip_tid || skip_code) return EXCEPTION_CONTINUE_SEARCH;
    skip_code = e->ExceptionRecord->ExceptionCode;
    skip_write = (DWORD)e->ExceptionRecord->ExceptionInformation[0];
    skip_at_touched = e->ExceptionRecord->ExceptionInformation[1] == skip_touched;
    e->ContextRecord->Rip = (DWORD64)(ULONG_PTR)skip_landed;
    e->ContextRecord->Rsp = (e->ContextRecord->Rsp & ~(DWORD64)15) - 8;     /* as at a function's entry */
    return EXCEPTION_CONTINUE_EXECUTION;
}

static DWORD WINAPI skipper(LPVOID param) {
    (void)param;
    skip_tid = GetCurrentThreadId();
    skip_limit = teb_field(0x10);
    skip_touched = skip_limit - SKIP * PAGE;
    *(volatile char *)skip_touched = 1;
    skip_limit_after = teb_field(0x10);
    return 0;
}

void start(void) {
    ready = CreateEventW(NULL, FALSE, FALSE, NULL);
    go = CreateEventW(NULL, FALSE, FALSE, NULL);
    run_subject(0, "default", 0, 0, 1);
    run_subject(1, "reserve", 0x40000, STACK_SIZE_PARAM_IS_A_RESERVATION, 0);
    run_subject(2, "commit", 0x20000, 0, 0);

    PVOID veh = AddVectoredExceptionHandler(1, skip_handler);
    HANDLE h = CreateThread(NULL, 0, skipper, NULL, 0, NULL);
    WaitForSingleObject(h, INFINITE);
    DWORD code = 0;
    GetExitCodeThread(h, &code);
    CloseHandle(h);
    RemoveVectoredExceptionHandler(veh);
    row("skip", ""); field("code", skip_code); field("write", skip_write); field("at_touched", skip_at_touched);
    field("moved", (skip_limit - skip_limit_after) / PAGE); field("exit_code", code); end();
    ExitProcess(0);
}
