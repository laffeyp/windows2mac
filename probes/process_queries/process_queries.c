/* process_queries.c — the C runtime's first questions to Windows (Sprint 044, family "process queries").
 *
 * No C runtime: entry `start`, kernel32 imports only. The launcher's start-up stops first at
 * GetSystemTimeAsFileTime (frontier 2026-10-10); this family is the stateless queries around it:
 * GetSystemTimeAsFileTime, GetCurrentProcessId, GetCurrentThreadId, QueryPerformanceCounter,
 * QueryPerformanceFrequency, IsProcessorFeaturePresent, InitializeSListHead, GetStartupInfoW,
 * IsDebuggerPresent.
 *
 * Values that differ between machines and runs (the time, the ids, the counter) are printed as
 * the relations that must hold on any machine; values Windows fixes are printed as they are.
 * For IsProcessorFeaturePresent the probe prints the answer and, where a CPUID bit names the same
 * feature (Intel SDM Vol 2A CPUID), that bit as this CPU reports it, read here with CPUID; the
 * contract's rows are derived from both (scripts/family_run.py, family process_queries).
 */
#include <windows.h>

static char out[16384];
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

static void puti(long long v) {
    if (v < 0) {
        out[n++] = '-';
        putu((unsigned long long)-v);
    } else {
        putu((unsigned long long)v);
    }
}

static void row(const char *s) {
    put("{\"s\":\"");
    put(s);
    put("\"");
}

static void field(const char *k, long long v) {
    put(",\"");
    put(k);
    put("\":");
    puti(v);
}

static void flag(const char *k, int v) {
    put(",\"");
    put(k);
    put(v ? "\":true" : "\":false");
}

static void end(void) {
    put("}\n");
}

static void cpuid(unsigned leaf, unsigned sub, unsigned r[4]) {
    __asm__ volatile("cpuid" : "=a"(r[0]), "=b"(r[1]), "=c"(r[2]), "=d"(r[3]) : "a"(leaf), "c"(sub));
}

static unsigned long long gs(unsigned off) {
    unsigned long long v;
    __asm__ volatile("movq %%gs:(%1), %0" : "=r"(v) : "r"((unsigned long long)off));
    return v;
}

static unsigned long long filetime(void) {
    FILETIME f;
    GetSystemTimeAsFileTime(&f);
    return (unsigned long long)f.dwHighDateTime << 32 | f.dwLowDateTime;
}

static unsigned long long counter(void) {
    LARGE_INTEGER c;
    QueryPerformanceCounter(&c);
    return (unsigned long long)c.QuadPart;
}

/* PF_* value -> (CPUID leaf, register 0-3 = EAX EBX ECX EDX, bit), or leaf 0xFFFFFFFF when no
 * CPUID bit names the feature. Leaves and bits: Intel SDM Vol 2A, CPUID, tables 3-10, 3-11, 3-8. */
struct feature { unsigned pf, leaf, reg, bit; };
static const struct feature FEATURES[] = {
    {0, 0xFFFFFFFFu, 0, 0},  {1, 0xFFFFFFFFu, 0, 0},  {2, 1, 3, 8},    {3, 1, 3, 23},  {6, 1, 3, 25},
    {7, 0xFFFFFFFFu, 0, 0},  {8, 1, 3, 4},            {9, 0xFFFFFFFFu, 0, 0},          {10, 1, 3, 26},
    {12, 0xFFFFFFFFu, 0, 0}, {13, 1, 2, 0},           {14, 1, 2, 13},  {15, 0xFFFFFFFFu, 0, 0},
    {16, 0xFFFFFFFFu, 0, 0}, {17, 1, 2, 26},          {20, 0xFFFFFFFFu, 0, 0},         {21, 0xFFFFFFFFu, 0, 0},
    {22, 7, 1, 0},           {23, 0xFFFFFFFFu, 0, 0}, {36, 1, 2, 9},   {37, 1, 2, 19}, {38, 1, 2, 20},
    {39, 1, 2, 28},          {40, 7, 1, 5},           {41, 7, 1, 16},  {60, 7, 1, 8},  {61, 7, 2, 28},
    {89, 7, 2, 5},
};

static void processor_features(void) {
    for (unsigned i = 0; i < sizeof FEATURES / sizeof FEATURES[0]; i++) {
        const struct feature *f = &FEATURES[i];
        static char name[16];
        unsigned k = 0, v = f->pf;
        const char *p = "ipfp.";
        while (*p) name[k++] = *p++;
        if (v >= 10) name[k++] = (char)('0' + v / 10);
        name[k++] = (char)('0' + v % 10);
        name[k] = 0;
        row(name);
        field("value", IsProcessorFeaturePresent(f->pf) != 0);
        if (f->leaf != 0xFFFFFFFFu) {
            unsigned r[4];
            cpuid(f->leaf, 0, r);
            field("cpuid", r[f->reg] >> f->bit & 1);
        }
        end();
    }
}

