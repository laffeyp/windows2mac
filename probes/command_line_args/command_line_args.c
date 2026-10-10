// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* command_line_args.c — SHELL32's CommandLineToArgvW (Sprint 047, family "command line arguments").
 *
 * No C runtime. UnityPlayer.dll's UnityMain calls CommandLineToArgvW at 0x180527460 (frontier 2026-10-10,
 * 403,044 instructions). Microsoft Learn documents two rule sets: the program name (argv[0]) ends at the
 * first space or tab outside quotes and takes backslashes literally; later arguments follow the C runtime's
 * rules (2n backslashes before a quote give n and the quote toggles; 2n+1 give n and a literal quote; a
 * quote doubled inside quotes is literal). Rows: each input's count and arguments (JSON-escaped), the
 * result's layout (one LocalAlloc block: LocalFree returns NULL; the strings follow the pointer array),
 * the empty command line (argv[0] is the executable's path), and a NULL count pointer.
 */
#include <windows.h>
#include <shellapi.h>

#define SENTINEL 0xDEADu

static char out[32768];
static unsigned n;

static void put(const char *s) { while (*s) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void putw(const WCHAR *s) {               /* a JSON string; the inputs are ASCII, so is every argument */
    out[n++] = '"';
    for (; *s; s++) {
        WCHAR c = *s;
        if (c == '"' || c == '\\') { out[n++] = '\\'; out[n++] = (char)c; }
        else if (c == '\t') { out[n++] = '\\'; out[n++] = 't'; }
        else if (c >= 0x20 && c < 0x7F) out[n++] = (char)c;
        else { put("\\u00"); out[n++] = "0123456789abcdef"[c >> 4 & 15]; out[n++] = "0123456789abcdef"[c & 15]; }
    }
    out[n++] = '"';
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void end(void) { put("}\n"); }

static const WCHAR *const INPUTS[] = {
    L"prog a b c",
    L"prog \"a b\" c",
    L"prog a\\b",
    L"prog a\\\"b",
    L"prog a\\\\\\\"b",
    L"prog a\\\\\\\\\"b c\"",
    L"prog \"a\"\"b\"",
    L"prog \"unterminated",
    L"\"C:\\Program Files\\x.exe\" arg",
    L"C:\\a\\\"b c\" d",
    L"prog   ",
    L"  prog a",
    L"prog\ta\tb",
    L"prog \"\"",
    L"prog a\"\"b",
    L"prog \"\"\"a\"\"\"",
    L"prog \\\\server\\share",
    L"prog \"a\\\"b\"",
    L"prog \"\\\\\"",
    L"prog x \"y\"z",
    L"prog a\\\\\\\\",
    L"\"prog\"x y",
    L"prog \"a b\"c\" d",
    L"prog a \"\" b",
};

void start(void) {
    for (unsigned i = 0; i < sizeof INPUTS / sizeof INPUTS[0]; i++) {
        int argc = -1;
        SetLastError(SENTINEL);
        LPWSTR *argv = CommandLineToArgvW(INPUTS[i], &argc);
        DWORD err = GetLastError();
        static char name[16] = "input.";
        name[6] = (char)('0' + i / 10); name[7] = (char)('0' + i % 10); name[8] = 0;
        row(name); put(",\"in\":"); putw(INPUTS[i]); field("non_null", argv != 0); field("err", err);
        if (argv) {
            field("argc", (unsigned long long)argc);
            put(",\"args\":[");
            for (int k = 0; k < argc; k++) { if (k) put(","); putw(argv[k]); }
            put("]");
            int after = 1;                                 /* every string lies after the pointer array */
            for (int k = 0; k < argc; k++) after &= (char *)argv[k] >= (char *)(argv + argc);
            flag("strings_after_array", after);
            flag("null_terminated_array", argv[argc] == 0);
            flag("local_free_null", LocalFree(argv) == 0);
        }
        end();
    }
    /* the empty command line: argv[0] is the executable's path (Microsoft Learn) */
    int argc = -1;
    SetLastError(SENTINEL);
    LPWSTR *argv = CommandLineToArgvW(L"", &argc);
    DWORD err = GetLastError();
    row("empty"); field("non_null", argv != 0); field("err", err);
    if (argv) {
        WCHAR path[MAX_PATH];
        DWORD len = GetModuleFileNameW(NULL, path, MAX_PATH);
        int same = (int)len > 0;
        for (DWORD k = 0; same && k <= len; k++) same = argv[0][k] == path[k];
        field("argc", (unsigned long long)argc); flag("arg0_is_module_path", same);
        LocalFree(argv);
    }
    end();
    /* a NULL count pointer */
    SetLastError(SENTINEL);
    argv = CommandLineToArgvW(L"prog a", NULL);
    err = GetLastError();
    row("null_count"); field("non_null", argv != 0); field("err", err); end();
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
