// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* module_names_ansi.c — the ANSI module calls (Sprint 049, family "module names ansi").
 *
 * No C runtime. mono-2.0-bdwgc.dll's start-up calls GetModuleFileNameA (look-ahead 2026-10-11); UnityPlayer.dll
 * and mono import GetModuleHandleA (specs/work_list.json, family run_time_loading). Paths differ between machines,
 * so rows print each answer against the W call's: the A text equals the W text narrowed (the paths are ASCII),
 * the lengths, the short-buffer forms, and the last error after SetLastError(0xDEAD).
 */
#include <windows.h>

#define SENTINEL 0xDEADu

extern IMAGE_DOS_HEADER __ImageBase;

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
static void end(void) { put("}\n"); }

static WCHAR wide[MAX_PATH];
static char narrow[MAX_PATH + 8];

static int same(const char *a, const WCHAR *w, DWORD len) {
    for (DWORD i = 0; i < len; i++) if ((unsigned char)a[i] != w[i]) return 0;
    return 1;
}

static void file_name(const char *name, HMODULE m, DWORD size) {
    DWORD wl = GetModuleFileNameW(m, wide, MAX_PATH);
    for (unsigned i = 0; i < sizeof narrow; i++) narrow[i] = (char)0xA5;
    SetLastError(SENTINEL);
    DWORD r = GetModuleFileNameA(m, narrow, size);
    DWORD err = GetLastError();
    row(name); field("ret", r); field("err", err); field("w_len", wl);
    field("ret_is_w_len", r == wl);
    DWORD shown = r < size ? r : (size ? size - 1 : 0);
    field("text_is_w", same(narrow, wide, shown)); field("nul_at_shown", size && narrow[shown] == 0);
    field("byte_after", (unsigned char)narrow[size < sizeof narrow ? size : 0]);
    end();
}

void start(void) {
    HMODULE k32 = GetModuleHandleW(L"kernel32.dll");
    file_name("file.image", NULL, MAX_PATH);
    file_name("file.kernel32", k32, MAX_PATH);
    DWORD wl = GetModuleFileNameW(NULL, wide, MAX_PATH);
    file_name("file.exact", NULL, wl + 1);
    file_name("file.short", NULL, wl);
    file_name("file.one", NULL, 1);

    static const struct { const char *row; const char *name; } HANDLES[] = {
        {"handle.kernel32", "kernel32.dll"}, {"handle.upper", "KERNEL32.DLL"}, {"handle.no_extension", "kernel32"},
        {"handle.missing", "w2m_no_such.dll"},
    };
    for (unsigned i = 0; i < sizeof HANDLES / sizeof HANDLES[0]; i++) {
        SetLastError(SENTINEL);
        HMODULE h = GetModuleHandleA(HANDLES[i].name);
        DWORD err = GetLastError();
        row(HANDLES[i].row); field("is_kernel32", h == k32); field("null", h == NULL); field("err", err); end();
    }
    SetLastError(SENTINEL);
    HMODULE self = GetModuleHandleA(NULL);
    DWORD err = GetLastError();
    row("handle.null"); field("is_image", self == (HMODULE)&__ImageBase); field("err", err); end();

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