static void times(void) {
    /* 2026-01-01T00:00:00Z and 2100-01-01T00:00:00Z in 100 ns units since 1601-01-01. */
    const unsigned long long y2026 = 134116992000000000ull, y2100 = 157469184000000000ull;
    unsigned long long t0 = filetime(), last = t0, now = t0;
    int ordered = 1;
    unsigned long long spins = 0;
    while (now == t0 && spins < 100000000ull) {
        now = filetime();
        ordered &= now >= last;
        last = now;
        spins++;
    }
    row("time.filetime");
    flag("after_2026", t0 >= y2026);
    flag("before_2100", t0 < y2100);
    flag("nondecreasing", ordered);
    flag("advances", now > t0);
    end();

    LARGE_INTEGER f;
    BOOL okf = QueryPerformanceFrequency(&f);
    row("time.qpf");
    field("ret", okf != 0);
    field("frequency", f.QuadPart);
    end();

    LARGE_INTEGER c;
    BOOL okc = QueryPerformanceCounter(&c);
    unsigned long long c0 = (unsigned long long)c.QuadPart, cl = c0, cn = c0;
    ordered = 1;
    for (int i = 0; i < 100000; i++) {
        cn = counter();
        ordered &= cn >= cl;
        cl = cn;
    }
    row("time.qpc");
    field("ret", okc != 0);
    flag("nondecreasing", ordered);
    flag("advances", cn > c0);
    end();

    /* Over one advance of the system time, the counter's elapsed seconds agree within 25%. The window
       runs twice and the second is kept: the first execution of new code may stall (a translator
       compiles it), and a stall between the counter's read and the clock's would sit in one window
       only (w2m failed 4 of 30 runs on the first window alone, with a monotonic counter). */
    unsigned long long counted = 0, timed = 0;
    for (int pass = 0; pass < 2; pass++) {
        unsigned long long a = filetime(), b = a;
        while (b - a < 200000ull) b = filetime();         /* at least 20 ms of system time */
        unsigned long long ca = counter();
        unsigned long long a2 = filetime(), b2 = a2;
        while (b2 - a2 < 500000ull) b2 = filetime();      /* 50 ms */
        unsigned long long cb = counter();
        counted = (cb - ca) * 10000000ull / (unsigned long long)f.QuadPart;  /* 100 ns units */
        timed = b2 - a2;
    }
    row("time.counter_tracks_time");
    flag("within_25_percent", counted * 4 >= timed * 3 && counted * 4 <= timed * 5);
    end();
}

static void identity(void) {
    DWORD pid = GetCurrentProcessId(), tid = GetCurrentThreadId();
    row("ids.process");
    flag("nonzero", pid != 0);
    flag("multiple_of_4", pid % 4 == 0);
    flag("equals_teb", pid == (DWORD)gs(0x40));           /* TEB.ClientId.UniqueProcess */
    end();
    row("ids.thread");
    flag("nonzero", tid != 0);
    flag("multiple_of_4", tid % 4 == 0);
    flag("equals_teb", tid == (DWORD)gs(0x48));           /* TEB.ClientId.UniqueThread */
    flag("differs_from_process", tid != pid);
    end();
}

static void misc(void) {
    static __declspec(align(16)) unsigned char slist[16];
    for (int i = 0; i < 16; i++) slist[i] = 0xA5;
    InitializeSListHead((PSLIST_HEADER)slist);
    int zero = 1;
    for (int i = 0; i < 16; i++) zero &= slist[i] == 0;
    row("slist.initialize");
    flag("all_zero", zero);
    end();

    static STARTUPINFOW si;
    unsigned char *b = (unsigned char *)&si;
    for (unsigned i = 0; i < sizeof si; i++) b[i] = 0xA5;
    GetStartupInfoW(&si);
    row("startup.info");
    field("cb", si.cb);
    flag("reserved_null", si.lpReserved == 0);
    flag("reserved2_null", si.lpReserved2 == 0);
    field("cb_reserved2", si.cbReserved2);
    end();

    row("debugger.present");
    field("value", IsDebuggerPresent() != 0);
    end();
}

void start(void) {
    identity();
    times();
    processor_features();
    misc();
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
