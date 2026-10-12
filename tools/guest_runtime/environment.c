// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* environment.c — the environment block (Sprint 044, family "environment").
 *
 * GetEnvironmentStringsW, FreeEnvironmentStringsW and lstrlenW, as
 * contracts/windows.environment.{set,empty}_block record Windows answering a process started with an
 * explicit block and no handles (12 runners, Server 2025 and 2022, AMD and Intel; 24 identical outputs):
 *   - the block is the one the start passed, entries in its order, a drive variable ("=C:=...") kept,
 *     nothing added
 *   - each call returns a new copy from the process heap of exactly the block's size: the entries, each
 *     with its NUL, then a NUL (69 units, 138 bytes, for five entries); an empty block is two NULs
 *     (HeapSize 4); a write into one copy does not show in the next
 *   - FreeEnvironmentStringsW frees the copy and answers TRUE; both calls leave the last error
 * The block is the host's: UTF-16 entries at ProcessParameters.Environment (src/w2m/vm_backend.py, `environment`).
 *
 * GetEnvironmentVariableW, SetEnvironmentVariableW and GetTempPathW (Sprint 048), as contracts/windows.environment.
 * temp_path records Windows answering (Server 2025 and 2022): SetEnvironmentVariableW changes the block in place
 * (half of W2M_ENVIRONMENT_SIZE bytes, Sprint 059; a block that would outgrow it stops the run):
 *   - names compare case-insensitively; a replaced variable keeps its spelling; a new one goes where the entries'
 *     upper-cased names keep their order ("=C:" before "alpha" before "EMPTY" before "ZED")
 *   - a NULL value deletes (TRUE also for a missing name); an empty name or one with '=' after its first character
 *     answers FALSE and 87; success leaves the last error
 * GetEnvironmentVariableW: the value and its length; a buffer too small gets a NUL at [0] and the answer is the size
 * needed; a missing name answers 0 and 203 and leaves the buffer; an empty value answers 0, writes the NUL and leaves
 * the last error. GetTempPathW: the first of TMP, TEMP and USERPROFILE, made full (GetFullPathNameW's rules) with
 * one backslash after it, else C:\Windows\; a value whose path passes MAX_PATH is skipped; an empty value answers
 * 1 and writes nothing; a buffer too small gets size - 1 units and a NUL and the answer is the size needed; the last
 * error is left.
 */
#include "runtime.h"

typedef unsigned short u16;

#define ERROR_INVALID_PARAMETER 87u
#define ERROR_ENVVAR_NOT_FOUND 203u
#define TEMP_MAX 260                                 /* MAX_PATH: GetTempPathW's longest path ("maximum ... 261") */

u64 w2m_full_path(const u16 *in, u16 *out, long *part);    /* paths.c */

/* ProcessParameters.Environment (Sprint 059): the host's block at the start, in the first half of its area; each
 * change moves it to the other half and sets EnvironmentSize to the bytes in use (row after.set_environment: Windows
 * gives the block a new address and its new size). */
static u16 *block(void) { return *(u16 **)(w2m_parameters() + 0x80); }
static u16 upper(u16 c) { return c >= 'a' && c <= 'z' ? (u16)(c - 32) : c; }
static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

/* The units in use: the entries, each with its NUL, then the final NUL (an empty block: 1). */
static u64 used(void) {
    const u16 *b = block();
    if (!b[0]) return 1;
    u64 n = 0;
    while (b[n] || b[n + 1]) n++;
    return n + 2;
}

/* The name's length in an entry: to the first '=' after the first unit. */
static u64 name_length(const u16 *e) {
    u64 k = 1;
    while (e[k] && e[k] != '=') k++;
    return e[0] ? k : 0;
}

/* -1, 0, 1: the upper-cased name `a` (length la) against `b` (length lb). */
static int compare(const u16 *a, u64 la, const u16 *b, u64 lb) {
    for (u64 i = 0; i < la && i < lb; i++) {
        u16 x = upper(a[i]), y = upper(b[i]);
        if (x != y) return x < y ? -1 : 1;
    }
    return la == lb ? 0 : la < lb ? -1 : 1;
}

/* The entry named `name` (length len), or 0; *at: where it is, or where a new one goes. */
static u16 *find(const u16 *name, u64 len, u16 **at) {
    u16 *e = block();
    while (*e) {
        int c = compare(e, name_length(e), name, len);
        if (c == 0) { *at = e; return e; }
        if (c > 0) break;
        while (*e) e++;
        e++;
    }
    *at = e;
    return 0;
}

static u64 entry_units(const u16 *e) { u64 n = 0; while (e[n]) n++; return n + 1; }

/* The value of an entry, after its '='. */
static const u16 *value_of(u16 *e) { return e + name_length(e) + 1; }

static u64 length(const u16 *s) { u64 n = 0; while (s[n]) n++; return n; }

/* Move the block's tail from `from` by `delta` units (the final NUL included). */
static void shift(u16 *from, long delta) {
    u16 *b = block();
    u64 end = used();
    u64 i = (u64)(from - b);
    if (delta > 0) for (u64 k = end; k-- > i; ) b[k + (u64)delta] = b[k];
    else for (u64 k = i; k < end; k++) b[k + (u64)delta] = b[k];
}

/* After a change: the block to the other half of the area, with its size and a new version. */
static void moved(void) {
    u8 *p = w2m_parameters();
    u16 *from = block(), *to = (u16 *)W2M_ENVIRONMENT_VA;
    if (from == to) to += W2M_ENVIRONMENT_SIZE / 4;
    u64 n = used();
    for (u64 k = 0; k < n; k++) to[k] = from[k];
    if (n == 1) to[1] = 0;
    *(u16 **)(p + 0x80) = to;
    *(u64 *)(p + 0x3F0) = 2 * (n == 1 ? 2 : n);                 /* an empty block: two NULs */
    *(u64 *)(p + 0x3F8) += 1;
}

u64 k32_GetEnvironmentStringsW(void) {
    const u16 *host = block();
    u64 units = 0;
    if (!host[0]) units = 2;                         /* empty: two NULs (row empty.heap) */
    else {
        while (host[units] || host[units + 1]) units++;
        units += 2;
    }
    u64 copy = w2m_heap_take(units * 2);
    if (!copy) w2m_unimplemented(W2M_API_GetEnvironmentStringsW, units);
    u16 *to = (u16 *)copy;
    for (u64 i = 0; i < units; i++) to[i] = host[i];
    return w2m_logged(W2M_API_GetEnvironmentStringsW, units, copy);
}

u64 k32_FreeEnvironmentStringsW(u64 block) {
    if (!block) w2m_unimplemented(W2M_API_FreeEnvironmentStringsW, 0);     /* not in the rows */
    w2m_heap_release(W2M_API_FreeEnvironmentStringsW, block);
    return w2m_logged(W2M_API_FreeEnvironmentStringsW, block, 1);
}

/* lstrlenW: the units before the NUL (the probe's child counts entries with it). */
u64 k32_lstrlenW(u64 s) {
    if (!s) w2m_unimplemented(W2M_API_lstrlenW, 0);                         /* not in the rows */
    const u16 *w = (const u16 *)s;
    u64 n = 0;
    while (w[n]) n++;
    return w2m_logged(W2M_API_lstrlenW, s, n);
}

/* GetEnvironmentVariableW(lpName, lpBuffer, nSize) */
u64 k32_GetEnvironmentVariableW(u64 name, u64 buffer, u64 size) {
    if (!name) w2m_unimplemented(W2M_API_GetEnvironmentVariableW, 0);                   /* not in the rows */
    const u16 *n = (const u16 *)name;
    u16 *at;
    u16 *e = find(n, length(n), &at);
    if (!e || !length(n)) {
        set_error(ERROR_ENVVAR_NOT_FOUND);
        return w2m_logged(W2M_API_GetEnvironmentVariableW, name, 0);
    }
    const u16 *v = value_of(e);
    u64 len = length(v);
    u32 cap = (u32)size;
    u16 *b = (u16 *)buffer;
    if (len < cap && b) {
        for (u64 k = 0; k <= len; k++) b[k] = v[k];
        return w2m_logged(W2M_API_GetEnvironmentVariableW, name, len);
    }
    if (cap && b) b[0] = 0;                                      /* row get.size_short */
    return w2m_logged(W2M_API_GetEnvironmentVariableW, name, len + 1);
}

/* SetEnvironmentVariableW(lpName, lpValue) */
u64 k32_SetEnvironmentVariableW(u64 name, u64 value) {
    if (!name) w2m_unimplemented(W2M_API_SetEnvironmentVariableW, 0);                   /* not in the rows */
    const u16 *n = (const u16 *)name;
    u64 len = length(n);
    int bad = len == 0;
    for (u64 k = 1; k < len; k++) bad |= n[k] == '=';
    if (bad) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_SetEnvironmentVariableW, name, 0);
    }
    u16 *at;
    u16 *e = find(n, len, &at);
    const u16 *keep = n;                                         /* the spelling the entry is written with */
    u64 old = 0;
    if (e) {
        old = entry_units(e);
        keep = e;
    }
    if (!value) {                                                /* delete: TRUE also when missing */
        if (e) {
            shift(e + old, -(long)old);
            if (!block()[0]) block()[1] = 0;
            moved();
        }
        return w2m_logged(W2M_API_SetEnvironmentVariableW, name, 1);
    }
    const u16 *v = (const u16 *)value;
    u64 vlen = length(v), units = len + 1 + vlen + 1;
    if (used() - old + units + 1 > W2M_ENVIRONMENT_SIZE / 4)
        w2m_unimplemented(W2M_API_SetEnvironmentVariableW, units);   /* the block's half of the area is full */
    u16 spelled[256];
    if (len >= 256) w2m_unimplemented(W2M_API_SetEnvironmentVariableW, len);
    for (u64 k = 0; k < len; k++) spelled[k] = keep[k];
    int empty = !block()[0];
    if (empty) block()[1] = 0;                                   /* "\0\0": the new entry and the final NUL follow */
    shift(at + old, (long)units - (long)old);
    if (empty) at[units] = 0;
    for (u64 k = 0; k < len; k++) at[k] = spelled[k];
    at[len] = '=';
    for (u64 k = 0; k <= vlen; k++) at[len + 1 + k] = v[k];
    moved();
    return w2m_logged(W2M_API_SetEnvironmentVariableW, name, 1);
}

