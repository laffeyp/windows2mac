// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* process_parameters.c — PEB.ProcessParameters (RTL_USER_PROCESS_PARAMETERS) as Windows fills it (Sprint 059,
 * family "process_parameters").
 *
 * The C runtime linked into UnityPlayer.dll, the game's exe and UnityCrashHandler64.exe reads
 * ProcessParameters->Flags from the PEB with no call (UnityPlayer RVA 0x143a6c6 and 0x143a70d: mov rcx,
 * [PEB+0x20]; cmp dword [rcx+8]); the game ran on it into a NULL in Sprint 055. No module of the game reads another
 * field directly (a scan for gs:[0x60] then +0x20, Sprint 059); the probe records the whole block so w2m fills it
 * as Windows does. Offsets: phnt's ntrtl.h (Process Hacker's reverse-engineered headers; ImagePathName at 0x60 and
 * CommandLine at 0x70 agree with winternl.h on Microsoft Learn).
 *
 * No C runtime; a GUI program, as the game is. Started by the runner (the parent: standard handles redirected), it
 * records its block, starts itself again with "child" as Steam starts the game (an empty STARTUPINFO, no handles
 * inherited), and prints the child's rows, which the child wrote to w2m-params-child.jsonl. Then the parent changes
 * its directory, a standard handle and its environment, and records what the block holds after each.
 * Values that differ by run (handles, addresses, paths) are kept as relations.
 */
#include <windows.h>

#define SENTINEL 0xDEADu

