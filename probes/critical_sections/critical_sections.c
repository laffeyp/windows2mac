/* critical_sections.c — CRITICAL_SECTION as Windows leaves it (Sprint 044, family "critical sections").
 *
 * No C runtime. The launcher's start-up reaches InitializeCriticalSectionEx through GetProcAddress
 * on api-ms-win-core-synch-l1-2-0 (frontier 2026-10-10, 260 instructions). This family is
 * InitializeCriticalSectionEx, InitializeCriticalSectionAndSpinCount, InitializeCriticalSection,
 * EnterCriticalSection, TryEnterCriticalSection, LeaveCriticalSection and DeleteCriticalSection, on
 * one thread. Each row prints the structure's fields (winnt.h RTL_CRITICAL_SECTION: DebugInfo,
 * LockCount, RecursionCount, OwningThread, LockSemaphore, SpinCount) after a call, the owner as a
 * relation to this thread's id, and the return value and last error where the function has them.
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

static void puti(long long v) {
    if (v < 0) { out[n++] = '-'; putu((unsigned long long)-v); } else putu((unsigned long long)v);
}

static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, long long v) { put(",\""); put(k); put("\":"); puti(v); }
static void hexfield(const char *k, unsigned long long v) {
    static const char d[] = "0123456789abcdef";
    put(",\""); put(k); put("\":\"0x");
    int started = 0;
    for (int i = 60; i >= 0; i -= 4) {
        unsigned x = (unsigned)(v >> i) & 15;
        if (x || started || i == 0) { out[n++] = d[x]; started = 1; }
    }
    put("\"");
}
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void end(void) { put("}\n"); }

static void state(const char *s, CRITICAL_SECTION *cs) {
    row(s);
    hexfield("debug_info", (unsigned long long)cs->DebugInfo);
    field("lock_count", cs->LockCount);
    field("recursion_count", cs->RecursionCount);
    flag("owner_is_this_thread", (DWORD)(ULONG_PTR)cs->OwningThread == GetCurrentThreadId());
    flag("owner_null", cs->OwningThread == 0);
    hexfield("lock_semaphore", (unsigned long long)cs->LockSemaphore);
    hexfield("spin_count", (unsigned long long)cs->SpinCount);
    end();
}

static CRITICAL_SECTION cs;

static void fill(void) {
    unsigned char *b = (unsigned char *)&cs;
    for (unsigned i = 0; i < sizeof cs; i++) b[i] = 0xA5;
}

void start(void) {
    row("size"); field("bytes", sizeof(CRITICAL_SECTION)); end();

    fill();
    SetLastError(SENTINEL);
    BOOL ok = InitializeCriticalSectionEx(&cs, 4000, 0);
    DWORD err = GetLastError();
    row("init_ex.ret"); field("ret", ok != 0); field("err", err); end();
    state("init_ex.4000.0", &cs);
    EnterCriticalSection(&cs);
    state("enter.1", &cs);
    EnterCriticalSection(&cs);
    state("enter.2", &cs);
    SetLastError(SENTINEL);
    BOOL t = TryEnterCriticalSection(&cs);
    err = GetLastError();
    row("try_enter.owned.ret"); field("ret", t != 0); field("err", err); end();
    state("try_enter.owned", &cs);
    LeaveCriticalSection(&cs);
    state("leave.1", &cs);
    LeaveCriticalSection(&cs);
    state("leave.2", &cs);
    LeaveCriticalSection(&cs);
    state("leave.3", &cs);
    SetLastError(SENTINEL);
    t = TryEnterCriticalSection(&cs);
    err = GetLastError();
    row("try_enter.free.ret"); field("ret", t != 0); field("err", err); end();
    state("try_enter.free", &cs);
    LeaveCriticalSection(&cs);
    DeleteCriticalSection(&cs);
    state("delete", &cs);

    fill();
    InitializeCriticalSectionEx(&cs, 0, 0);
    state("init_ex.0.0", &cs);
    DeleteCriticalSection(&cs);

    fill();
    InitializeCriticalSectionEx(&cs, 4000, CRITICAL_SECTION_NO_DEBUG_INFO);
    state("init_ex.4000.no_debug_info", &cs);
    DeleteCriticalSection(&cs);

    fill();
    SetLastError(SENTINEL);
    ok = InitializeCriticalSectionAndSpinCount(&cs, 1500);
    err = GetLastError();
    row("init_spin.ret"); field("ret", ok != 0); field("err", err); end();
    state("init_spin.1500", &cs);
    DeleteCriticalSection(&cs);

    fill();
    InitializeCriticalSection(&cs);
    state("init", &cs);
    DeleteCriticalSection(&cs);

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
