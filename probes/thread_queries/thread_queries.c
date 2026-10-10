/* thread_queries.c — GetCurrentThread, GetThreadTimes, GetThreadPriority (Sprint 046, family "thread queries").
 *
 * No C runtime. UnityPlayer.dll's DllMain calls GetCurrentThread and GetThreadTimes (scratch run of the
 * launcher, 2026-10-10). Times differ between runs, so they are printed as relations: creation within a
 * second after now or ten minutes before it, exit time zero, kernel and user time each under ten seconds.
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
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void end(void) { put("}\n"); }
static void finish(void) {
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}

static unsigned long long ft(FILETIME f) { return (unsigned long long)f.dwHighDateTime << 32 | f.dwLowDateTime; }

void start(void) {
    HANDLE t = GetCurrentThread();
    row("current_thread"); field("value", (unsigned long long)t); end();

    FILETIME c, e, k, u, now;
    c.dwLowDateTime = c.dwHighDateTime = 0xA5A5A5A5;
    e.dwLowDateTime = e.dwHighDateTime = 0xA5A5A5A5;
    SetLastError(SENTINEL);
    BOOL ok = GetThreadTimes(t, &c, &e, &k, &u);
    DWORD err = GetLastError();
    GetSystemTimeAsFileTime(&now);
    const unsigned long long ten_minutes = 6000000000ull, ten_seconds = 100000000ull, one_second = 10000000ull;
    row("times.current"); field("ret", ok != 0); field("err", err);
    /* the creation time may read later than GetSystemTimeAsFileTime just after it (Server 2022, 6 of 8
       executions, 2026-10-10): the relation is a window around now, not an order */
    flag("created_near_now", ft(c) <= ft(now) + one_second && (ft(c) > ft(now) || ft(now) - ft(c) < ten_minutes));
    flag("exit_zero", ft(e) == 0); flag("kernel_under_ten_seconds", ft(k) < ten_seconds);
    flag("user_under_ten_seconds", ft(u) < ten_seconds); end();

    SetLastError(SENTINEL);
    ok = GetThreadTimes(NULL, &c, &e, &k, &u);
    err = GetLastError();
    row("times.null_handle"); field("ret", ok != 0); field("err", err); end();

    SetLastError(SENTINEL);
    int priority = GetThreadPriority(t);
    err = GetLastError();
    row("priority.current"); field("value", (unsigned long long)(long long)priority); field("err", err); end();
    finish();
}
