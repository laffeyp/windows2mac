// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* files_more.c — GetFileInformationByHandleEx, GetFileSize, SetHandleInformation, GetWindowsDirectoryW and
 * GetTempFileNameW (Sprint 049, family "files more"; specs/work_list.json).
 *
 * No C runtime. The capture (store/game-runs/38113342549, 38113343972) holds the game's forms: mono opens a file
 * GENERIC_READ, share read|write, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL and asks GetFileInformationByHandleEx for
 * FileBasicInfo (0x28 bytes) then FileStandardInfo (0x18); mono's GetFileSize(h, &high); UnityPlayer's
 * SetHandleInformation(h, HANDLE_FLAG_INHERIT, 0), GetWindowsDirectoryW(buf, 260) and GetTempFileNameW(dir, prefix,
 * 0, buf). Rows hold each call's return and last error (after SetLastError(0xDEAD)); times print as relations to
 * GetFileAttributesExW's and to each other, never as values; every output buffer is filled with 0xA5 first, so the
 * rows show which bytes a call writes. Names under the working directory print as "<cwd>".
 */
#include <windows.h>

#define SENTINEL 0xDEADu
#define MARK 0xA5u

static char out[65536];
static unsigned n;
static WCHAR cwd[MAX_PATH];
static unsigned cwd_len;
static unsigned char buf[256];
static WCHAR wbuf[600];

static void put(const char *s) { while (*s && n < sizeof out - 64) out[n++] = *s++; }
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
static int same(WCHAR a, WCHAR b) {
    if (a >= 'a' && a <= 'z') a -= 32;
    if (b >= 'a' && b <= 'z') b -= 32;
    return a == b;
}
static void text(const char *k, const WCHAR *s) {
    put(",\""); put(k); put("\":\"");
    unsigned i = 0;
    while (i < cwd_len && same(s[i], cwd[i])) i++;
    if (i == cwd_len && (s[i] == 0 || s[i] == '\\')) { put("<cwd>"); raw(s + cwd_len); }
    else raw(s);
    put("\"");
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
/* Bytes [from, to) of buf as hex: what a call wrote, or left as 0xA5. */
static void bytes(const char *k, unsigned from, unsigned to) {
    put(",\""); put(k); put("\":\"");
    for (unsigned i = from; i < to; i++) {
        out[n++] = "0123456789abcdef"[buf[i] >> 4];
        out[n++] = "0123456789abcdef"[buf[i] & 15];
    }
    put("\"");
}
/* Each row is written when it ends, so a fault leaves the rows before it. */
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}
static void fill(void) {
    for (unsigned i = 0; i < sizeof buf; i++) buf[i] = MARK;
    for (unsigned i = 0; i < 600; i++) wbuf[i] = 0xA5A5;
}
static unsigned long long q(unsigned off) { return *(unsigned long long *)(buf + off); }
static unsigned long long ft(const FILETIME *t) { return (unsigned long long)t->dwHighDateTime << 32 | t->dwLowDateTime; }

