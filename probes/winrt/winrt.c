// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* winrt.c — RoInitialize as UnityPlayer reaches it (Sprint 055, family "winrt"; specs/work_list.json).
 *
 * No C runtime. Game run 38176899195 (seq 72-75): LoadLibraryExW of api-ms-win-core-winrt-l1-1-0.dll,
 * api-ms-win-core-com-l1-1-1.dll and api-ms-win-core-winrt-string-l1-1-0.dll, then GetProcAddress of RoInitialize,
 * which combase.dll answers, and RoInitialize(RO_INIT_SINGLETHREADED) answering S_OK on the main thread. The rows:
 * the three API sets, the function, RoInitialize in each model, again, against CoInitializeEx, and RoUninitialize.
 */
#include <windows.h>

typedef HRESULT (WINAPI *ro_init_fn)(int);
typedef void (WINAPI *ro_uninit_fn)(void);

static char out[4096];
static unsigned n;
static void put(const char *s) { while (*s && n < sizeof out - 64) out[n++] = *s++; }
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

void start(void) {
    HMODULE a = LoadLibraryExW(L"api-ms-win-core-winrt-l1-1-0.dll", NULL, 0);
    HMODULE b = LoadLibraryExW(L"api-ms-win-core-com-l1-1-1.dll", NULL, 0);
    HMODULE c = LoadLibraryExW(L"api-ms-win-core-winrt-string-l1-1-0.dll", NULL, 0);
    HMODULE combase = GetModuleHandleW(L"combase.dll");
    row("load.api_sets"); flag("winrt", a != NULL); flag("com", b != NULL); flag("winrt_string", c != NULL);
    flag("winrt_is_combase", a == combase); flag("com_is_combase", b == combase); flag("string_is_combase", c == combase); end();
    ro_init_fn init = a ? (ro_init_fn)GetProcAddress(a, "RoInitialize") : NULL;
    ro_uninit_fn uninit = a ? (ro_uninit_fn)GetProcAddress(a, "RoUninitialize") : NULL;
    row("proc"); flag("init", init != NULL); flag("uninit", uninit != NULL);
    flag("init_in_combase", init && combase && init == (ro_init_fn)GetProcAddress(combase, "RoInitialize")); end();
    if (init && uninit) {
        row("init.single"); field("hr", (DWORD)init(0)); end();
        row("init.single_again"); field("hr", (DWORD)init(0)); end();
        row("init.multi_after_single"); field("hr", (DWORD)init(1)); end();
        uninit();
        uninit();
        row("init.multi"); field("hr", (DWORD)init(1)); end();
        row("init.bad_type"); field("hr", (DWORD)init(5)); end();
        uninit();
    }
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
