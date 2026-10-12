// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* paths.c — the working directory, full path names and directories (Sprint 048, family "paths").
 *
 * GetFullPathNameW, GetCurrentDirectoryW, SetCurrentDirectoryW, CreateDirectoryW and RemoveDirectoryW, as
 * contracts/windows.file.paths records Windows answering (Server 2025 and 2022; identical but for "x\CON", a
 * device on 2022 and a name on 2025, which w2m follows):
 *   - a name is made full against the working directory: relative names follow it, "\name" its drive's root,
 *     "X:name" it on its own drive and "X:\name" on another; "\\?\" keeps its prefix and the rest is still
 *     collapsed; "\\server\share" is a UNC root
 *   - every component loses trailing dots and spaces; "." goes; ".." takes the last component, never the root;
 *     "/" is "\"; doubled separators are one; a trailing separator stays; case and wildcards stay
 *   - the empty name or blanks only: 0, ERROR_INVALID_NAME (123), the buffer untouched; a short buffer gets the
 *     length needed, NUL included; the file part is the last component, or NULL after a trailing separator
 *   - GetCurrentDirectoryW: the length, or the length needed with the NUL; success leaves the last error
 *   - CreateDirectoryW: 183 when it exists, 3 when a parent is missing; RemoveDirectoryW: 145 not empty, 267 a
 *     file, 2 missing, 32 the working directory; SetCurrentDirectoryW: 2 missing, 267 a file
 * The working directory starts at the program's directory, C:\w2m\app, as Steam starts a game.
 */
#include "runtime.h"

typedef unsigned short u16;

#define MAX_CHARS 1024
#define ERROR_FILE_NOT_FOUND 2u
#define ERROR_PATH_NOT_FOUND 3u
#define ERROR_SHARING_VIOLATION 32u
#define ERROR_INVALID_NAME 123u
#define ERROR_DIR_NOT_EMPTY 145u
#define ERROR_ALREADY_EXISTS 183u
#define ERROR_DIRECTORY 267u
#define EEXIST 17
#define ENOENT 2
#define ENOTEMPTY 66                                 /* macOS */

static u16 cwd[MAX_CHARS];                           /* the working directory without its final '\\' */
static u16 scratch[MAX_CHARS];

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }
/* The working directory: ProcessParameters.CurrentDirectory.DosPath (Sprint 059), which holds it with a final '\\'
 * (rows *.current_directory); the host wrote the start's (a child's: its parent's, Sprint 058). */
static const u16 *working(void) {
    const u8 *p = w2m_parameters();
    const u16 *dos = *(const u16 *const *)(p + 0x40);
    u64 n = *(const u16 *)(p + 0x38) / 2;
    if (n > 3 && dos[n - 1] == '\\') n--;
    for (u64 k = 0; k < n; k++) cwd[k] = dos[k];
    cwd[n] = 0;
    return cwd;
}
static int sep(u16 c) { return c == '\\' || c == '/'; }
static u16 upper(u16 c) { return c >= 'a' && c <= 'z' ? (u16)(c - 32) : c; }
static u64 length(const u16 *s) { u64 n = 0; while (s[n]) n++; return n; }

/* `in` made full into out (MAX_CHARS); returns its length, 0 for a name that is empty or blanks only. *part: the
 * index of the last component, or -1 after a trailing separator. */
