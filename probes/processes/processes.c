// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* processes.c — a second process as UnityPlayer starts its crash handler (Sprint 055, family "processes";
 * specs/work_list.json).
 *
 * No C runtime. Game run 38176899195 holds both sides. UnityPlayer: three CreateEventW(NULL, FALSE, FALSE, name) named
 * UnityCrashHandler*_<pid>; CreatePipe with inheritable ends, SetHandleInformation(write end, HANDLE_FLAG_INHERIT, 0);
 * CreateProcessW(NULL, "<dir>\UnityCrashHandler64.exe --attach <pid> <address>", NULL, NULL, TRUE,
 * NORMAL_PRIORITY_CLASS, ...) with the read end as the child's standard input; CloseHandle(read end); framed writes
 * (8 bytes, then the body); zero-timeout waits on the process. The handler: OpenProcess(SYNCHRONIZE |
 * PROCESS_QUERY_LIMITED_INFORMATION, the parent's id), CreateEventW on the three names, GetStdHandle(STD_INPUT_HANDLE),
 * reads of 8 bytes then the body, ERROR_BROKEN_PIPE when the writer is gone, WaitForMultipleObjectsEx.
 * This probe is both: run with no argument it is the parent and starts itself with "child <parent id>"; the child
 * writes its rows to w2m_child_rows.txt, which the parent prints after the child exits. Ids and handles are kept as
 * relations.
 * Sprint 058 (the review of Sprint 055) adds: a second child, "writer", started after the parent set a variable and
 * changed its directory, which prints what it inherited down a pipe and exits holding the pipe's write end; and a
 * pipe of UnityPlayer's size (0, game run 38176899195 seq 9) filled by a thread a byte at a time, then by one write
 * larger than it holds.
 */
#include <windows.h>

#define SENTINEL 0xDEADu
#define CHILD_EXIT 7u

static char out[16384];
static unsigned n;
static void put(const char *s) { while (*s && n < sizeof out - 64) out[n++] = *s++; }
static void putw(const WCHAR *s) { while (*s && n < sizeof out - 64) out[n++] = *s < 128 && *s != '"' && *s != '\\' ? (char)*s : '_', s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void wtext(const char *k, const WCHAR *v) { put(",\""); put(k); put("\":\""); putw(v); put("\""); }
static void end(void) { put("}\n"); }
static void flush_to(HANDLE h) {
    DWORD written;
    WriteFile(h, out, n, &written, NULL);
    n = 0;
}

static void event_name(WCHAR *to, const WCHAR *stem, DWORD pid) {
    int k = 0;
    for (const WCHAR *p = stem; *p; p++) to[k++] = *p;
    WCHAR t[12];
    int m = 0;
    do { t[m++] = (WCHAR)(L'0' + pid % 10); pid /= 10; } while (pid);
    while (m) to[k++] = t[--m];
    to[k] = 0;
}

static DWORD parse(const WCHAR *s) {
    DWORD v = 0;
    while (*s >= L'0' && *s <= L'9') v = v * 10 + (DWORD)(*s++ - L'0');
    return v;
}

/* ---- the child, as the crash handler ---------------------------------------------------------------- */

static int read_all(HANDLE h, void *buf, DWORD size, DWORD *err) {
    DWORD got = 0;
    SetLastError(SENTINEL);
    BOOL ok = ReadFile(h, buf, size, &got, NULL);
    *err = GetLastError();
    return ok && got == size;
}

static void child(DWORD parent_id, const WCHAR *line) {
    row("child.command_line"); wtext("tail", line); end();
    SetLastError(SENTINEL);
    HANDLE parent = OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, FALSE, parent_id);
    row("child.open_parent"); flag("non_null", parent != NULL); field("err", GetLastError()); end();
    WCHAR name[64];
    static const WCHAR *const stems[3] = {L"W2mProbe_", L"W2mProbeResp_", L"W2mProbeTerm_"};
    HANDLE events[3];
    for (int i = 0; i < 3; i++) {
        event_name(name, stems[i], parent_id);
        SetLastError(SENTINEL);
        events[i] = CreateEventW(NULL, FALSE, FALSE, name);
        row(i == 0 ? "child.event_0" : i == 1 ? "child.event_1" : "child.event_2");
        flag("non_null", events[i] != NULL); field("err", GetLastError()); end();
    }
    HANDLE in = GetStdHandle(STD_INPUT_HANDLE);
    row("child.std_handles"); flag("input", in != NULL && in != INVALID_HANDLE_VALUE);
    field("input_type", GetFileType(in)); flag("output_null", GetStdHandle(STD_OUTPUT_HANDLE) == NULL);
    flag("error_null", GetStdHandle(STD_ERROR_HANDLE) == NULL); end();
    for (int r = 0; r < 4; r++) {
        DWORD header[2] = {0, 0}, err = 0;
        char body[64];
        int ok = read_all(in, header, 8, &err);
        row(r == 0 ? "child.record_0" : r == 1 ? "child.record_1" : r == 2 ? "child.record_2" : "child.record_3");
        flag("header", ok); field("header_err", err);
        if (ok) {
            field("kind", header[0]); field("size", header[1]);
            ok = header[1] <= sizeof body && read_all(in, body, header[1], &err);
            flag("body", ok);
            if (ok) field("first", (unsigned char)body[0]);
        }
        end();
    }
    SetEvent(events[1]);                                     /* "Resp": the records are in */
    HANDLE waits[2] = {parent, events[2]};
    SetLastError(SENTINEL);
    DWORD w = WaitForMultipleObjectsEx(2, waits, FALSE, 10000, FALSE);
    row("child.wait_parent_or_term"); field("ret", w); field("err", GetLastError()); end();
    for (int i = 0; i < 3; i++) CloseHandle(events[i]);
    CloseHandle(parent);
    HANDLE f = CreateFileW(L"w2m_child_rows.txt", GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    flush_to(f);
    CloseHandle(f);
    ExitProcess(CHILD_EXIT);
}

/* ---- the second child: what a start inherits, and a writer that exits holding its end (Sprint 058) ------- */

static void writer(void) {
    WCHAR v[64];
    SetLastError(SENTINEL);
    DWORD k = GetEnvironmentVariableW(L"W2M_PROBE_VAR", v, 64);
    row("writer.environment"); field("len", k); if (k && k < 64) wtext("value", v); end();
    WCHAR d[MAX_PATH];
    DWORD m = GetCurrentDirectoryW(MAX_PATH, d);
    const WCHAR *tail = d;
    for (DWORD i = 0; i < m && i < MAX_PATH; i++) if (d[i] == L'\\') tail = d + i + 1;
    row("writer.directory"); wtext("tail", m && m < MAX_PATH ? tail : L"?"); end();
    flush_to(GetStdHandle(STD_OUTPUT_HANDLE));
    ExitProcess(5);                                          /* the pipe's write end still open */
}

/* ---- a pipe's capacity --------------------------------------------------------------------------------- */

static HANDLE fill_end, big_end;
static volatile LONG fill_count, big_done;
static DWORD big_written;
static char big[10000];

static DWORD WINAPI fill(void *unused) {
    (void)unused;
    char b = 'x';
    DWORD k;
    for (int i = 0; i < 65536; i++) {
        if (!WriteFile(fill_end, &b, 1, &k, NULL)) break;
        fill_count++;
    }
    return 0;
}

static DWORD WINAPI big_write(void *unused) {
    (void)unused;
    WriteFile(big_end, big, sizeof big, &big_written, NULL);
    big_done = 1;
    return 0;
}

static DWORD drain(HANDLE r, DWORD want) {
    static char buf[4096];
    DWORD total = 0, got = 0;
    while (total < want && ReadFile(r, buf, want - total < sizeof buf ? want - total : sizeof buf, &got, NULL) && got)
        total += got;
    return total;
}

static void capacity(void) {
    HANDLE r, w;
    CreatePipe(&r, &w, NULL, 0);
    fill_end = w;
    HANDLE t = CreateThread(NULL, 0, fill, NULL, 0, NULL);
    Sleep(500);
    LONG before = fill_count;
    DWORD read = drain(r, 65536);
    DWORD done = WaitForSingleObject(t, 10000);
    row("pipe.capacity"); field("written_before_read", (unsigned long long)before); field("read", read);
    field("writer_done", done); end();
    CloseHandle(t); CloseHandle(r); CloseHandle(w);

    CreatePipe(&r, &w, NULL, 0);
    big_end = w;
    t = CreateThread(NULL, 0, big_write, NULL, 0, NULL);
    Sleep(500);
    LONG done_before = big_done;
    DWORD part = drain(r, 1000);
    Sleep(500);
    LONG done_after_part = big_done;
    part += drain(r, 5000);                                  /* 4000 left: less than the 4096 a pipe holds */
    Sleep(500);
    LONG done_after_6000 = big_done;
    DWORD rest = drain(r, sizeof big - part);
    done = WaitForSingleObject(t, 10000);
    row("pipe.large_write"); field("done_before_read", (unsigned long long)done_before);
    field("done_after_1000", (unsigned long long)done_after_part);
    field("done_after_6000", (unsigned long long)done_after_6000); field("read", part + rest);
    field("written", big_written); field("writer_done", done); end();
    CloseHandle(t); CloseHandle(r); CloseHandle(w);
}

/* ---- the parent, as UnityPlayer ----------------------------------------------------------------------- */

static BOOL WINAPI ctrl_handler(DWORD type) { (void)type; return FALSE; }

static void write_record(HANDLE w, DWORD kind, const char *body, DWORD size, const char *name) {
    DWORD header[2] = {kind, size}, a = 0, b = 0;
    SetLastError(SENTINEL);
    BOOL ok1 = WriteFile(w, header, 8, &a, NULL);
    BOOL ok2 = WriteFile(w, body, size, &b, NULL);
    row(name); flag("header", ok1 && a == 8); flag("body", ok2 && b == size); field("err", GetLastError()); end();
}

void start(void) {
    WCHAR *line = GetCommandLineW();
    const WCHAR *p = line;
    while (*p) {                                             /* " child <id>" ends a child's command line */
        if (p[0] == L' ' && p[1] == L'c' && p[2] == L'h' && p[3] == L'i' && p[4] == L'l' && p[5] == L'd' && p[6] == L' ')
            child(parse(p + 7), p + 1);
        if (p[0] == L' ' && p[1] == L'w' && p[2] == L'r' && p[3] == L'i' && p[4] == L't' && p[5] == L'e' && p[6] == L'r' && !p[7])
            writer();
        p++;
    }
    HANDLE out_handle = GetStdHandle(STD_OUTPUT_HANDLE);
    DWORD self = GetCurrentProcessId();
    WCHAR name[64];
    static const WCHAR *const stems[3] = {L"W2mProbe_", L"W2mProbeResp_", L"W2mProbeTerm_"};
    HANDLE events[3];
    for (int i = 0; i < 3; i++) {
        event_name(name, stems[i], self);
        SetLastError(SENTINEL);
        events[i] = CreateEventW(NULL, FALSE, FALSE, name);
        row(i == 0 ? "event.create_0" : i == 1 ? "event.create_1" : "event.create_2");
        flag("non_null", events[i] != NULL); field("err", GetLastError()); end();
    }
    event_name(name, stems[0], self);
    SetLastError(SENTINEL);
    HANDLE again = CreateEventW(NULL, FALSE, FALSE, name);
    row("event.create_again"); flag("non_null", again != NULL); flag("same_handle", again == events[0]);
    field("err", GetLastError()); end();
    SetEvent(again);
    row("event.one_object"); field("wait_first_handle", WaitForSingleObject(events[0], 0)); end();
    CloseHandle(again);

    SECURITY_ATTRIBUTES sa = {sizeof sa, NULL, TRUE};
    HANDLE r = NULL, w = NULL;
    SetLastError(SENTINEL);
    BOOL ok = CreatePipe(&r, &w, &sa, 0);
    row("pipe.create"); field("ret", ok != 0); field("err", GetLastError()); flag("distinct", r != w);
    field("read_type", GetFileType(r)); field("write_type", GetFileType(w)); end();
    DWORD flags = 0xA5;
    GetHandleInformation(w, &flags);
    row("pipe.write_inherit_before"); field("flags", flags); end();
    SetLastError(SENTINEL);
    ok = SetHandleInformation(w, HANDLE_FLAG_INHERIT, 0);
    flags = 0xA5;
    GetHandleInformation(w, &flags);
    row("pipe.write_inherit_cleared"); field("ret", ok != 0); field("err", GetLastError()); field("flags", flags); end();

    WCHAR exe[MAX_PATH];
    DWORD len = GetModuleFileNameW(NULL, exe, MAX_PATH);
    WCHAR cmd[MAX_PATH + 40];
    int k = 0;
    cmd[k++] = L'"';
    for (DWORD i = 0; i < len; i++) cmd[k++] = exe[i];
    cmd[k++] = L'"';
    static const WCHAR tail[] = L" child ";
    for (int i = 0; tail[i]; i++) cmd[k++] = tail[i];
    WCHAR id[12];
    event_name(id, L"", self);
    for (int i = 0; id[i]; i++) cmd[k++] = id[i];
    cmd[k] = 0;
    STARTUPINFOW si;
    for (unsigned i = 0; i < sizeof si; i++) ((char *)&si)[i] = 0;
    si.cb = sizeof si;
    si.dwFlags = STARTF_USESTDHANDLES;
    si.hStdInput = r;
    PROCESS_INFORMATION pi = {0};
    SetLastError(SENTINEL);
    ok = CreateProcessW(NULL, cmd, NULL, NULL, TRUE, NORMAL_PRIORITY_CLASS, NULL, NULL, &si, &pi);
    row("process.create"); field("ret", ok != 0); field("err", GetLastError()); flag("process", pi.hProcess != NULL);
    flag("thread", pi.hThread != NULL); flag("id_differs", pi.dwProcessId != self && pi.dwProcessId != 0);
    flag("thread_id", pi.dwThreadId != 0); end();
    CloseHandle(r);
    DWORD code = 0xA5;
    GetExitCodeProcess(pi.hProcess, &code);
    row("process.running"); field("wait_0", WaitForSingleObject(pi.hProcess, 0)); field("exit_code", code); end();

    write_record(w, 1, "abcdefghijklmnopqrstuvwxyz01234", 31, "pipe.write_0");
    write_record(w, 2, "hello world!", 12, "pipe.write_1");
    write_record(w, 3, "x", 1, "pipe.write_2");
    CloseHandle(w);
    SetLastError(SENTINEL);
    DWORD resp = WaitForSingleObject(events[1], 10000);
    row("process.response"); field("ret", resp); field("err", GetLastError()); end();
    SetEvent(events[2]);
    HANDLE both[2] = {events[0], pi.hProcess};
    DWORD done = WaitForMultipleObjectsEx(2, both, FALSE, 10000, FALSE);
    code = 0xA5;
    GetExitCodeProcess(pi.hProcess, &code);
    row("process.exited"); field("wait", done); field("wait_0", WaitForSingleObject(pi.hProcess, 0)); field("exit_code", code); end();
    CloseHandle(pi.hThread);
    CloseHandle(pi.hProcess);

    SetEnvironmentVariableW(L"W2M_PROBE_VAR", L"from_parent");
    CreateDirectoryW(L"w2m_probe_dir", NULL);
    SetLastError(SENTINEL);
    ok = SetCurrentDirectoryW(L"w2m_probe_dir");
    row("writer.parent_directory"); field("ret", ok != 0); end();
    HANDLE r2 = NULL, w2 = NULL;
    CreatePipe(&r2, &w2, &sa, 0);
    SetHandleInformation(r2, HANDLE_FLAG_INHERIT, 0);
    k = 0;
    cmd[k++] = L'"';
    for (DWORD i = 0; i < len; i++) cmd[k++] = exe[i];
    cmd[k++] = L'"';
    static const WCHAR tail2[] = L" writer";
    for (int i = 0; tail2[i]; i++) cmd[k++] = tail2[i];
    cmd[k] = 0;
    si.hStdInput = NULL;
    si.hStdOutput = w2;
    si.hStdError = NULL;
    PROCESS_INFORMATION pi2 = {0};
    ok = CreateProcessW(NULL, cmd, NULL, NULL, TRUE, NORMAL_PRIORITY_CLASS, NULL, NULL, &si, &pi2);
    CloseHandle(w2);
    static char got_rows[4096];
    DWORD total = 0, piece = 0;
    SetLastError(SENTINEL);
    while (total < sizeof got_rows - 1 && ReadFile(r2, got_rows + total, sizeof got_rows - 1 - total, &piece, NULL) && piece)
        total += piece;
    DWORD read_err = GetLastError();
    DWORD wexit = WaitForSingleObject(pi2.hProcess, 10000);
    code = 0xA5;
    GetExitCodeProcess(pi2.hProcess, &code);
    row("writer.pipe"); field("created", ok != 0); flag("bytes", total > 0); field("read_err", read_err);
    field("wait", wexit); field("exit_code", code); end();
    for (DWORD i = 0; i < total && n < sizeof out - 64; i++) out[n++] = got_rows[i];
    CloseHandle(r2);
    CloseHandle(pi2.hThread);
    CloseHandle(pi2.hProcess);
    SetCurrentDirectoryW(L"..");
    RemoveDirectoryW(L"w2m_probe_dir");
    SetEnvironmentVariableW(L"W2M_PROBE_VAR", NULL);
    capacity();

    SetLastError(SENTINEL);
    ok = SetConsoleCtrlHandler(ctrl_handler, TRUE);
    row("ctrl.add"); field("ret", ok != 0); field("err", GetLastError()); end();
    SetLastError(SENTINEL);
    ok = SetConsoleCtrlHandler(ctrl_handler, FALSE);
    row("ctrl.remove"); field("ret", ok != 0); field("err", GetLastError()); end();
    SetLastError(SENTINEL);
    ok = SetConsoleCtrlHandler(ctrl_handler, FALSE);
    row("ctrl.remove_again"); field("ret", ok != 0); field("err", GetLastError()); end();
    flush_to(out_handle);

    HANDLE f = CreateFileW(L"w2m_child_rows.txt", GENERIC_READ, 0, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    DWORD got = 0;
    if (f != INVALID_HANDLE_VALUE) {
        ReadFile(f, out, sizeof out - 1, &got, NULL);
        CloseHandle(f);
        DeleteFileW(L"w2m_child_rows.txt");
    }
    n = got;
    flush_to(out_handle);
    ExitProcess(0);
}
