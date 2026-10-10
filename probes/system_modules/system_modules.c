// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* system_modules.c — a statically imported system DLL through the module calls (Sprint 048, family "system modules").
 *
 * No C runtime. UnityPlayer.dll imports SHELL32.dll statically and later calls LoadLibraryW("shell32.dll")
 * (look-ahead 2026-10-10, 2,703,381 instructions). This probe imports SHELL32 the same way (CommandLineToArgvW)
 * and records: LoadLibraryW and GetModuleHandleW under several spellings, all compared with the first handle;
 * GetModuleFileNameW's text; GetProcAddress for the imported function (against the import's address), for another
 * export, and for a name SHELL32 does not export; FreeLibrary; each with its last error (after SetLastError(0xDEAD)).
 */
#include <windows.h>
#include <shellapi.h>

#define SENTINEL 0xDEADu

static char out[8192];
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
static void text(const char *k, const WCHAR *s) {
    put(",\""); put(k); put("\":\"");
    for (; *s; s++) { if (*s == '\\' || *s == '"') out[n++] = '\\'; out[n++] = *s < 128 ? (char)*s : '?'; }
    put("\"");
}
static void end(void) { put("}\n"); }

void start(void) {
    int argc = 0;
    LPWSTR *argv = CommandLineToArgvW(L"p", &argc);               /* the static import */
    LocalFree(argv);
    HMODULE first = GetModuleHandleW(L"shell32.dll");
    row("first"); field("non_null", first != NULL); end();
    static const struct { const char *name; const WCHAR *w; int load; } NAMES[] = {
        {"load.lower", L"shell32.dll", 1}, {"load.upper", L"SHELL32.DLL", 1}, {"load.no_extension", L"shell32", 1},
        {"load.full_path", L"C:\\Windows\\System32\\shell32.dll", 1}, {"handle.upper", L"SHELL32.dll", 0},
        {"handle.no_extension", L"shell32", 0}, {"handle.full_path", L"C:\\Windows\\System32\\shell32.dll", 0},
    };
    for (unsigned i = 0; i < sizeof NAMES / sizeof NAMES[0]; i++) {
        SetLastError(SENTINEL);
        HMODULE h = NAMES[i].load ? LoadLibraryW(NAMES[i].w) : GetModuleHandleW(NAMES[i].w);
        DWORD err = GetLastError();
        row(NAMES[i].name); field("same", h == first); field("err", err); end();
    }
    WCHAR path[MAX_PATH];
    SetLastError(SENTINEL);
    DWORD len = GetModuleFileNameW(first, path, MAX_PATH);
    DWORD err = GetLastError();
    row("file_name"); field("len_is_text", len > 0 && path[len] == 0); text("path", path); field("err", err); end();
    SetLastError(SENTINEL);
    FARPROC p = GetProcAddress(first, "CommandLineToArgvW");
    err = GetLastError();
    row("proc.imported"); field("is_import", p == (FARPROC)CommandLineToArgvW); field("non_null", p != NULL); field("err", err); end();
    SetLastError(SENTINEL);
    p = GetProcAddress(first, "SHGetFolderPathW");
    err = GetLastError();
    row("proc.other_export"); field("non_null", p != NULL); field("err", err); end();
    SetLastError(SENTINEL);
    p = GetProcAddress(first, "W2mNoSuchExport");
    err = GetLastError();
    row("proc.missing"); field("non_null", p != NULL); field("err", err); end();
    SetLastError(SENTINEL);
    BOOL ok = FreeLibrary(first);
    err = GetLastError();
    row("free"); field("ret", ok != 0); field("err", err); end();
    row("after_free"); field("still_loaded", GetModuleHandleW(L"shell32.dll") == first); end();
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
