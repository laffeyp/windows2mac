// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* activation.c — the activation messages a first SetFocus sends, by how the process was started (Sprint 065).
 *
 * No C runtime. Game run 38216081440 (seq 3381-3391; the game started by pwsh's Start-Process through the
 * jit_capture stub, a GUI program with no console, which starts the game with CreateProcessW) has WM_ACTIVATEAPP's
 * lParam 0 and WM_NCACTIVATE's wParam 0; the windows probe (runs 38210803867, 38210806334; started by cmd with its
 * console) has the lParam not NULL and the wParam 1. This probe starts itself four ways and each copy, a trial,
 * records the foreground window before it, creates the batch run's window (the RegisterClassW class by atom, style 0,
 * CW_USEDEFAULT four times), calls SetFocus on it and records each activation message and the state after:
 *   - console: CreateProcessW, the console shared
 *   - detached: CreateProcessW with DETACHED_PROCESS: no console, as a GUI program
 *   - stub: a detached copy that starts the trial detached and waits, as jit_capture starts the game
 *   - shell: ShellExecuteExW (SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NO_CONSOLE, SW_SHOWNORMAL), as Start-Process does
 *   - late2, late8, late20: as console, with that many seconds between CreateWindowExW and SetFocus; stub_late20 as
 *     stub (the game creates its window at seq 1659 and calls SetFocus at 3380; the first four ways all have the
 *     lParam not NULL and the wParam 1, runs 38273292412 and 38273295692)
 *   - child, pump, child_pump, stub_child_pump: before its window the trial starts an idle copy of itself (the game
 *     starts UnityCrashHandler64 at seq 18), or pumps its messages as the game does before SetFocus (PeekMessageA
 *     NULL 0 0 PM_NOREMOVE, then PM_REMOVE with TranslateMessage and DispatchMessageA until empty), or both; the late
 *     ways change nothing (runs 38273502864, 38273506587)
 *   - gui_shell, gui_create, gui_shell_child_pump: a copy of the probe whose PE header says IMAGE_SUBSYSTEM_WINDOWS_GUI
 *     (the game is jit_capture, a GUI program, started by Start-Process: ShellExecuteEx), started by ShellExecuteExW
 *     (SEE_MASK_NOCLOSEPROCESS, SW_SHOWNORMAL) or CreateProcessW; the child and pump ways change nothing either (runs
 *     38274040670, 38274044288), and every game run (six) has the lParam 0 and the wParam 0
 * A trial writes its rows to the file its command line names; the first copy prints them in order. Rows:
 * <way>.before, <way>.msg.<i> (WM_ACTIVATEAPP, WM_NCACTIVATE, WM_ACTIVATE, WM_KILLFOCUS, WM_SETFOCUS), <way>.after.
 */
#include <windows.h>
#include <shellapi.h>

static char out[16384];
static unsigned n;
static HANDLE sink;

