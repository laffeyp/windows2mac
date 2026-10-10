/* exception_filter.c — SetUnhandledExceptionFilter's answers (Sprint 044, family "exception filter").
 *
 * No C runtime. The launcher's C runtime sets its unhandled-exception filter after its environment
 * (frontier 2026-10-10, 18,684 instructions), the last service before UnityPlayer.dll!UnityMain. Rows:
 * each call's return (NULL, or which earlier filter) and last error (after SetLastError(0xDEAD)), for
 * a first filter, a second, NULL, and the first again. Whether Windows calls the filter on an unhandled
 * exception is exception dispatch, not these rows.
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
static void end(void) { put("}\n"); }

static LONG WINAPI first(EXCEPTION_POINTERS *e) { (void)e; return EXCEPTION_CONTINUE_SEARCH; }
static LONG WINAPI second(EXCEPTION_POINTERS *e) { (void)e; return EXCEPTION_EXECUTE_HANDLER; }

/* 0 NULL, 1 first, 2 second, 3 anything else */
static unsigned which(LPTOP_LEVEL_EXCEPTION_FILTER f) { return !f ? 0 : f == first ? 1 : f == second ? 2 : 3; }

static void call(const char *name, LPTOP_LEVEL_EXCEPTION_FILTER f) {
    SetLastError(SENTINEL);
    LPTOP_LEVEL_EXCEPTION_FILTER before = SetUnhandledExceptionFilter(f);
    DWORD err = GetLastError();
    row(name); field("returned", which(before)); field("err", err); end();
}

void start(void) {
    call("set.first", first);
    call("set.second", second);
    call("set.null", NULL);
    call("set.first_again", first);
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
