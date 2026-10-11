// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* temp_path.c — GetTempPathW and the environment variables it reads (Sprint 048, family "temp path").
 *
 * No C runtime. UnityPlayer.dll calls GetTempPathW(260, buffer) after SHGetKnownFolderPath (frontier 2026-10-10,
 * 2,641,240 instructions); it and mono-2.0-bdwgc.dll import GetEnvironmentVariableW, SetEnvironmentVariableW and
 * GetEnvironmentStringsW. Microsoft documents GetTempPathW's order: TMP, TEMP, USERPROFILE, the Windows directory.
 * A runner's environment is not w2m's, so the probe first deletes every variable its block holds (the names that
 * failed are printed), then sets the variables each row needs. Paths under the working directory print as
 * "<cwd>". Rows: each call's return, last error (after SetLastError(0xDEAD)) and text; a buffer too small is
 * filled with 0xA5A5 first, and the row says whether it was left so.
 */
#include <windows.h>

#define SENTINEL 0xDEADu
#define MARK 0xA5A5u

static char out[16384];
static unsigned n;
static WCHAR cwd[MAX_PATH];
static unsigned cwd_len;
static WCHAR buf[1024];

static void put(const char *s) { while (*s) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
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
static void text(const char *k, const WCHAR *s) {
    put(",\""); put(k); put("\":\"");
    if (starts(s, cwd, cwd_len) && (s[cwd_len] == 0 || s[cwd_len] == '\\')) { put("<cwd>"); raw(s + cwd_len); }
    else raw(s);
    put("\"");
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void end(void) { put("}\n"); }
static void fill(void) { for (unsigned i = 0; i < 1024; i++) buf[i] = MARK; }

static void set(const WCHAR *name, const WCHAR *value) { SetEnvironmentVariableW(name, value); }

/* GetTempPathW(size, buf or NULL) */
static void temp(const char *name, DWORD size, int null_buffer) {
    fill();
    SetLastError(SENTINEL);
    DWORD r = GetTempPathW(size, null_buffer ? NULL : buf);
    DWORD err = GetLastError();
    row(name); field("ret", r); field("err", err);
    if (r && r < size) text("path", buf);
    else if (!null_buffer) field("untouched", buf[0] == MARK);
    end();
}

/* GetEnvironmentVariableW(name, buf or NULL, size) */
static void get(const char *name, const WCHAR *var, DWORD size, int null_buffer) {
    fill();
    SetLastError(SENTINEL);
    DWORD r = GetEnvironmentVariableW(var, null_buffer ? NULL : buf, size);
    DWORD err = GetLastError();
    row(name); field("ret", r); field("err", err);
    if (r < size && !null_buffer && buf[0] != MARK) text("value", buf);
    else if (!null_buffer) field("untouched", buf[0] == MARK);
    end();
}

static void put_set(const char *name, const WCHAR *var, const WCHAR *value) {
    SetLastError(SENTINEL);
    BOOL r = SetEnvironmentVariableW(var, value);
    DWORD err = GetLastError();
    row(name); field("ret", r != 0); field("err", err); end();
}

/* The block's entries in order. */
static void block(const char *name) {
    WCHAR *b = GetEnvironmentStringsW();
    row(name); put(",\"entries\":[");
    for (WCHAR *e = b; *e; ) {
        if (e != b) put(",");
        put("\""); raw(e); put("\"");
        while (*e) e++;
        e++;
    }
    put("]"); end();
    FreeEnvironmentStringsW(b);
}

void start(void) {
    cwd_len = GetCurrentDirectoryW(MAX_PATH, cwd);

    /* Delete every variable: names run to the first '=' after the first character ("=C:=C:\x" is "=C:"). */
    WCHAR *b = GetEnvironmentStringsW();
    static WCHAR names[64][128];
    unsigned count = 0;
    for (WCHAR *e = b; *e && count < 64; ) {
        unsigned k = 0;
        while (e[k] && (k == 0 || e[k] != '=') && k < 127) { names[count][k] = e[k]; k++; }
        names[count][k] = 0;
        count++;
        while (*e) e++;
        e++;
    }
    FreeEnvironmentStringsW(b);
    row("clear"); put(",\"failed\":[");
    unsigned failed = 0;
    for (unsigned i = 0; i < count; i++)
        if (!SetEnvironmentVariableW(names[i], NULL)) { if (failed++) put(","); put("\""); raw(names[i]); put("\""); }
    put("]"); end();
    block("cleared");

    temp("temp.none", MAX_PATH + 1, 0);
    set(L"USERPROFILE", L"C:\\u");
    temp("temp.profile", MAX_PATH + 1, 0);
    set(L"TEMP", L"C:\\t");
    temp("temp.temp", MAX_PATH + 1, 0);
    set(L"TMP", L"C:\\m");
    temp("temp.tmp", MAX_PATH + 1, 0);
    temp("temp.size_exact", 6, 0);            /* "C:\m\" and its NUL */
    temp("temp.size_short", 5, 0);
    temp("temp.size_zero", 0, 1);
    set(L"TMP", L"C:\\m\\");
    temp("temp.tmp_backslash", MAX_PATH + 1, 0);
    set(L"TMP", L"rel");
    temp("temp.relative", MAX_PATH + 1, 0);
    set(L"TMP", L"C:/m/x");
    temp("temp.forward", MAX_PATH + 1, 0);
    set(L"TMP", L"C:\\m\\..\\n");
    temp("temp.dots", MAX_PATH + 1, 0);
    set(L"TMP", L"C:\\m\\nonexistent\\deeper");
    temp("temp.missing_dir", MAX_PATH + 1, 0);
    set(L"TMP", L"");
    temp("temp.tmp_empty", MAX_PATH + 1, 0);
    static WCHAR longer[400] = L"C:\\";
    for (unsigned i = 3; i < 303; i++) longer[i] = 'a';
    set(L"TMP", longer);
    temp("temp.long", MAX_PATH + 1, 0);
    set(L"TMP", NULL);
    temp("temp.after_delete", MAX_PATH + 1, 0);

    get("get.present", L"TEMP", 64, 0);
    get("get.case", L"temp", 64, 0);
    get("get.missing", L"W2M_NO_SUCH", 64, 0);
    get("get.size_exact", L"TEMP", 5, 0);     /* "C:\t" and its NUL */
    get("get.size_short", L"TEMP", 4, 0);
    get("get.size_zero", L"TEMP", 0, 1);
    set(L"EMPTY", L"");
    get("get.empty_value", L"EMPTY", 64, 0);

    put_set("set.delete_missing", L"W2M_NO_SUCH", NULL);
    put_set("set.equals_in_name", L"A=B", L"x");
    put_set("set.empty_name", L"", L"x");
    put_set("set.replace", L"TEMP", L"C:\\t2");
    get("get.replaced", L"TEMP", 64, 0);
    put_set("set.case_replace", L"temp", L"C:\\t3");
    block("order.before");
    put_set("set.new_lower", L"alpha", L"1");
    put_set("set.new_upper", L"ZED", L"2");
    put_set("set.new_drive", L"=C:", L"C:\\w");
    put_set("set.new_mixed", L"Mid", L"3");
    block("order.after");

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
