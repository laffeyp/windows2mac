// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* file_search.c — FindFirstFileExW, FindNextFileW and FindClose (Sprint 054, family "file_search";
 * specs/work_list.json).
 *
 * contracts/windows.file.file_search records Windows (Server 2025 and 2022, equal) for UnityPlayer's form,
 * FindFirstFileExW(name, FindExInfoBasic, &data, FindExSearchNameMatch, NULL, 0):
 *   - a folder's entries come "." and ".." first, then by name compared without case ("a.dat", "b.txt", "Sub");
 *     a folder has FILE_ATTRIBUTE_DIRECTORY (0x10) and size 0, a file FILE_ATTRIBUTE_ARCHIVE (0x20) and its size;
 *     the times are GetFileAttributesExW's; FindExInfoBasic leaves the short name empty
 *   - "*" and "?" match without case; a name without them finds that one entry, spelled as stored; a folder's own
 *     name finds the folder
 *   - past the last entry FindNextFileW answers FALSE with ERROR_NO_MORE_FILES (18); success leaves the last error
 *   - a missing folder is ERROR_PATH_NOT_FOUND (3), no match ERROR_FILE_NOT_FOUND (2), both INVALID_HANDLE_VALUE
 *   - FindClose answers TRUE, and FALSE with ERROR_INVALID_HANDLE (6) for INVALID_HANDLE_VALUE
 * The folder is listed through the host (FILE_LIST); matching, order and the dot entries are Windows's and live here.
 * FindExInfoStandard (short names), other search operations and flags, a filter, the DOS wildcards (<, >, ") and
 * "*.*" (whose DOS meaning no row fixes) stop the run.
 */
#include "runtime.h"

typedef unsigned short u16;

#define ERROR_FILE_NOT_FOUND 2u
#define ERROR_PATH_NOT_FOUND 3u
#define ERROR_INVALID_HANDLE 6u
#define ERROR_NO_MORE_FILES 18u
#define ATTRIBUTE_DIRECTORY 0x10u
#define ATTRIBUTE_ARCHIVE 0x20u
#define FIND_EX_INFO_BASIC 1u
#define INVALID_HANDLE 0xFFFFFFFFFFFFFFFFull
#define SEARCH_BASE 0x4000ull                       /* search handles: SEARCH_BASE + 4*slot */
#define SEARCHES 16
#define FULL 1024

u64 w2m_full_path(const u16 *in, u16 *out, long *part);       /* paths.c */
u64 w2m_file_stat_full(const u16 *full, u64 out);              /* file.c */

/* A search: its folder's full path, the matching names (each with a NUL) in a heap block, pointers to them in the
 * order they are answered, and the next to answer. */
static struct search { int used; u64 names; u16 **order; u64 count, next; u16 folder[FULL]; } searches[SEARCHES];

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }
static u16 upper(u16 c) { return c >= 'a' && c <= 'z' ? (u16)(c - 32) : c; }

/* A name against a pattern of "*" and "?", without case. */
static int matches(const u16 *p, const u16 *s) {
    if (!*p) return !*s;
    if (*p == '*') {
        for (;; s++) {
            if (matches(p + 1, s)) return 1;
            if (!*s) return 0;
        }
    }
    if (!*s) return 0;
    return (*p == '?' || upper(*p) == upper(*s)) && matches(p + 1, s + 1);
}

static int before(const u16 *a, const u16 *b) {
    for (; *a && upper(*a) == upper(*b); a++, b++) {}
    return upper(*a) < upper(*b);
}

static u64 length(const u16 *s) { u64 n = 0; while (s[n]) n++; return n; }

/* The entry for `name` in the search's folder into WIN32_FIND_DATAW `data`. */
static void fill(const struct search *f, const u16 *name, u64 data) {
    u16 full[FULL];
    u64 n = length(f->folder);
    for (u64 i = 0; i <= n; i++) full[i] = f->folder[i];
    int dot = name[0] == '.' && (!name[1] || (name[1] == '.' && !name[2]));
    if (!dot) {
        if (n && full[n - 1] != '\\') full[n++] = '\\';
        for (u64 i = 0; name[i]; i++) full[n++] = name[i];
        full[n] = 0;
    } else if (name[1]) {                                /* "..": the parent folder */
        while (n && full[n - 1] != '\\') n--;
        if (n > 3) n--;
        full[n] = 0;
    }
    u64 st[4] = {0, 0, 0, 0};
    u64 kind = w2m_file_stat_full(full, (u64)st);
    u8 *d = (u8 *)data;
    for (int i = 0; i < 592; i++) d[i] = 0;
    *(u32 *)d = kind == 2 ? ATTRIBUTE_DIRECTORY : ATTRIBUTE_ARCHIVE;
    *(u64 *)(d + 4) = st[1];                             /* ftCreationTime */
    *(u64 *)(d + 12) = st[2];                            /* ftLastAccessTime */
    *(u64 *)(d + 20) = st[3];                            /* ftLastWriteTime */
    if (kind != 2) { *(u32 *)(d + 28) = (u32)(st[0] >> 32); *(u32 *)(d + 32) = (u32)st[0]; }
    u16 *out = (u16 *)(d + 44);                          /* cFileName[260]; cAlternateFileName stays empty */
    for (int i = 0; name[i] && i < 259; i++) out[i] = name[i];
}