u64 w2m_full_path(const u16 *in, u16 *out, long *part) {
    u64 n = 0, i = 0, root = 0;
    int blanks = 1;
    for (u64 k = 0; in[k]; k++) blanks &= in[k] == ' ';
    if (blanks) return 0;
    const u16 *base = 0;                                /* the components to start from (the working directory) */
    if (in[0] == '\\' && in[1] == '\\' && in[2] == '?' && in[3] == '\\') {      /* "\\?\": kept, the rest collapsed */
        for (; i < 4; i++) out[n++] = in[i];
        if (in[4] && in[5] == ':') { out[n++] = in[4]; out[n++] = ':'; out[n++] = '\\'; i = sep(in[6]) ? 7 : 6; }
    } else if (sep(in[0]) && sep(in[1])) {             /* UNC: \\server\share is the root */
        out[n++] = '\\'; out[n++] = '\\';
        i = 2;
        for (int c = 0; c < 2 && in[i]; c++) {
            while (in[i] && !sep(in[i])) out[n++] = in[i++];
            if (c == 0) { out[n++] = '\\'; if (in[i]) i++; }
        }
        out[n++] = '\\';
        if (in[i]) i++;
    } else if (in[0] && in[1] == ':') {
        if (sep(in[2])) { out[n++] = in[0]; out[n++] = ':'; out[n++] = '\\'; i = 3; }
        else if (upper(in[0]) == upper(working()[0])) { base = working(); i = 2; }
        else { out[n++] = in[0]; out[n++] = ':'; out[n++] = '\\'; i = 2; }
    } else if (sep(in[0])) {
        out[n++] = working()[0]; out[n++] = ':'; out[n++] = '\\'; i = 1;
    } else {
        base = working();
    }
    if (base) {                                         /* the working directory: its root, then its components */
        out[n++] = base[0]; out[n++] = ':'; out[n++] = '\\';
        root = n;
        for (u64 k = 3; base[k]; k++) out[n++] = base[k];
        if (n > root) out[n++] = '\\';
    } else {
        root = n;
    }
    /* components; out holds root, then "component\" for each */
    while (in[i]) {
        if (sep(in[i])) { i++; continue; }
        u64 start = i;
        while (in[i] && !sep(in[i])) i++;
        u64 len = i - start;
        if (len == 1 && in[start] == '.') continue;
        if (len == 2 && in[start] == '.' && in[start + 1] == '.') {
            if (n > root) {                             /* drop the last "component\" */
                n--;
                while (n > root && out[n - 1] != '\\') n--;
            }
            continue;
        }
        while (len && (in[start + len - 1] == '.' || in[start + len - 1] == ' ')) len--;
        if (!len) continue;
        if (n + len + 2 >= MAX_CHARS) w2m_unimplemented(W2M_API_GetFullPathNameW, n + len);
        for (u64 k = 0; k < len; k++) out[n++] = in[start + k];
        out[n++] = '\\';
    }
    u64 l = 0;
    while (in[l]) l++;
    int trailing = l && sep(in[l - 1]);
    if (n > root && !trailing) n--;                    /* the last separator goes, unless the name ended with one */
    out[n] = 0;
    long last = -1;
    if (!(n && out[n - 1] == '\\')) {
        last = (long)n;
        while (last > 0 && out[last - 1] != '\\') last--;
    }
    *part = last;
    return n;
}

u64 k32_GetFullPathNameW(u64 name, u64 size, u64 buffer, u64 part) {
    if (!name) w2m_unimplemented(W2M_API_GetFullPathNameW, 0);
    long last;
    u64 n = w2m_full_path((const u16 *)name, scratch, &last);
    if (!n) {
        set_error(ERROR_INVALID_NAME);
        return w2m_logged(W2M_API_GetFullPathNameW, 0, 0);
    }
    u32 cap = (u32)size;
    u16 *b = (u16 *)buffer;
    if (cap > n) {
        for (u64 k = 0; k <= n; k++) b[k] = scratch[k];
        if (part) *(u64 *)part = last < 0 ? 0 : buffer + 2 * (u64)last;
        return w2m_logged(W2M_API_GetFullPathNameW, n, n);
    }
    if (cap) {                                          /* row full.small_buffer: Windows writes into a short buffer */
        for (u64 k = 0; k + 1 < cap; k++) b[k] = scratch[k];
        b[cap - 1] = 0;
    }
    return w2m_logged(W2M_API_GetFullPathNameW, n, n + 1);
}

/* For processes.c: the directory a child started with no directory of its own gets (Sprint 058). */
const u16 *w2m_working_directory(void) { return working(); }

u64 k32_GetCurrentDirectoryW(u64 size, u64 buffer) {
    const u16 *w = working();
    u64 n = length(w);
    if (!buffer || (u32)size <= n) return w2m_logged(W2M_API_GetCurrentDirectoryW, n, n + 1);
    for (u64 k = 0; k <= n; k++) ((u16 *)buffer)[k] = w[k];
    return w2m_logged(W2M_API_GetCurrentDirectoryW, n, n);
}

u64 w2m_file_stat_full(const u16 *full, u64 out, u64 *attributes);   /* file.c: FILE_STAT on a full path */

/* The kind of a full path: 0 absent, 1 file, 2 directory, 3 a directory on the way is missing. */
static u64 kind_of(const u16 *full) {
    static u64 st[4];
    return w2m_file_stat_full(full, (u64)st, 0);
}

