/* storage_get2.c — FlsGetValue2 and TlsGetValue2 (Sprint 046, family "storage get2").
 *
 * No C runtime. The static C runtime of the runners' MSVC toolset looks up FlsGetValue2 with
 * GetProcAddress (w2m stopped on it running the static-TLS probe's DLL, 2026-10-10). Microsoft Learn
 * documents TlsGetValue2 (Windows 11 24H2) as TlsGetValue without the last error; FlsGetValue2's page is
 * not published. Rows: whether each image's kernel32 and kernelbase export the two, and, where they do,
 * each one's value and last error (after SetLastError(0xDEAD)) for a set slot, an unset slot and a
 * freed one, beside FlsGetValue's and TlsGetValue's.
 */
#include <windows.h>

#define SENTINEL 0xDEADu

typedef PVOID (WINAPI *get2_fn)(DWORD);

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
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void end(void) { put("}\n"); }

/* get1 is FlsGetValue (fls) or TlsGetValue, called directly: an import reached only through a function
 * pointer is invisible to the reference traversal (tests/test_x86ref.py) */
static void probe(const char *name, get2_fn get2, int fls, DWORD index) {
    SetLastError(SENTINEL);
    PVOID v2 = get2(index);
    DWORD e2 = GetLastError();
    SetLastError(SENTINEL);
    PVOID v1 = fls ? FlsGetValue(index) : TlsGetValue(index);
    DWORD e1 = GetLastError();
    row(name); field("value2", (unsigned long long)v2); field("err2", e2); field("value1", (unsigned long long)v1);
    field("err1", e1); end();
}

void start(void) {
    HMODULE k32 = GetModuleHandleW(L"kernel32.dll"), kb = GetModuleHandleW(L"kernelbase.dll");
    get2_fn fls2 = (get2_fn)GetProcAddress(kb, "FlsGetValue2"), tls2 = (get2_fn)GetProcAddress(kb, "TlsGetValue2");
    row("exports"); flag("kernel32_fls2", GetProcAddress(k32, "FlsGetValue2") != 0); flag("kernelbase_fls2", fls2 != 0);
    flag("kernel32_tls2", GetProcAddress(k32, "TlsGetValue2") != 0); flag("kernelbase_tls2", tls2 != 0); end();
    if (fls2) {
        DWORD i = FlsAlloc(NULL);
        probe("fls2.unset", fls2, 1, i);
        FlsSetValue(i, (PVOID)42);
        probe("fls2.set", fls2, 1, i);
        FlsFree(i);
        probe("fls2.freed", fls2, 1, i);
    }
    if (tls2) {
        DWORD i = TlsAlloc();
        probe("tls2.unset", tls2, 0, i);
        TlsSetValue(i, (PVOID)43);
        probe("tls2.set", tls2, 0, i);
        TlsFree(i);
        probe("tls2.freed", tls2, 0, i);
    }
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