/* FindFirstFileExW(lpFileName, fInfoLevelId, lpFindFileData, fSearchOp, lpSearchFilter, dwAdditionalFlags) */
u64 k32_FindFirstFileExW(u64 name, u64 level, u64 data, u64 op, u64 rsp) {
    u64 filter = *(const u64 *)(rsp + 0x28), flags = *(const u64 *)(rsp + 0x30);
    if ((u32)level != FIND_EX_INFO_BASIC) w2m_unimplemented(W2M_API_FindFirstFileExW, level);
    if ((u32)op || filter || (u32)flags) w2m_unimplemented(W2M_API_FindFirstFileExW, 0x100 | (u32)op);
    u16 full[FULL];
    long part;
    u64 n = w2m_full_path((const u16 *)name, full, &part);
    if (!n || n >= FULL) w2m_unimplemented(W2M_API_FindFirstFileExW, n);
    u64 cut = n;
    while (cut && full[cut - 1] != '\\') cut--;
    const u16 *pattern = full + cut;
    for (const u16 *p = pattern; *p; p++)
        if (*p == '<' || *p == '>' || *p == '"') w2m_unimplemented(W2M_API_FindFirstFileExW, *p);
    if (pattern[0] == '*' && pattern[1] == '.' && pattern[2] == '*' && !pattern[3])
        w2m_unimplemented(W2M_API_FindFirstFileExW, 0x2A2E2A);
    int slot = -1;
    for (int i = 0; i < SEARCHES; i++) if (!searches[i].used) { slot = i; break; }
    if (slot < 0) w2m_unimplemented(W2M_API_FindFirstFileExW, SEARCHES);
    struct search *f = &searches[slot];
    u64 folder = cut > 3 ? cut - 1 : cut;                /* "C:\" keeps its separator */
    for (u64 i = 0; i < folder; i++) f->folder[i] = full[i];
    f->folder[folder] = 0;

    u64 kind = 0;
    u64 need = w2m_host(W2M_FILE_LIST, (u64)f->folder, folder, 0, 0, &kind);
    if (kind != 2) {
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_FindFirstFileExW, kind, INVALID_HANDLE);
    }
    u64 listing = w2m_heap_take(need + 8);
    w2m_host(W2M_FILE_LIST, (u64)f->folder, folder, listing, need, &kind);
    static const u16 dots[] = {'.', 0, '.', '.', 0};
    u64 total = 0, count = 0;
    for (int pass = 0; pass < 2; pass++) {                /* sizes, then the copy */
        const u16 *list[2] = {dots, (const u16 *)listing};
        u16 *to = pass ? (u16 *)f->names : 0;
        for (int which = folder > 3 ? 0 : 1; which < 2; which++)
            for (const u16 *s = list[which]; (which || s < dots + 5) && *s; s += length(s) + 1) {
                if (!matches(pattern, s)) continue;
                u64 k = length(s) + 1;
                if (pass) { for (u64 i = 0; i < k; i++) *to++ = s[i]; }
                else { total += k; count++; }
            }
        if (!pass) {
            if (!count) {
                w2m_heap_release(W2M_API_FindFirstFileExW, listing);
                set_error(ERROR_FILE_NOT_FOUND);
                return w2m_logged(W2M_API_FindFirstFileExW, 0, INVALID_HANDLE);
            }
            f->names = w2m_heap_take(2 * total + 8);
        }
    }
    w2m_heap_release(W2M_API_FindFirstFileExW, listing);
    /* order: the dot entries as listed first, then insertion by name without case */
    u16 **order = (u16 **)w2m_heap_take(8 * count);
    u64 dotted = 0, k = 0;
    for (u16 *s = (u16 *)f->names; k < count; s += length(s) + 1, k++) {
        order[k] = s;
        if (s[0] == '.' && (!s[1] || (s[1] == '.' && !s[2]))) dotted++;
    }
    for (u64 i = dotted + 1; i < count; i++)
        for (u64 j = i; j > dotted && before(order[j], order[j - 1]); j--) {
            u16 *t = order[j]; order[j] = order[j - 1]; order[j - 1] = t;
        }
    f->order = order;
    f->count = count;
    f->next = 1;
    f->used = 1;
    fill(f, order[0], data);
    return w2m_logged(W2M_API_FindFirstFileExW, count, SEARCH_BASE + 4 * (u64)slot);
}

static struct search *search_of(u64 h) {
    if (h < SEARCH_BASE || (h - SEARCH_BASE) % 4 || (h - SEARCH_BASE) / 4 >= SEARCHES) return 0;
    struct search *f = &searches[(h - SEARCH_BASE) / 4];
    return f->used ? f : 0;
}

u64 k32_FindNextFileW(u64 h, u64 data) {
    struct search *f = search_of(h);
    if (!f) w2m_unimplemented(W2M_API_FindNextFileW, h);                  /* not in the rows */
    if (f->next >= f->count) {
        set_error(ERROR_NO_MORE_FILES);
        return w2m_logged(W2M_API_FindNextFileW, h, 0);
    }
    fill(f, f->order[f->next++], data);
    return w2m_logged(W2M_API_FindNextFileW, h, 1);
}

u64 k32_FindClose(u64 h) {
    struct search *f = search_of(h);
    if (!f) {
        if (h != INVALID_HANDLE) w2m_unimplemented(W2M_API_FindClose, h);  /* only INVALID_HANDLE_VALUE is in the rows */
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_FindClose, h, 0);
    }
    w2m_heap_release(W2M_API_FindClose, f->names);
    w2m_heap_release(W2M_API_FindClose, (u64)f->order);
    f->used = 0;
    return w2m_logged(W2M_API_FindClose, h, 1);
}