static int same(const u16 *a, const u16 *b) {
    u64 k = 0;
    for (; a[k] && b[k]; k++) if (upper(a[k]) != upper(b[k])) return 0;
    return a[k] == b[k];
}

u64 k32_SetCurrentDirectoryW(u64 name) {
    long last;
    u64 n = w2m_full_path((const u16 *)name, scratch, &last);
    if (!n) { set_error(ERROR_INVALID_NAME); return w2m_logged(W2M_API_SetCurrentDirectoryW, 0, 0); }
    u64 kind = kind_of(scratch);
    if (kind != 2) {
        set_error(kind == 1 ? ERROR_DIRECTORY : kind == 0 ? ERROR_FILE_NOT_FOUND : ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_SetCurrentDirectoryW, kind, 0);
    }
    if (n > 3 && scratch[n - 1] == '\\') scratch[--n] = 0;
    /* DosPath gets the path and a final '\\' in the other of its two buffers (row after.current_directory: Windows
     * moves it to a new buffer); MaximumLength is 0x208 bytes */
    u8 *p = w2m_parameters();
    u64 units = n + (scratch[n - 1] != '\\');
    if (2 * units + 2 > 0x208) w2m_unimplemented(W2M_API_SetCurrentDirectoryW, n);   /* longer than MAX_PATH */
    u16 *now = *(u16 **)(p + 0x40), *next = (u16 *)W2M_CURRENT_DIRECTORY_VA;
    if (now == next) next += 0x208 / 2;
    for (u64 k = 0; k < n; k++) next[k] = scratch[k];
    next[units - 1] = '\\';
    next[units] = 0;
    *(u16 *)(p + 0x38) = (u16)(2 * units);
    *(u16 **)(p + 0x40) = next;
    return w2m_logged(W2M_API_SetCurrentDirectoryW, n, 1);
}

u64 k32_CreateDirectoryW(u64 name, u64 security) {
    if (security) w2m_unimplemented(W2M_API_CreateDirectoryW, security);       /* not in the rows */
    long last;
    u64 n = w2m_full_path((const u16 *)name, scratch, &last);
    if (!n) { set_error(ERROR_PATH_NOT_FOUND); return w2m_logged(W2M_API_CreateDirectoryW, 0, 0); }
    long r = (long)w2m_host(W2M_FILE_MKDIR, (u64)scratch, n, 0, 0, 0);
    if (r) {
        set_error(r == -EEXIST ? ERROR_ALREADY_EXISTS : r == -ENOENT ? ERROR_PATH_NOT_FOUND : 0);
        if (r != -EEXIST && r != -ENOENT) w2m_unimplemented(W2M_API_CreateDirectoryW, (u64)-r);
        return w2m_logged(W2M_API_CreateDirectoryW, (u64)-r, 0);
    }
    return w2m_logged(W2M_API_CreateDirectoryW, n, 1);
}

u64 k32_RemoveDirectoryW(u64 name) {
    long last;
    u64 n = w2m_full_path((const u16 *)name, scratch, &last);
    if (!n) { set_error(ERROR_PATH_NOT_FOUND); return w2m_logged(W2M_API_RemoveDirectoryW, 0, 0); }
    u64 kind = kind_of(scratch);
    if (kind != 2) {
        set_error(kind == 1 ? ERROR_DIRECTORY : kind == 0 ? ERROR_FILE_NOT_FOUND : ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_RemoveDirectoryW, kind, 0);
    }
    if (n > 3 && scratch[n - 1] == '\\') scratch[--n] = 0;
    if (same(scratch, working())) {                    /* row dir.remove_cwd: the working directory is in use */
        set_error(ERROR_SHARING_VIOLATION);
        return w2m_logged(W2M_API_RemoveDirectoryW, n, 0);
    }
    long r = (long)w2m_host(W2M_FILE_RMDIR, (u64)scratch, n, 0, 0, 0);
    if (r) {
        if (r != -ENOTEMPTY) w2m_unimplemented(W2M_API_RemoveDirectoryW, (u64)-r);
        set_error(ERROR_DIR_NOT_EMPTY);
        return w2m_logged(W2M_API_RemoveDirectoryW, (u64)-r, 0);
    }
    return w2m_logged(W2M_API_RemoveDirectoryW, n, 1);
}
