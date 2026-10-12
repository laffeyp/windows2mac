// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* module_search.c — how Windows answers a program that enumerates its modules and moves its DLL search (Sprint 063,
 * family "module search").
 *
 * No C runtime: entry `start`, kernel32 and psapi imports only. The game makes these calls (game run 38206263976):
 *   - mono-2.0-bdwgc.dll: EnumProcessModules(GetCurrentProcess(), a buffer, 0x2000, &needed) after
 *     GetModuleHandleW(NULL), seq 2128 and 2210, answering 1 with the last error left;
 *   - UnityPlayer.dll: SetDllDirectoryW(a directory), LoadLibraryW(L"steam_api64.dll"), SetDllDirectoryW(NULL),
 *     twice, seq 2915-2920, each SetDllDirectoryW answering 1.
 * The rows hold those forms and the edges the implementation accepts. A module list's handles and length differ by
 * machine (each system loads other DLLs), so the rows print relations: the list's first entries in load order, that
 * the length the call reports is the list's, and where a module the probe loads appears. The DLL search is tested with
 * the probe's own DLL copied into a subdirectory under two names that no other search finds. Every error is read
 * after SetLastError(0xDEAD).
 */
#include <windows.h>
#include <psapi.h>

#define SENTINEL 0xDEADu
#define MODULES 512

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

static HMODULE list[MODULES];
static DWORD err;

static int listed(HMODULE h, DWORD count) {
    for (DWORD i = 0; i < count; i++)
        if (list[i] == h) return 1;
    return 0;
}

static void wcopy(WCHAR *to, const WCHAR *from) { while ((*to++ = *from++)) {} }
static void wcat(WCHAR *to, const WCHAR *from) { while (*to) to++; wcopy(to, from); }

/* the probe's directory with a trailing backslash */
static WCHAR dir[MAX_PATH];
static void own_directory(void) {
    DWORD k = GetModuleFileNameW(NULL, dir, MAX_PATH);
    while (k && dir[k - 1] != L'\\') k--;
    dir[k] = 0;
}

/* the DLL beside the probe, written again under `name` in w2m_search\ (no CopyFileW: the bytes read and written) */
static int copy_dll(const WCHAR *name) {
    static char bytes[65536];
    WCHAR from[MAX_PATH], to[MAX_PATH];
    wcopy(from, dir); wcat(from, L"module_search_dll.dll");
    wcopy(to, dir); wcat(to, L"w2m_search\\"); wcat(to, name);
    HANDLE f = CreateFileW(from, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    if (f == INVALID_HANDLE_VALUE) return 0;
    DWORD got = 0;
    BOOL ok = ReadFile(f, bytes, sizeof bytes, &got, NULL);
    CloseHandle(f);
    if (!ok || got == 0 || got == sizeof bytes) return 0;
    HANDLE t = CreateFileW(to, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    if (t == INVALID_HANDLE_VALUE) return 0;
    DWORD put_bytes = 0;
    ok = WriteFile(t, bytes, got, &put_bytes, NULL);
    CloseHandle(t);
    return ok && put_bytes == got;
}

static HMODULE load(const WCHAR *name) {
    SetLastError(SENTINEL);
    HMODULE h = LoadLibraryW(name);
    err = GetLastError();
    return h;
}

static BOOL set_directory(const WCHAR *path) {
    SetLastError(SENTINEL);
    BOOL ok = SetDllDirectoryW(path);
    err = GetLastError();
    return ok;
}

void start(void) {
    own_directory();
    HMODULE exe = GetModuleHandleW(NULL);
    HMODULE nt = GetModuleHandleW(L"ntdll.dll"), k32 = GetModuleHandleW(L"kernel32.dll");
    HMODULE kb = GetModuleHandleW(L"kernelbase.dll");

    /* the game's form: the current process, a buffer of 0x2000 bytes */
    DWORD needed = 0xA5A5;
    SetLastError(SENTINEL);
    BOOL ok = EnumProcessModules(GetCurrentProcess(), list, 0x2000, &needed);
    err = GetLastError();
    DWORD count = needed / sizeof(HMODULE);
    int nonzero = 1;
    for (DWORD i = 0; i < count && i < MODULES; i++) nonzero &= list[i] != NULL;
    row("enum.full"); field("ret", ok != 0); field("err", err); flag("needed_whole", needed % sizeof(HMODULE) == 0);
    flag("at_least_four", count >= 4); flag("fits", needed <= 0x2000); flag("first_is_program", list[0] == exe);
    flag("second_is_ntdll", list[1] == nt); flag("third_is_kernel32", list[2] == k32);
    flag("fourth_is_kernelbase", list[3] == kb); flag("all_nonzero", nonzero); end();
    DWORD full = needed;

    /* a buffer for one handle: the call still reports the whole list's length */
    HMODULE one[1] = {0};
    needed = 0xA5A5;
    SetLastError(SENTINEL);
    ok = EnumProcessModules(GetCurrentProcess(), one, sizeof one, &needed);
    err = GetLastError();
    row("enum.short"); field("ret", ok != 0); field("err", err); flag("needed_is_full", needed == full);
    flag("first_is_program", one[0] == exe); end();

    /* no buffer: the length alone */
    needed = 0xA5A5;
    SetLastError(SENTINEL);
    ok = EnumProcessModules(GetCurrentProcess(), NULL, 0, &needed);
    err = GetLastError();
    row("enum.size_only"); field("ret", ok != 0); field("err", err); flag("needed_is_full", needed == full); end();

    /* SetDllDirectoryW: a DLL only in w2m_search\ is found while the directory is set, and not before or after */
    WCHAR search[MAX_PATH];
    wcopy(search, dir); wcat(search, L"w2m_search");
    CreateDirectoryW(search, NULL);
    int copied = copy_dll(L"w2m_found.dll") && copy_dll(L"w2m_later.dll");
    row("search.setup"); flag("copied", copied); end();

    HMODULE before = load(L"w2m_found.dll");
    row("search.before"); flag("loaded", before != NULL); field("err", err); end();

    ok = set_directory(search);
    row("search.set"); field("ret", ok != 0); field("err", err); end();

    HMODULE found = load(L"w2m_found.dll");
    row("search.load"); flag("loaded", found != NULL); field("err", err);
    typedef DWORD (*value_fn)(void);
    value_fn value = found ? (value_fn)(void *)GetProcAddress(found, "module_search_value") : NULL;
    field("value", value ? value() : 0);
    WCHAR path[MAX_PATH];
    DWORD k = found ? GetModuleFileNameW(found, path, MAX_PATH) : 0;
    int in_search = k > 25;
    const WCHAR *tail = L"w2m_search\\w2m_found.dll";
    for (DWORD i = 0; in_search && i < 24; i++) in_search = path[k - 24 + i] == tail[i];
    flag("path_in_directory", in_search); end();

    needed = 0;
    EnumProcessModules(GetCurrentProcess(), list, sizeof list, &needed);
    count = needed / sizeof(HMODULE);
    row("enum.after_load"); field("one_more", count == full / sizeof(HMODULE) + 1);
    flag("listed", listed(found, count)); flag("last", count && list[count - 1] == found); end();

    ok = set_directory(NULL);
    row("search.reset"); field("ret", ok != 0); field("err", err); end();

    HMODULE again = load(L"w2m_found.dll");            /* loaded already: found by name, as the game's second load is */
    row("search.loaded_after_reset"); flag("same", again == found); field("err", err); end();

    HMODULE later = load(L"w2m_later.dll");            /* only in w2m_search\: not found once the directory is reset */
    row("search.after_reset"); flag("loaded", later != NULL); field("err", err); end();

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
