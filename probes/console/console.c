/* console.c — standard handles and the console in a GUI process started without handles (Sprint 044,
 * family "console").
 *
 * No C runtime; built as a GUI program (subsystem windows), as the launcher is. The launcher's C
 * runtime asks for its standard handles after the heap is ready (frontier 2026-10-10, 7,531
 * instructions). A game started by Steam is a GUI process that inherits no handles; the runner's cmd
 * instead gives this binary redirected ones. So the binary runs twice: started by cmd (the parent),
 * it starts itself again with no inherited handles and an empty STARTUPINFO (the child, the
 * Steam-like start), waits, and copies the child's rows from a file to its own standard output. The
 * child's rows: GetStdHandle, GetFileType, GetConsoleMode, GetConsoleCP, WriteConsoleW, SetStdHandle,
 * FlushFileBuffers and GetStartupInfoW, each with its return and last error (after SetLastError(0xDEAD)).
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

static DWORD err;
static const WCHAR ROWS[] = L"w2m-console-child.jsonl";

static int is_child(void) {
    const WCHAR *c = GetCommandLineW();
    DWORD len = 0;
    while (c[len]) len++;
    return len >= 6 && c[len - 6] == ' ' && c[len - 5] == 'c' && c[len - 4] == 'h' && c[len - 3] == 'i' &&
           c[len - 2] == 'l' && c[len - 1] == 'd';
}

static void handle_rows(const char *name, DWORD which) {
    SetLastError(SENTINEL);
    HANDLE h = GetStdHandle(which);
    err = GetLastError();
    row(name); flag("null", h == 0); flag("invalid", h == INVALID_HANDLE_VALUE); field("err", err);
    SetLastError(SENTINEL);
    DWORD type = GetFileType(h);
    err = GetLastError();
    field("file_type", type); field("file_type_err", err);
    DWORD mode = 0;
    SetLastError(SENTINEL);
    BOOL ok = GetConsoleMode(h, &mode);
    err = GetLastError();
    field("console_mode_ret", ok != 0); field("console_mode_err", err);
    end();
}

static void child(void) {
    handle_rows("std.input", STD_INPUT_HANDLE);
    handle_rows("std.output", STD_OUTPUT_HANDLE);
    handle_rows("std.error", STD_ERROR_HANDLE);

    SetLastError(SENTINEL);
    UINT cp = GetConsoleCP();
    err = GetLastError();
    row("console_cp"); field("value", cp); field("err", err); end();

    DWORD wrote = 77;
    SetLastError(SENTINEL);
    BOOL ok = WriteConsoleW(GetStdHandle(STD_OUTPUT_HANDLE), L"x", 1, &wrote, NULL);
    err = GetLastError();
    row("write_console.no_console"); field("ret", ok != 0); field("written", wrote); field("err", err); end();

    HANDLE f = CreateFileW(ROWS, GENERIC_READ | GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    SetLastError(SENTINEL);
    ok = SetStdHandle(STD_OUTPUT_HANDLE, f);
    err = GetLastError();
    row("set_std_handle"); field("ret", ok != 0); field("err", err); flag("get_returns_it", GetStdHandle(STD_OUTPUT_HANDLE) == f);
    SetLastError(SENTINEL);
    DWORD type = GetFileType(f);
    err = GetLastError();
    field("file_type_disk", type); field("file_type_err", err); end();

    SetLastError(SENTINEL);
    ok = FlushFileBuffers(f);
    err = GetLastError();
    row("flush_file_buffers.disk"); field("ret", ok != 0); field("err", err); end();

    STARTUPINFOW si;
    unsigned char *b = (unsigned char *)&si;
    for (unsigned i = 0; i < sizeof si; i++) b[i] = 0xA5;
    GetStartupInfoW(&si);
    row("startup_info.child"); field("cb", si.cb); field("flags", si.dwFlags); field("show_window", si.wShowWindow);
    flag("std_input_null", si.hStdInput == 0); flag("std_output_null", si.hStdOutput == 0);
    flag("reserved_null", si.lpReserved == 0); flag("desktop_null", si.lpDesktop == 0); flag("title_null", si.lpTitle == 0);
    end();

    DWORD w;
    WriteFile(f, out, n, &w, NULL);
    CloseHandle(f);
}

static void parent(void) {
    static WCHAR self[600];
    DWORD len = GetModuleFileNameW(NULL, self, 300);
    static WCHAR cmd[700];
    DWORD k = 0;
    cmd[k++] = '"';
    for (DWORD i = 0; i < len; i++) cmd[k++] = self[i];
    const WCHAR *tail = L"\" child";
    for (DWORD i = 0; tail[i]; i++) cmd[k++] = tail[i];
    cmd[k] = 0;
    STARTUPINFOW si;
    unsigned char *b = (unsigned char *)&si;
    for (unsigned i = 0; i < sizeof si; i++) b[i] = 0;
    si.cb = sizeof si;
    PROCESS_INFORMATION pi;
    DeleteFileW(ROWS);
    if (!CreateProcessW(self, cmd, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi)) ExitProcess(2);
    WaitForSingleObject(pi.hProcess, 60000);
    CloseHandle(pi.hThread);
    CloseHandle(pi.hProcess);
    HANDLE f = CreateFileW(ROWS, GENERIC_READ, 0, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    if (f == INVALID_HANDLE_VALUE) ExitProcess(3);
    DWORD got = 0;
    ReadFile(f, out, sizeof out, &got, NULL);
    CloseHandle(f);
    DeleteFileW(ROWS);
    DWORD w;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, got, &w, NULL);
}

void start(void) {
    if (is_child()) child();
    else parent();
    ExitProcess(0);
}
