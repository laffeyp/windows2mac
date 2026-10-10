// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* file_attributes.c — GetFileAttributesExW and GetFileAttributesW (Sprint 047, family "file attributes").
 *
 * As contracts/windows.file.attributes records Windows answering (Server 2025 and 2022; identical but for a
 * file named with a trailing backslash, ERROR_DIRECTORY on 2025 and ERROR_INVALID_NAME on 2022):
 *   - a file: FILE_ATTRIBUTE_ARCHIVE (0x20), its size, times with creation no later than the last write and
 *     the last write when it was written; any case, trailing dots and spaces stripped (as CreateFileW names)
 *   - a directory ("." and ".\"): FILE_ATTRIBUTE_DIRECTORY (0x10), size 4096
 *   - the empty name and a missing directory or a file used as one: ERROR_PATH_NOT_FOUND (3); a missing file:
 *     ERROR_FILE_NOT_FOUND (2); "*" or "?": ERROR_INVALID_NAME (123); a file with a trailing backslash:
 *     ERROR_DIRECTORY (267), as 2025; an information level other than GetFileExInfoStandard:
 *     ERROR_INVALID_PARAMETER (87)
 *   - success leaves the last error; failure leaves the caller's structure untouched; GetFileAttributesW answers
 *     the attributes, or INVALID_FILE_ATTRIBUTES (0xFFFFFFFF) with the same error
 * File times are the Mac's (birth, access, modification) under the run's root.
 */
#include "runtime.h"

typedef unsigned short u16;

#define ERROR_FILE_NOT_FOUND 2u
#define ERROR_PATH_NOT_FOUND 3u
#define ERROR_INVALID_PARAMETER 87u
#define ERROR_INVALID_NAME 123u
#define ERROR_DIRECTORY 267u
#define ATTRIBUTE_DIRECTORY 0x10u
#define ATTRIBUTE_ARCHIVE 0x20u

u64 w2m_file_stat(u64 name, u64 out);                /* file.c */

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

/* 0 and the attributes, size and times in *attributes and st; or the error. */
static u32 look(u64 api, u64 name, u32 *attributes, u64 st[4]) {
    if (!name) w2m_unimplemented(api, 0);                                   /* not in the rows */
    const u16 *s = (const u16 *)name;
    if (!s[0]) return ERROR_PATH_NOT_FOUND;
    u64 n = 0;
    for (; s[n]; n++)
        if (s[n] == '*' || s[n] == '?') return ERROR_INVALID_NAME;
    int trailing = s[n - 1] == '\\' || s[n - 1] == '/';
    u64 kind = w2m_file_stat(name, (u64)st);
    if (kind == 0) return ERROR_FILE_NOT_FOUND;
    if (kind == 3) return ERROR_PATH_NOT_FOUND;
    if (kind == 1 && trailing) return ERROR_DIRECTORY;
    *attributes = kind == 2 ? ATTRIBUTE_DIRECTORY : ATTRIBUTE_ARCHIVE;
    if (kind == 2) st[0] = 4096;                                             /* rows ex.dot, ex.dot.backslash */
    return 0;
}

u64 k32_GetFileAttributesExW(u64 name, u64 level, u64 data) {
    if ((u32)level != 0) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_GetFileAttributesExW, level, 0);
    }
    u32 attributes = 0;
    u64 st[4];
    u32 err = look(W2M_API_GetFileAttributesExW, name, &attributes, st);
    if (err) {
        set_error(err);
        return w2m_logged(W2M_API_GetFileAttributesExW, err, 0);
    }
    u8 *d = (u8 *)data;                                /* WIN32_FILE_ATTRIBUTE_DATA, 36 bytes */
    *(u32 *)d = attributes;
    *(u32 *)(d + 4) = (u32)st[1]; *(u32 *)(d + 8) = (u32)(st[1] >> 32);     /* ftCreationTime */
    *(u32 *)(d + 12) = (u32)st[2]; *(u32 *)(d + 16) = (u32)(st[2] >> 32);   /* ftLastAccessTime */
    *(u32 *)(d + 20) = (u32)st[3]; *(u32 *)(d + 24) = (u32)(st[3] >> 32);   /* ftLastWriteTime */
    *(u32 *)(d + 28) = (u32)(st[0] >> 32); *(u32 *)(d + 32) = (u32)st[0];   /* nFileSizeHigh, nFileSizeLow */
    return w2m_logged(W2M_API_GetFileAttributesExW, attributes, 1);
}

u64 k32_GetFileAttributesW(u64 name) {
    u32 attributes = 0;
    u64 st[4];
    u32 err = look(W2M_API_GetFileAttributesW, name, &attributes, st);
    if (err) {
        set_error(err);
        return w2m_logged(W2M_API_GetFileAttributesW, err, 0xFFFFFFFFu);
    }
    return w2m_logged(W2M_API_GetFileAttributesW, attributes, attributes);
}