static char out[32768];
static unsigned n;
static void put(const char *s) { while (*s && n < sizeof out - 64) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static const char *prefix;
static void row(const char *s) { put("{\"s\":\""); put(prefix); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void hexfield(const char *k, unsigned long long v) {
    static const char d[] = "0123456789abcdef";
    put(",\""); put(k); put("\":\"0x");
    int started = 0;
    for (int i = 60; i >= 0; i -= 4) {
        unsigned x = (unsigned)(v >> i) & 15;
        if (x || started || i == 0) { out[n++] = d[x]; started = 1; }
    }
    put("\"");
}
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void wide(const char *k, const WCHAR *w, unsigned units) {
    put(",\""); put(k); put("\":\"");
    for (unsigned i = 0; w && i < units && n < sizeof out - 64; i++) {
        WCHAR c = w[i];
        if (c == '\\' || c == '"') out[n++] = '\\';
        out[n++] = (c >= 32 && c < 127) ? (char)c : '?';
    }
    put("\"");
}
static void end(void) { put("}\n"); }

static unsigned char *params(void) {
    unsigned char *peb;
    __asm__ volatile("movq %%gs:0x60, %0" : "=r"(peb));
    return *(unsigned char **)(peb + 0x20);
}
#define U16(o) (*(const USHORT *)(p + (o)))
#define U32(o) (*(const ULONG *)(p + (o)))
#define PTR(o) (*(void *const *)(p + (o)))
#define BUF(o) (*(const WCHAR *const *)(p + (o) + 8))

static unsigned wlen(const WCHAR *s) { unsigned k = 0; while (s[k]) k++; return k; }
static int same(const WCHAR *a, const WCHAR *b, unsigned units) {
    for (unsigned i = 0; i < units; i++) if (a[i] != b[i]) return 0;
    return 1;
}
static int inside(const unsigned char *p, const void *q) {
    return (const unsigned char *)q >= p && (const unsigned char *)q < p + U32(0);
}
static unsigned block_bytes(const WCHAR *e) {          /* an environment block's bytes, its last NUL included */
    unsigned k = 0;
    while (e[k] || e[k + 1]) k++;
    return 2 * (k + 2);
}

static void record(void) {
    const unsigned char *p = params();
    row("present"); flag("non_null", p != NULL); end();
    if (!p) return;
    row("header"); hexfield("maximum_length", U32(0)); hexfield("length", U32(4)); hexfield("flags", U32(8));
    hexfield("debug_flags", U32(0xC)); hexfield("console_flags", U32(0x18)); field("loader_threads", U32(0x40C));
    flag("group_is_self", U32(0x408) == GetCurrentProcessId()); flag("group_zero", U32(0x408) == 0); end();

    ULONG_PTR console = (ULONG_PTR)PTR(0x10);
    row("console_handle"); flag("null", console == 0);
    if (console >= (ULONG_PTR)-16) hexfield("pseudo", console); end();

    row("standard_handles");
    flag("input_is_std", PTR(0x20) == GetStdHandle(STD_INPUT_HANDLE)); flag("input_null", PTR(0x20) == NULL);
    flag("output_is_std", PTR(0x28) == GetStdHandle(STD_OUTPUT_HANDLE)); flag("output_null", PTR(0x28) == NULL);
    flag("error_is_std", PTR(0x30) == GetStdHandle(STD_ERROR_HANDLE)); flag("error_null", PTR(0x30) == NULL); end();

    WCHAR dir[MAX_PATH + 2];
    DWORD dn = GetCurrentDirectoryW(MAX_PATH, dir);
    row("current_directory"); hexfield("max_length", U16(0x3A));
    flag("length_is_dir_and_slash", U16(0x38) == 2 * (dn + (dir[dn - 1] != '\\')));
    flag("text_is_dir", same(BUF(0x38), dir, dn)); flag("ends_in_slash", BUF(0x38)[U16(0x38) / 2 - 1] == '\\');
    flag("nul_after", BUF(0x38)[U16(0x38) / 2] == 0);
    flag("handle_non_null", PTR(0x48) != NULL); flag("buffer_inside", inside(p, BUF(0x38))); end();

    row("dll_path"); field("length", U16(0x50)); field("max_length", U16(0x52)); flag("buffer_null", BUF(0x50) == NULL); end();

    WCHAR exe[MAX_PATH];
    DWORD en = GetModuleFileNameW(NULL, exe, MAX_PATH);
    row("image_path"); flag("length_is_module", U16(0x60) == 2 * en); field("max_minus_length", U16(0x62) - U16(0x60));
    flag("text_is_module", same(BUF(0x60), exe, en)); flag("nul_after", BUF(0x60)[en] == 0);
    flag("buffer_inside", inside(p, BUF(0x60))); end();

    const WCHAR *line = GetCommandLineW();
    unsigned ln = wlen(line);
    row("command_line"); flag("buffer_is_get_command_line", BUF(0x70) == line); flag("length_is_line", U16(0x70) == 2 * ln);
    field("max_minus_length", U16(0x72) - U16(0x70)); flag("buffer_inside", inside(p, BUF(0x70))); end();

    const WCHAR *env = (const WCHAR *)PTR(0x80);
    WCHAR *copy = GetEnvironmentStringsW();
    unsigned used = block_bytes(env);
    row("environment"); flag("contents_equal_copy", used == block_bytes(copy) && same(env, copy, used / 2));
    flag("copy_is_block", copy == env); flag("inside", inside(p, env));
    flag("size_is_used", *(const ULONG_PTR *)(p + 0x3F0) == used);
    flag("size_at_least_used", *(const ULONG_PTR *)(p + 0x3F0) >= used); field("version", *(const ULONG_PTR *)(p + 0x3F8));
    FreeEnvironmentStringsW(copy); end();

    STARTUPINFOW si;
    for (unsigned i = 0; i < sizeof si; i++) ((char *)&si)[i] = 0;
    si.cb = sizeof si;
    GetStartupInfoW(&si);
    row("window"); field("x", U32(0x88)); field("y", U32(0x8C)); field("count_x", U32(0x90)); field("count_y", U32(0x94));
    field("chars_x", U32(0x98)); field("chars_y", U32(0x9C)); field("fill", U32(0xA0)); hexfield("window_flags", U32(0xA4));
    field("show_window", U32(0xA8));
    flag("flags_are_startup", U32(0xA4) == si.dwFlags); flag("show_is_startup", U32(0xA8) == si.wShowWindow); end();

    row("strings");
    flag("title_is_module", U16(0xB0) == 2 * en && same(BUF(0xB0), exe, en));
    flag("title_is_startup", si.lpTitle != NULL && BUF(0xB0) == si.lpTitle);
    wide("desktop", BUF(0xC0), U16(0xC0) / 2); flag("desktop_is_startup", si.lpDesktop != NULL && BUF(0xC0) == si.lpDesktop);
    field("shell_length", U16(0xD0)); field("runtime_data_length", U16(0xE0)); end();

    unsigned drives = 0;
    for (int i = 0; i < 32; i++) if (U16(0xF0 + 0x18 * i + 2) || BUF(0xF0 + 0x18 * i + 8)) drives++;
    row("drive_directories"); field("set", drives); end();

    MEMORY_BASIC_INFORMATION mbi;
    VirtualQuery(p, &mbi, sizeof mbi);
    row("block"); flag("own_allocation", mbi.AllocationBase == p); hexfield("protect", mbi.Protect);
    hexfield("type", mbi.Type); flag("in_process_heap", mbi.AllocationBase == (void *)GetProcessHeap());
    hexfield("region_size", mbi.RegionSize); end();
}

static const WCHAR ROWS[] = L"w2m-params-child.jsonl";

static int is_child(void) {
    const WCHAR *c = GetCommandLineW();
    unsigned len = wlen(c);
    return len >= 6 && c[len - 6] == ' ' && c[len - 5] == 'c' && c[len - 4] == 'h' && c[len - 3] == 'i' &&
           c[len - 2] == 'l' && c[len - 1] == 'd';
}

void start(void) {
    DWORD written;
    if (is_child()) {
        prefix = "child.";
        record();
        HANDLE f = CreateFileW(ROWS, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
        WriteFile(f, out, n, &written, NULL);
        CloseHandle(f);
        ExitProcess(0);
    }
    HANDLE std_out = GetStdHandle(STD_OUTPUT_HANDLE);
    prefix = "parent.";
    record();

    WCHAR exe[MAX_PATH], cmd[MAX_PATH + 16];
    DWORD en = GetModuleFileNameW(NULL, exe, MAX_PATH);
    unsigned k = 0;
    cmd[k++] = '"';
    for (DWORD i = 0; i < en; i++) cmd[k++] = exe[i];
    cmd[k++] = '"';
    static const WCHAR tail[] = L" child";
    for (int i = 0; tail[i]; i++) cmd[k++] = tail[i];
    cmd[k] = 0;
    STARTUPINFOW si;
    for (unsigned i = 0; i < sizeof si; i++) ((char *)&si)[i] = 0;
    si.cb = sizeof si;
    PROCESS_INFORMATION pi = {0};
    BOOL ok = CreateProcessW(NULL, cmd, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi);
    if (ok) {
        WaitForSingleObject(pi.hProcess, 30000);
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);
    }
    row("child_started"); flag("ret", ok); end();
    HANDLE f = CreateFileW(ROWS, GENERIC_READ, 0, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    if (f != INVALID_HANDLE_VALUE) {
        DWORD got = 0;
        ReadFile(f, out + n, sizeof out - 64 - n, &got, NULL);
        n += got;
        CloseHandle(f);
        DeleteFileW(ROWS);
    }

    const unsigned char *p = params();
    const WCHAR *before = BUF(0x38);
    CreateDirectoryW(L"w2m_params_dir", NULL);
    SetCurrentDirectoryW(L"w2m_params_dir");
    WCHAR dir[MAX_PATH];
    DWORD dn = GetCurrentDirectoryW(MAX_PATH, dir);
    row("after.current_directory"); flag("text_is_dir", same(BUF(0x38), dir, dn) && BUF(0x38)[dn] == '\\');
    flag("same_buffer", BUF(0x38) == before); flag("length_is_dir_and_slash", U16(0x38) == 2 * (dn + 1)); end();
    SetCurrentDirectoryW(L"..");
    RemoveDirectoryW(L"w2m_params_dir");

    HANDLE saved = GetStdHandle(STD_ERROR_HANDLE);
    SetStdHandle(STD_ERROR_HANDLE, (HANDLE)(ULONG_PTR)0x1234);
    row("after.set_std_handle"); flag("block_holds_new", PTR(0x30) == (void *)(ULONG_PTR)0x1234); end();
    SetStdHandle(STD_ERROR_HANDLE, saved);

    const void *env_before = PTR(0x80);
    ULONG_PTR size_before = *(const ULONG_PTR *)(p + 0x3F0);
    SetEnvironmentVariableW(L"W2M_PARAMS_VAR", L"a_value");
    const WCHAR *env = (const WCHAR *)PTR(0x80);
    int found = 0;
    for (const WCHAR *e = env; *e; e += wlen(e) + 1)
        if (same(e, L"W2M_PARAMS_VAR=a_value", 23)) found = 1;
    row("after.set_environment"); flag("same_pointer", env == env_before); flag("holds_new", found);
    flag("size_same", *(const ULONG_PTR *)(p + 0x3F0) == size_before);
    flag("size_is_used", *(const ULONG_PTR *)(p + 0x3F0) == block_bytes(env)); end();

    WriteFile(std_out, out, n, &written, NULL);
    ExitProcess(0);
}
