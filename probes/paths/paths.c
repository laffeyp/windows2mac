// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* paths.c — the working directory, full path names, and directories (Sprint 048, family "paths").
 *
 * No C runtime. UnityPlayer.dll imports GetFullPathNameW, GetCurrentDirectoryW, CreateDirectoryW and
 * RemoveDirectoryW and names its files by absolute path ("C:\w2m\app\Data\boot.config", look-ahead 2026-10-10).
 * A runner's working directory is not w2m's, so every path printed has the working directory's prefix replaced
 * by "<cwd>" and the drive's root ("D:\") by "<root>", and the drive letter by "<d>": the rows hold on any machine.
 * Rows: each call's return, last error (after SetLastError(0xDEAD)) and text, and GetFullPathNameW's file part as
 * its distance back from the end of the result (minus the file part's length), or null.
 */
#include <windows.h>

#define SENTINEL 0xDEADu

static char out[32768];
static unsigned n;
static WCHAR cwd[MAX_PATH], root[4] = L"?:\\";
static unsigned cwd_len, parent_len;            /* parent: cwd up to its last backslash */

static void put(const char *s) { while (*s) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void puts_(long long v) { if (v < 0) { out[n++] = '-'; v = -v; } putu((unsigned long long)v); }
static void raw(const WCHAR *s) {
    for (; *s; s++) {
        WCHAR c = *s;
        if (c == '"' || c == '\\') { out[n++] = '\\'; out[n++] = (char)c; }
        else if (c >= 0x20 && c < 0x7F) out[n++] = (char)c;
        else put("?");
    }
}
static int starts(const WCHAR *s, const WCHAR *p, unsigned len) {
    for (unsigned i = 0; i < len; i++) {
        WCHAR a = s[i], b = p[i];
        if (a >= 'a' && a <= 'z') a -= 32;
        if (b >= 'a' && b <= 'z') b -= 32;
        if (a != b) return 0;
    }
    return 1;
}
/* A path as a JSON string with the machine's prefixes replaced. */
static void path(const WCHAR *s) {
    out[n++] = '"';
    if (starts(s, cwd, cwd_len) && (s[cwd_len] == 0 || s[cwd_len] == '\\')) { put("<cwd>"); raw(s + cwd_len); }
    else if (parent_len > 3 && starts(s, cwd, parent_len) && (s[parent_len] == 0 || s[parent_len] == '\\')) { put("<parent>"); raw(s + parent_len); }
    else if (starts(s, L"\\\\?\\", 4) && starts(s + 4, cwd, cwd_len)) { put("\\\\\\\\?\\\\<cwd>"); raw(s + 4 + cwd_len); }
    else if (starts(s, root, 3)) { put("<root>"); raw(s + 3); }
    else if (starts(s, root, 2)) { put("<d>:"); raw(s + 2); }
    else raw(s);
    out[n++] = '"';
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void end(void) { put("}\n"); }

static void full(const char *name, const WCHAR *in, DWORD size) {
    static WCHAR buf[1024];
    for (unsigned i = 0; i < 1024; i++) buf[i] = 0xA5A5;
    LPWSTR part = (LPWSTR)1;
    SetLastError(SENTINEL);
    DWORD r = GetFullPathNameW(in, size, buf, &part);
    DWORD err = GetLastError();
    unsigned len = 0;
    while (r && r < size && buf[len]) len++;
    row(name); put(",\"in\":"); path(in); field("ret_zero", r == 0); field("err", err);
    if (r && r < size) field("ret_is_len", r == len);
    if (r >= size) {                                  /* the length needed, NUL included */
        static WCHAR big[1024];
        DWORD whole = GetFullPathNameW(in, 1024, big, NULL);
        field("ret_is_need", r == whole + 1);
    }
    if (r && r < size) { put(",\"out\":"); path(buf); put(",\"part_from_end\":"); if (part) puts_((long long)(part - buf) - (long long)r); else put("null"); }
    else { put(",\"buffer_untouched\":"); put(buf[0] == 0xA5A5 ? "true" : "false"); }
    end();
}

static void dir(const char *name, const char *call, const WCHAR *p, BOOL (*f)(const WCHAR *)) {
    SetLastError(SENTINEL);
    BOOL ok = f(p);
    DWORD err = GetLastError();
    row(name); put(",\"call\":\""); put(call); put("\""); field("ret", ok != 0); field("err", err); end();
}
static BOOL make(const WCHAR *p) { return CreateDirectoryW(p, NULL); }
static BOOL unmake(const WCHAR *p) { return RemoveDirectoryW(p); }
static BOOL chdir_(const WCHAR *p) { return SetCurrentDirectoryW(p); }

void start(void) {
    cwd_len = GetCurrentDirectoryW(MAX_PATH, cwd);
    root[0] = cwd[0];
    for (unsigned i = 0; i < cwd_len; i++) if (cwd[i] == '\\') parent_len = i;
    row("cwd"); field("len_is_text", cwd_len > 3 && cwd[cwd_len] == 0); field("drive_form", cwd[1] == ':' && cwd[2] == '\\');
    field("trailing_backslash", cwd[cwd_len - 1] == '\\'); end();
    static WCHAR tiny[4];
    SetLastError(SENTINEL);
    DWORD need = GetCurrentDirectoryW(4, tiny);
    DWORD err = GetLastError();
    row("cwd.small_buffer"); field("ret_is_len_plus_1", need == cwd_len + 1); field("err", err); end();
    SetLastError(SENTINEL);
    need = GetCurrentDirectoryW(0, NULL);
    err = GetLastError();
    row("cwd.null_buffer"); field("ret_is_len_plus_1", need == cwd_len + 1); field("err", err); end();

    static const struct { const char *name; const WCHAR *in; } NAMES[] = {
        {"relative", L"a.txt"}, {"dot_relative", L".\\a.txt"}, {"dotdot", L"..\\a.txt"}, {"sub_dotdot", L"x\\..\\a.txt"},
        {"slashes", L"x/y/a.txt"}, {"doubled", L"x\\\\y\\a.txt"}, {"trailing_dot", L"a.txt."}, {"trailing_dots_spaces", L"a.txt. . "},
        {"inner_trailing_dot", L"x.\\a.txt"}, {"trailing_backslash", L"x\\"}, {"dot", L"."}, {"dotdot_alone", L".."},
        {"rooted", L"\\a.txt"}, {"rooted_dotdot", L"\\..\\..\\a.txt"}, {"drive_relative", L"?:a.txt"},
        {"absolute", L"?:\\w2m\\app\\Data\\boot.config"}, {"absolute_dotdot", L"?:\\w2m\\app\\..\\x"},
        {"absolute_lower", L"?:\\W2M\\app"}, {"other_drive", L"Q:\\x"}, {"other_drive_relative", L"Q:x"},
        {"device_prefix", L"\\\\?\\?:\\x\\..\\y"}, {"unc", L"\\\\server\\share\\x\\..\\y"}, {"empty", L""},
        {"spaces_only", L"   "}, {"wildcard", L"x\\*.txt"}, {"device_name", L"x\\CON"},
    };
    static WCHAR in[64];
    for (unsigned i = 0; i < sizeof NAMES / sizeof NAMES[0]; i++) {
        unsigned k = 0;
        for (const WCHAR *s = NAMES[i].in; *s; s++, k++) {      /* '?' is the drive letter, but not \\?\'s */
            int prefix = k == 2 && NAMES[i].in[0] == '\\' && NAMES[i].in[1] == '\\';
            in[k] = *s == '?' && !prefix ? root[0] : *s;
        }
        in[k] = 0;
        static char name[48];
        unsigned j = 0;
        for (const char *s = "full."; *s;) name[j++] = *s++;
        for (const char *s = NAMES[i].name; *s;) name[j++] = *s++;
        name[j] = 0;
        full(name, in, 1024);
    }
    full("full.small_buffer", L"a.txt", 4);
    full("full.zero_buffer", L"a.txt", 0);

    dir("dir.create", "CreateDirectoryW", L"w2m_paths_dir", make);
    dir("dir.create_again", "CreateDirectoryW", L"w2m_paths_dir", make);
    dir("dir.create_missing_parent", "CreateDirectoryW", L"w2m_missing\\x", make);
    dir("dir.create_trailing_backslash", "CreateDirectoryW", L"w2m_paths_dir2\\", make);
    dir("dir.chdir", "SetCurrentDirectoryW", L"w2m_paths_dir", chdir_);
    full("full.after_chdir", L"a.txt", 1024);
    {
        static WCHAR now[MAX_PATH];
        DWORD len = GetCurrentDirectoryW(MAX_PATH, now);
        row("cwd.after_chdir"); put(",\"text\":"); path(now); field("len_grew_by", len - cwd_len); end();
    }
    dir("dir.chdir_back", "SetCurrentDirectoryW", L"..", chdir_);
    dir("dir.chdir_missing", "SetCurrentDirectoryW", L"w2m_missing", chdir_);
    HANDLE h = CreateFileW(L"w2m_paths_dir\\f.txt", GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    CloseHandle(h);
    dir("dir.chdir_file", "SetCurrentDirectoryW", L"w2m_paths_dir\\f.txt", chdir_);
    dir("dir.remove_not_empty", "RemoveDirectoryW", L"w2m_paths_dir", unmake);
    dir("dir.remove_file", "RemoveDirectoryW", L"w2m_paths_dir\\f.txt", unmake);
    DeleteFileW(L"w2m_paths_dir\\f.txt");
    dir("dir.remove", "RemoveDirectoryW", L"w2m_paths_dir", unmake);
    dir("dir.remove_again", "RemoveDirectoryW", L"w2m_paths_dir", unmake);
    dir("dir.remove_trailing_backslash", "RemoveDirectoryW", L"w2m_paths_dir2\\", unmake);
    dir("dir.remove_cwd", "RemoveDirectoryW", L".", unmake);

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