static HANDLE make(const WCHAR *name, DWORD count) {
    HANDLE h = CreateFileW(name, GENERIC_READ | GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    static char data[8192];
    for (unsigned i = 0; i < sizeof data; i++) data[i] = (char)('a' + i % 26);
    DWORD done;
    if (count) WriteFile(h, data, count, &done, NULL);
    return h;
}
/* The game's form of open. */
static HANDLE open_read(const WCHAR *name) {
    return CreateFileW(name, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, NULL, OPEN_EXISTING,
                       FILE_ATTRIBUTE_NORMAL, NULL);
}

/* GetFileInformationByHandleEx(h, FileBasicInfo, buf, size): FILE_BASIC_INFO is four LARGE_INTEGER times
 * (creation, last access, last write, change) and FileAttributes at 0x20. */
static void basic(const char *name, HANDLE h, DWORD size, const WCHAR *file) {
    fill();
    SetLastError(SENTINEL);
    BOOL r = GetFileInformationByHandleEx(h, FileBasicInfo, buf, size);
    DWORD err = GetLastError();
    row(name); field("ret", r != 0); field("err", err);
    if (r) {
        WIN32_FILE_ATTRIBUTE_DATA a;
        GetFileAttributesExW(file, GetFileExInfoStandard, &a);
        field("attributes", *(DWORD *)(buf + 0x20));
        field("creation_as_attributes_ex", q(0) == ft(&a.ftCreationTime));
        field("access_as_attributes_ex", q(8) == ft(&a.ftLastAccessTime));
        field("write_as_attributes_ex", q(16) == ft(&a.ftLastWriteTime));
        field("change_ge_write", q(24) >= q(16));
        field("write_ge_creation", q(16) >= q(0));
        field("times_nonzero", q(0) && q(8) && q(16) && q(24));
    }
    bytes("tail", 0x20, size + 8 < sizeof buf ? size + 8 : sizeof buf);
    end();
}

/* FileStandardInfo: AllocationSize, EndOfFile (LARGE_INTEGER), NumberOfLinks (DWORD at 0x10), DeletePending and
 * Directory (BOOLEAN at 0x14, 0x15). */
static void standard(const char *name, HANDLE h, DWORD size) {
    fill();
    SetLastError(SENTINEL);
    BOOL r = GetFileInformationByHandleEx(h, FileStandardInfo, buf, size);
    DWORD err = GetLastError();
    row(name); field("ret", r != 0); field("err", err);
    if (r) {
        field("allocation", q(0)); field("end_of_file", q(8)); field("links", *(DWORD *)(buf + 0x10));
        field("delete_pending", buf[0x14]); field("directory", buf[0x15]);
    }
    bytes("tail", 0x14, size + 8 < sizeof buf ? size + 8 : sizeof buf);
    end();
}

static void size(const char *name, HANDLE h, int with_high) {
    DWORD high = 0xA5A5A5A5u;
    SetLastError(SENTINEL);
    DWORD r = GetFileSize(h, with_high ? &high : NULL);
    DWORD err = GetLastError();
    row(name); field("ret", r); field("err", err);
    if (with_high) field("high", high);
    end();
}

static void set_info(const char *name, HANDLE h, DWORD mask, DWORD flags) {
    SetLastError(SENTINEL);
    BOOL r = SetHandleInformation(h, mask, flags);
    DWORD err = GetLastError();
    DWORD now = 0xA5A5A5A5u;
    BOOL g = GetHandleInformation(h, &now);
    row(name); field("ret", r != 0); field("err", err); field("get_ret", g != 0); field("flags", now); end();
}

static void windows(const char *name, UINT size, int null_buffer) {
    fill();
    SetLastError(SENTINEL);
    UINT r = GetWindowsDirectoryW(null_buffer ? NULL : wbuf, size);
    DWORD err = GetLastError();
    row(name); field("ret", r); field("err", err);
    if (!null_buffer && r < size && wbuf[r] == 0) text("path", wbuf);
    else if (!null_buffer) field("first_unit", wbuf[0]);
    end();
}

/* GetTempFileNameW(dir, prefix, unique, buf): the name prints with the return's four hex digits as "<u>" when the
 * caller passed 0 (the number comes from the clock); the file's existence and size after the call. */
static void temp_name(const char *name, const WCHAR *dir, const WCHAR *prefix, UINT unique) {
    fill();
    SetLastError(SENTINEL);
    UINT r = GetTempFileNameW(dir, prefix, unique, wbuf);
    DWORD err = GetLastError();
    row(name); field("ret_nonzero", r != 0); field("err", err);
    if (unique) field("ret", r);
    if (r) {
        static WCHAR shown[600];
        static const WCHAR hex[] = L"0123456789ABCDEF";
        unsigned k = 0, j = 0;
        while (wbuf[k]) k++;
        /* the characters before ".tmp" are the number's hex digits, as many as it has (run 38119521556: 0xABC
         * prints as three) */
        unsigned digits = 1;
        while (digits < 8 && r >> (4 * digits)) digits++;
        for (unsigned i = 0; i < k; i++) {
            int digit = !unique && k >= digits + 4 && i >= k - digits - 4 && i < k - 4;
            if (digit && i == k - digits - 4) {
                int match = 1;
                for (unsigned d = 0; d < digits; d++)
                    if (!same(wbuf[i + d], hex[(r >> (4 * (digits - 1 - d))) & 15])) match = 0;
                const WCHAR *u = match ? L"<u>" : L"<?>";
                while (*u) shown[j++] = *u++;
            }
            if (!digit) shown[j++] = wbuf[i];
        }
        shown[j] = 0;
        text("name", shown);
        WIN32_FILE_ATTRIBUTE_DATA a;
        BOOL there = GetFileAttributesExW(wbuf, GetFileExInfoStandard, &a);
        field("exists", there != 0);
        if (there) { field("size", a.nFileSizeLow); field("attributes", a.dwFileAttributes); DeleteFileW(wbuf); }
    }
    end();
}

void start(void) {
    cwd_len = GetCurrentDirectoryW(MAX_PATH, cwd);

    HANDLE w = make(L"fm_five.dat", 5);
    CloseHandle(w);
    w = make(L"fm_empty.dat", 0);
    CloseHandle(w);

    HANDLE h = open_read(L"fm_five.dat");
    basic("basic.read", h, 0x28, L"fm_five.dat");
    basic("basic.short", h, 0x27, L"fm_five.dat");
    basic("basic.exact_less_pad", h, 0x24, L"fm_five.dat");
    basic("basic.long", h, 0x30, L"fm_five.dat");
    standard("standard.read", h, 0x18);
    standard("standard.short", h, 0x17);
    standard("standard.exact_less_pad", h, 0x16);
    standard("standard.long", h, 0x20);
    size("size.five", h, 1);
    size("size.null_high", h, 0);
    set_info("set.inherit", h, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT);
    set_info("set.clear", h, HANDLE_FLAG_INHERIT, 0);
    set_info("set.flags_outside_mask", h, 0, HANDLE_FLAG_INHERIT);
    set_info("set.protect", h, HANDLE_FLAG_PROTECT_FROM_CLOSE, HANDLE_FLAG_PROTECT_FROM_CLOSE);
    set_info("set.unprotect", h, HANDLE_FLAG_PROTECT_FROM_CLOSE, 0);
    set_info("set.both", h, 3, 3);
    set_info("set.none", h, 3, 0);
    set_info("set.unknown_bit", h, 4, 4);
    CloseHandle(h);
    basic("basic.closed", h, 0x28, L"fm_five.dat");
    standard("standard.closed", h, 0x18);
    size("size.closed", h, 1);
    set_info("set.closed", h, HANDLE_FLAG_INHERIT, 0);

    h = open_read(L"fm_empty.dat");
    basic("basic.empty", h, 0x28, L"fm_empty.dat");
    standard("standard.empty", h, 0x18);
    size("size.empty", h, 1);
    CloseHandle(h);

    h = CreateFileW(L"fm_five.dat", 0, FILE_SHARE_READ | FILE_SHARE_WRITE, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL,
                    NULL);
    basic("basic.no_access", h, 0x28, L"fm_five.dat");
    standard("standard.no_access", h, 0x18);
    size("size.no_access", h, 1);
    CloseHandle(h);

    h = make(L"fm_grow.dat", 5000);                    /* written through this handle, not closed */
    standard("standard.after_write", h, 0x18);
    size("size.after_write", h, 1);
    CloseHandle(h);

    /* AllocationSize by size: NTFS keeps small data in the file record (8-byte units), larger data in clusters */
    static const DWORD sizes[] = {1, 8, 9, 256, 512, 600, 650, 700, 720, 740, 760, 800, 900, 1000, 1024, 2048, 4095,
                                  4096, 4097, 8192};
    for (unsigned i = 0; i < sizeof sizes / sizeof *sizes; i++) {
        h = make(L"fm_sweep.dat", sizes[i]);
        CloseHandle(h);
        h = open_read(L"fm_sweep.dat");
        fill();
        BOOL r = GetFileInformationByHandleEx(h, FileStandardInfo, buf, 0x18);
        row("standard.sweep"); field("size", sizes[i]); field("ret", r != 0); field("allocation", q(0)); end();
        CloseHandle(h);
    }
    /* fm_sweep.dat now holds 8192 bytes in clusters: cut to 0 by CREATE_ALWAYS, then by TRUNCATE_EXISTING, each time
     * rewritten to 5 bytes; and 5 more bytes appended through a second handle */
    h = make(L"fm_sweep.dat", 5);
    CloseHandle(h);
    h = open_read(L"fm_sweep.dat");
    standard("standard.recreated", h, 0x18);
    CloseHandle(h);
    h = make(L"fm_sweep.dat", 8192);
    CloseHandle(h);
    h = CreateFileW(L"fm_sweep.dat", GENERIC_WRITE, 0, NULL, TRUNCATE_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    DWORD done;
    WriteFile(h, "abcde", 5, &done, NULL);
    CloseHandle(h);
    h = open_read(L"fm_sweep.dat");
    standard("standard.truncated", h, 0x18);
    CloseHandle(h);
    h = CreateFileW(L"fm_sweep.dat", FILE_APPEND_DATA, 0, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    WriteFile(h, "abcde", 5, &done, NULL);
    CloseHandle(h);
    h = open_read(L"fm_sweep.dat");
    standard("standard.appended", h, 0x18);
    CloseHandle(h);
    DeleteFileW(L"fm_sweep.dat");

    windows("windows.full", MAX_PATH, 0);
    windows("windows.exact", 11, 0);                   /* "C:\Windows" and its NUL */
    windows("windows.short", 10, 0);
    windows("windows.zero_null", 0, 1);

    temp_name("temp_name.zero", L".", L"w2m", 0);
    temp_name("temp_name.cwd_full", cwd, L"w2m", 0);
    temp_name("temp_name.unique", L".", L"w2m", 0x12AB);
    temp_name("temp_name.unique_high", L".", L"w2m", 0x12345);
    temp_name("temp_name.long_prefix", L".", L"abcdef", 0x1);
    temp_name("temp_name.empty_prefix", L".", L"", 0x2);
    temp_name("temp_name.trailing_backslash", L".\\", L"w2m", 0x3);
    temp_name("temp_name.missing_dir", L"no_such_dir", L"w2m", 0);
    temp_name("temp_name.missing_dir_unique", L"no_such_dir", L"w2m", 0x4);
    temp_name("temp_name.file_as_dir", L"fm_five.dat", L"w2m", 0);
    WCHAR first[600];                                  /* unique 0 again while the first call's file exists */
    UINT a = GetTempFileNameW(L".", L"w2m", 0, first);
    temp_name("temp_name.second", L".", L"w2m", 0);
    if (a) DeleteFileW(first);

    DeleteFileW(L"fm_five.dat");
    DeleteFileW(L"fm_empty.dat");
    DeleteFileW(L"fm_grow.dat");
    ExitProcess(0);
}