static void put(const char *s) { while (*s) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void flush(void) {
    DWORD written;
    WriteFile(sink, out, n, &written, NULL);
    n = 0;
}
static const char *way;
static void row(const char *s) { put("{\"s\":\""); put(way); put("."); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void end(void) { put("}\n"); flush(); }

static void wcat(WCHAR *d, const WCHAR *s);
static WCHAR exe[MAX_PATH];
static HWND window;
static DWORD foreground_thread;
static unsigned messages;

static LRESULT CALLBACK procedure(HWND w, UINT msg, WPARAM wp, LPARAM lp) {
    if (msg == WM_ACTIVATEAPP || msg == WM_NCACTIVATE || msg == WM_ACTIVATE || msg == WM_KILLFOCUS ||
        msg == WM_SETFOCUS) {
        char s[16] = "msg.";
        unsigned i = messages++, k = 4;
        if (i >= 10) s[k++] = (char)('0' + i / 10);
        s[k++] = (char)('0' + i % 10);
        s[k] = 0;
        row(s); field("msg", msg);
        if (msg == WM_KILLFOCUS || msg == WM_SETFOCUS) {
            flag("wparam_null", wp == 0); flag("wparam_is_self", (HWND)wp == w);
        } else {
            field("wparam", wp & 0xFFFF);
        }
        flag("lparam_null", lp == 0);
        flag("lparam_is_foreground_thread", foreground_thread && (DWORD)lp == foreground_thread);
        flag("lparam_is_own_thread", (DWORD)lp == GetCurrentThreadId());
        if (msg == WM_ACTIVATE) flag("lparam_is_foreground_window", lp && (HWND)lp == GetForegroundWindow());
        end();
    }
    return DefWindowProcW(w, msg, wp, lp);
}

static DWORD delay_ms;
static int with_child, with_pump;
static HANDLE idle_child;

static int contains(const WCHAR *s, const WCHAR *w) {
    for (; *s; s++) {
        int i = 0;
        while (w[i] && s[i] == w[i]) i++;
        if (!w[i]) return 1;
    }
    return 0;
}

static void pump(void) {
    MSG m;
    PeekMessageA(&m, NULL, 0, 0, PM_NOREMOVE);
    while (PeekMessageA(&m, NULL, 0, 0, PM_REMOVE)) {
        TranslateMessage(&m);
        DispatchMessageA(&m);
    }
}

static void trial(void) {
    HWND fg = GetForegroundWindow();
    DWORD pid = 0;
    foreground_thread = fg ? GetWindowThreadProcessId(fg, &pid) : 0;
    row("before"); flag("console_null", GetConsoleWindow() == NULL); flag("foreground_null", fg == NULL);
    flag("foreground_is_console", fg && fg == GetConsoleWindow());
    flag("foreground_own_process", fg && pid == GetCurrentProcessId());
    flag("active_null", GetActiveWindow() == NULL); flag("focus_null", GetFocus() == NULL); end();

    if (with_child) {
        static WCHAR cmd[1024];
        cmd[0] = 0;
        wcat(cmd, L"\""); wcat(cmd, exe); wcat(cmd, L"\" idle x x");
        STARTUPINFOW si = {0};
        si.cb = sizeof si;
        PROCESS_INFORMATION pi;
        if (CreateProcessW(exe, cmd, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi)) {
            CloseHandle(pi.hThread);
            idle_child = pi.hProcess;
        }
        row("child"); flag("started", idle_child != NULL); end();
    }
    WNDCLASSW c = {0};
    c.lpfnWndProc = procedure;
    c.hInstance = GetModuleHandleW(NULL);
    c.lpszClassName = L"W2mProbeActivation";
    ATOM atom = RegisterClassW(&c);
    window = CreateWindowExW(0, (LPCWSTR)(ULONG_PTR)atom, L"Hail to the Rainbow", 0, CW_USEDEFAULT, CW_USEDEFAULT,
                             CW_USEDEFAULT, CW_USEDEFAULT, NULL, NULL, c.hInstance, NULL);
    if (delay_ms) Sleep(delay_ms);
    if (with_pump) pump();
    row("before_focus"); flag("foreground_null", GetForegroundWindow() == NULL);
    flag("foreground_is_window", GetForegroundWindow() == window); end();
    SetLastError(0xDEAD);
    HWND previous = SetFocus(window);
    DWORD err = GetLastError();
    row("after"); flag("window_null", window == NULL); flag("previous_null", previous == NULL);
    field("err", err); field("messages", messages); flag("foreground_is_window", GetForegroundWindow() == window);
    flag("active_is_window", GetActiveWindow() == window); flag("focus_is_window", GetFocus() == window); end();
    if (idle_child) {
        TerminateProcess(idle_child, 0);
        CloseHandle(idle_child);
    }
}

/* The command line's words after the program: start, then 0, 1 or 2 more (no quotes inside a word but the path's). */
static WCHAR line[1024];
static WCHAR *words[4];
static int count;
static void split(void) {
    WCHAR *p = GetCommandLineW();
    unsigned k = 0;
    while (*p && k < 1023) line[k++] = *p++;
    line[k] = 0;
    p = line;
    while (*p && count < 4) {
        while (*p == L' ') p++;
        if (!*p) break;
        int quoted = *p == L'"';
        if (quoted) p++;
        words[count++] = p;
        while (*p && (quoted ? *p != L'"' : *p != L' ')) p++;
        if (*p) *p++ = 0;
    }
}

static void wcopy(WCHAR *d, const WCHAR *s) { while ((*d++ = *s++)) {} }
static void wcat(WCHAR *d, const WCHAR *s) { while (*d) d++; wcopy(d, s); }

static WCHAR gui[MAX_PATH];
static const WCHAR *target = exe;

/* gui: this file with the optional header's Subsystem (offset 68, PE32 and PE32+ alike) set to 2 */
static int write_gui_copy(void) {
    static unsigned char image[1 << 20];
    HANDLE f = CreateFileW(exe, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, 0, NULL);
    if (f == INVALID_HANDLE_VALUE) return 0;
    DWORD got = 0;
    ReadFile(f, image, sizeof image, &got, NULL);
    CloseHandle(f);
    DWORD pe = *(DWORD *)(image + 0x3C);
    *(WORD *)(image + pe + 4 + 20 + 68) = IMAGE_SUBSYSTEM_WINDOWS_GUI;
    GetTempPathW(MAX_PATH, gui);
    wcat(gui, L"w2m_activation_gui.exe");
    HANDLE g = CreateFileW(gui, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, 0, NULL);
    if (g == INVALID_HANDLE_VALUE) return 0;
    DWORD written = 0;
    WriteFile(g, image, got, &written, NULL);
    CloseHandle(g);
    return written == got;
}

static void start_copy(const WCHAR *mode, const WCHAR *name, const WCHAR *path, DWORD flags) {
    static WCHAR cmd[1024];
    cmd[0] = 0;
    wcat(cmd, L"\""); wcat(cmd, target); wcat(cmd, L"\" "); wcat(cmd, mode); wcat(cmd, L" "); wcat(cmd, name);
    wcat(cmd, L" \""); wcat(cmd, path); wcat(cmd, L"\"");
    STARTUPINFOW si = {0};
    si.cb = sizeof si;
    PROCESS_INFORMATION pi;
    if (CreateProcessW(target, cmd, NULL, NULL, FALSE, flags, NULL, NULL, &si, &pi)) {
        WaitForSingleObject(pi.hProcess, 60000);
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);
    }
}

static void run_way(const char *name8, const WCHAR *name) {
    static WCHAR path[MAX_PATH];
    GetTempPathW(MAX_PATH, path);
    wcat(path, L"w2m_activation_"); wcat(path, name); wcat(path, L".jsonl");
    DeleteFileW(path);
    target = contains(name, L"gui") ? gui : exe;
    if (contains(name, L"stub")) start_copy(L"stub", name, path, DETACHED_PROCESS);
    else if (contains(name, L"detached")) start_copy(L"trial", name, path, DETACHED_PROCESS);
    else if (!contains(name, L"shell")) start_copy(L"trial", name, path, 0);     /* console, late, child, pump, gui_create */
    else {
        static WCHAR params[1024];
        params[0] = 0;
        wcat(params, L"trial "); wcat(params, name); wcat(params, L" \""); wcat(params, path); wcat(params, L"\"");
        SHELLEXECUTEINFOW e = {0};
        e.cbSize = sizeof e;
        e.fMask = contains(name, L"gui") ? SEE_MASK_NOCLOSEPROCESS : SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NO_CONSOLE;
        e.lpFile = target;
        e.lpParameters = params;
        e.nShow = SW_SHOWNORMAL;
        if (ShellExecuteExW(&e) && e.hProcess) {
            WaitForSingleObject(e.hProcess, 60000);
            CloseHandle(e.hProcess);
        }
    }
    HANDLE f = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, 0, NULL);
    sink = GetStdHandle(STD_OUTPUT_HANDLE);
    if (f == INVALID_HANDLE_VALUE) {
        way = name8;
        row("missing"); end();
        return;
    }
    static char buf[16384];
    DWORD got = 0;
    ReadFile(f, buf, sizeof buf, &got, NULL);
    CloseHandle(f);
    DWORD written;
    WriteFile(sink, buf, got, &written, NULL);
}