/* GetTempPathW(nBufferLength, lpBuffer) */
u64 k32_GetTempPathW(u64 size, u64 buffer) {
    static const u16 TMP[] = {'T', 'M', 'P', 0}, TEMP[] = {'T', 'E', 'M', 'P', 0};
    static const u16 PROFILE[] = {'U', 'S', 'E', 'R', 'P', 'R', 'O', 'F', 'I', 'L', 'E', 0};
    static const u16 WINDOWS[] = {'C', ':', '\\', 'W', 'i', 'n', 'd', 'o', 'w', 's', '\\', 0};
    static const u16 *const ORDER[] = {TMP, TEMP, PROFILE};
    static u16 path[1024];
    u64 len = 0;
    for (int i = 0; i < 3 && !len; i++) {
        u16 *at;
        u16 *e = find(ORDER[i], length(ORDER[i]), &at);
        if (!e) continue;
        const u16 *v = value_of(e);
        if (!v[0])                                               /* row temp.tmp_empty: 1, nothing written */
            return w2m_logged(W2M_API_GetTempPathW, size, 1);
        if (length(v) > TEMP_MAX) continue;                      /* row temp.long: the next variable */
        long part;
        u64 n = w2m_full_path(v, path, &part);
        if (!n) w2m_unimplemented(W2M_API_GetTempPathW, i);      /* blanks only: not in the rows */
        if (path[n - 1] != '\\') path[n++] = '\\';
        path[n] = 0;
        if (n <= TEMP_MAX) len = n;
    }
    if (!len) {
        for (len = 0; WINDOWS[len]; len++) path[len] = WINDOWS[len];
        path[len] = 0;
    }
    u32 cap = (u32)size;
    u16 *b = (u16 *)buffer;
    if (len < cap && b) {
        for (u64 k = 0; k <= len; k++) b[k] = path[k];
        return w2m_logged(W2M_API_GetTempPathW, size, len);
    }
    if (cap && b) {                                              /* row temp.size_short */
        for (u64 k = 0; k + 1 < cap; k++) b[k] = path[k];
        b[cap - 1] = 0;
    }
    return w2m_logged(W2M_API_GetTempPathW, size, len + 1);
}
