/* modules.c — how Windows answers a program about modules (Sprint 044, family "modules").
 *
 * No C runtime: entry `start`, kernel32 imports only. The launcher's start-up calls
 * LoadLibraryExW(L"api-ms-win-core-synch-l1-2-0", NULL, LOAD_LIBRARY_SEARCH_SYSTEM32) after its
 * ISA probe (frontier 2026-10-10, 209 instructions); this family is LoadLibraryExW, GetProcAddress,
 * GetModuleHandleW, GetModuleHandleExW, GetModuleFileNameW and FreeLibrary.
 *
 * Handles and addresses differ between runs, so rows print the relations every run must show
 * (equal to another call's answer, nonzero, the image base) and the error codes, each read after
 * SetLastError(0xDEAD).
 */
#include <windows.h>

#define SENTINEL 0xDEADu
#define SEARCH_SYSTEM32 0x800u

extern IMAGE_DOS_HEADER __ImageBase;

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

static unsigned long long gs(unsigned off) {
    unsigned long long v;
    __asm__ volatile("movq %%gs:(%1), %0" : "=r"(v) : "r"((unsigned long long)off));
    return v;
}

static DWORD err;
static HMODULE load(const WCHAR *name, DWORD flags) {
    SetLastError(SENTINEL);
    HMODULE h = LoadLibraryExW(name, NULL, flags);
    err = GetLastError();
    return h;
}

static HMODULE handle(const WCHAR *name) {
    SetLastError(SENTINEL);
    HMODULE h = GetModuleHandleW(name);
    err = GetLastError();
    return h;
}

static FARPROC proc(HMODULE m, const char *name) {
    SetLastError(SENTINEL);
    FARPROC p = GetProcAddress(m, name);
    err = GetLastError();
    return p;
}

static int ends_with_ci(const WCHAR *s, DWORD len, const WCHAR *tail) {
    DWORD t = 0;
    while (tail[t]) t++;
    if (len < t) return 0;
    for (DWORD i = 0; i < t; i++) {
        WCHAR a = s[len - t + i], b = tail[i];
        if (a >= 'A' && a <= 'Z') a = (WCHAR)(a + 32);
        if (b >= 'A' && b <= 'Z') b = (WCHAR)(b + 32);
        if (a != b) return 0;
    }
    return 1;
}

void start(void) {
    HMODULE image = (HMODULE)&__ImageBase;
    HMODULE k32 = handle(L"kernel32.dll");
    row("handle.kernel32"); flag("nonzero", k32 != 0); field("err", err); end();
    HMODULE kb = handle(L"kernelbase.dll");
    row("handle.kernelbase"); flag("nonzero", kb != 0); flag("differs_from_kernel32", kb != k32); end();
    HMODULE up = handle(L"KERNEL32.DLL");
    row("handle.kernel32_upper_case"); flag("equals_kernel32", up == k32); end();
    HMODULE self = handle(NULL);
    row("handle.null"); flag("equals_image_base", self == image);
    flag("equals_peb_image_base", (unsigned long long)self == *(unsigned long long *)(gs(0x60) + 0x10)); end();
    HMODULE none = handle(L"w2m-no-such-module.dll");
    row("handle.missing"); flag("null", none == 0); field("err", err); end();

    HMODULE apiset = load(L"api-ms-win-core-synch-l1-2-0", SEARCH_SYSTEM32);
    row("load.apiset_synch"); flag("nonzero", apiset != 0); field("err", err);
    flag("equals_kernelbase", apiset == kb); flag("equals_kernel32", apiset == k32); end();
    HMODULE lk = load(L"kernel32.dll", SEARCH_SYSTEM32);
    row("load.kernel32"); flag("equals_kernel32", lk == k32); field("err", err); end();
    HMODULE lk0 = load(L"kernel32", 0);
    row("load.kernel32_no_extension"); flag("equals_kernel32", lk0 == k32); end();
    HMODULE miss = load(L"w2m-no-such-module.dll", SEARCH_SYSTEM32);
    row("load.missing"); flag("null", miss == 0); field("err", err); end();
    HMODULE miss0 = load(L"w2m-no-such-module.dll", 0);
    row("load.missing_default_search"); flag("null", miss0 == 0); field("err", err); end();

    FARPROC p = proc(k32, "GetCurrentProcessId");
    row("proc.kernel32_found"); flag("nonzero", p != 0); flag("equals_import", p == (FARPROC)GetCurrentProcessId);
    field("err", err); end();
    FARPROC pe = proc(apiset, "InitializeCriticalSectionEx");
    row("proc.apiset_found"); flag("nonzero", pe != 0); field("err", err); end();
    FARPROC pk = proc(k32, "InitializeCriticalSectionEx");
    row("proc.same_function_both_ways"); flag("equal", pk == pe); end();
    FARPROC pm = proc(k32, "W2mNoSuchFunction");
    row("proc.missing"); flag("null", pm == 0); field("err", err); end();

    HMODULE from = 0;
    SetLastError(SENTINEL);
    BOOL okx = GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                                  (LPCWSTR)(void *)start, &from);
    err = GetLastError();
    row("handle_ex.from_address"); field("ret", okx != 0); flag("equals_image_base", from == image); field("err", err); end();

    static WCHAR path[300];
    SetLastError(SENTINEL);
    DWORD len = GetModuleFileNameW(NULL, path, 300);
    err = GetLastError();
    DWORD real = 0;
    while (path[real]) real++;
    row("file_name.null"); flag("nonzero", len != 0); flag("length_matches", len == real);
    flag("ends_with_probe_exe", ends_with_ci(path, len, L"\\probe.exe")); flag("absolute", len > 2 && path[1] == ':');
    field("err", err); end();
    static WCHAR small[4];
    for (int i = 0; i < 4; i++) small[i] = 0x7777;
    SetLastError(SENTINEL);
    DWORD slen = GetModuleFileNameW(NULL, short_buffer, 4);
    err = GetLastError();
    row("file_name.small_buffer"); field("ret", slen); flag("terminated", small[3] == 0);
    flag("prefix_kept", small[0] == path[0] && small[1] == path[1] && small[2] == path[2]); field("err", err); end();

    SetLastError(SENTINEL);
    BOOL fr = FreeLibrary(lk);
    err = GetLastError();
    row("free.kernel32"); field("ret", fr != 0); field("err", err); end();

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