void start(void) {
    GetModuleFileNameW(NULL, exe, MAX_PATH);
    split();
    if (count >= 4 && words[1][0] == L't') {                     /* trial <way> <path> */
        static char name8[16];
        for (int i = 0; i < 15 && words[2][i]; i++) name8[i] = (char)words[2][i];
        way = name8;
        for (int i = 0; words[2][i]; i++)
            if (words[2][i] >= L'0' && words[2][i] <= L'9') delay_ms = delay_ms * 10 + (DWORD)(words[2][i] - L'0');
        delay_ms *= 1000;
        with_child = contains(words[2], L"child");
        with_pump = contains(words[2], L"pump");
        sink = CreateFileW(words[3], GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, 0, NULL);
        trial();
        CloseHandle(sink);
        ExitProcess(0);
    }
    if (count >= 2 && words[1][0] == L'i') {                     /* idle: as UnityCrashHandler64 waits */
        Sleep(60000);
        ExitProcess(0);
    }
    if (count >= 4 && words[1][0] == L's') {                     /* stub <way> <path>: the trial, detached */
        start_copy(L"trial", words[2], words[3], DETACHED_PROCESS);
        ExitProcess(0);
    }
    run_way("console", L"console");
    run_way("detached", L"detached");
    run_way("stub", L"stub");
    run_way("shell", L"shell");
    run_way("late2", L"late2");
    run_way("late8", L"late8");
    run_way("late20", L"late20");
    run_way("stub_late20", L"stub_late20");
    run_way("child", L"child");
    run_way("pump", L"pump");
    run_way("child_pump", L"child_pump");
    run_way("stub_child_pump", L"stub_child_pump");
    sink = GetStdHandle(STD_OUTPUT_HANDLE);
    way = "gui";
    row("copy"); flag("written", write_gui_copy()); end();
    run_way("gui_shell", L"gui_shell");
    run_way("gui_create", L"gui_create");
    run_way("gui_shell_child_pump", L"gui_shell_child_pump");
    ExitProcess(0);
}
