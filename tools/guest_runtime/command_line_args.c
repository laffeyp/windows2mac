// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* command_line_args.c — SHELL32's CommandLineToArgvW and KERNEL32's LocalFree (Sprint 047, family
 * "command line arguments").
 *
 * As contracts/windows.shell.command_line_to_argv records Windows answering (Server 2025 and 2022, identical):
 *   - the program name (argv[0]): a quoted name runs to the next quote, which ends it even when a
 *     character follows ("\"prog\"x y" gives prog, x, y); an unquoted one runs to the first space or
 *     tab, quotes and backslashes literal ("C:\a\"b c\" d" gives C:\a\"b, then "c d"); leading spaces
 *     give an empty name
 *   - later arguments: 2n backslashes before a quote give n and the quote toggles quoting, 2n+1 give n
 *     and a literal quote, other backslashes are literal; a doubled quote inside quotes is one literal
 *     quote and quoting goes on; an unterminated quote runs to the end; "" makes an empty argument;
 *     trailing blanks make none
 *   - the empty line: one argument, the module's path
 *   - one block: argc + 1 pointers (the last NULL), then the strings; LocalFree returns NULL
 *   - success sets the last error to 0; a NULL count pointer returns NULL with error 87
 */
#include "runtime.h"

typedef unsigned short u16;

u64 k32_GetModuleFileNameW(u64 module, u64 buffer, u64 size);

static int blank(u16 c) { return c == ' ' || c == '\t'; }

/* Parse `line` into `out` (NULL: count only); returns the argument count and adds the units written
 * (each string with its NUL) to *units. */
static u64 parse(const u16 *line, u16 **argv, u16 *out, u64 *units) {
    const u16 *p = line;
    u64 argc = 0, n = 0;
    if (argv) argv[argc] = out + n;
    if (*p == '"') {                                     /* argv[0], quoted: to the next quote */
        for (p++; *p && *p != '"'; p++) { if (out) out[n] = *p; n++; }
        if (*p == '"') p++;
    } else {                                             /* unquoted: to the first blank, all literal */
        for (; *p && !blank(*p); p++) { if (out) out[n] = *p; n++; }
    }
    if (out) out[n] = 0;
    n++;
    argc++;
    for (;;) {
        while (blank(*p)) p++;
        if (!*p) break;
        if (argv) argv[argc] = out + n;
        int quoted = 0;
        for (;;) {
            u64 slashes = 0;
            while (*p == '\\') { slashes++; p++; }
            if (*p == '"') {
                for (u64 i = 0; i < slashes / 2; i++) { if (out) out[n] = '\\'; n++; }
                if (slashes % 2) { if (out) out[n] = '"'; n++; p++; continue; }
                if (quoted && p[1] == '"') { if (out) out[n] = '"'; n++; p += 2; continue; }
                quoted = !quoted;
                p++;
                continue;
            }
            for (u64 i = 0; i < slashes; i++) { if (out) out[n] = '\\'; n++; }
            if (!*p || (!quoted && blank(*p))) break;
            if (out) out[n] = *p;
            n++;
            p++;
        }
        if (out) out[n] = 0;
        n++;
        argc++;
    }
    *units += n;
    return argc;
}

u64 shell32_CommandLineToArgvW(u64 line, u64 count) {
    if (!count) {
        w2m_set_last_error(87);                          /* row null_count */
        return w2m_logged(W2M_API_CommandLineToArgvW, line, 0);
    }
    if (!line) w2m_unimplemented(W2M_API_CommandLineToArgvW, 0);     /* not in the rows */
    const u16 *text = (const u16 *)line;
    u16 path[260];
    if (!text[0]) {                                      /* the empty line: the module's path */
        u64 len = k32_GetModuleFileNameW(0, (u64)path, 260);
        path[len] = 0;
        text = path;
        u64 bytes = 2 * 8 + 2 * (len + 1);
        u64 block = w2m_heap_take(bytes);
        if (!block) w2m_unimplemented(W2M_API_CommandLineToArgvW, bytes);
        u16 **argv = (u16 **)block;
        u16 *s = (u16 *)(block + 16);
        for (u64 i = 0; i <= len; i++) s[i] = path[i];
        argv[0] = s;
        argv[1] = 0;
        *(int *)count = 1;
        w2m_set_last_error(0);
        return w2m_logged(W2M_API_CommandLineToArgvW, line, block);
    }
    u64 units = 0;
    u64 argc = parse(text, 0, 0, &units);
    u64 bytes = 8 * (argc + 1) + 2 * units;
    u64 block = w2m_heap_take(bytes);
    if (!block) w2m_unimplemented(W2M_API_CommandLineToArgvW, bytes);
    u16 **argv = (u16 **)block;
    units = 0;
    parse(text, argv, (u16 *)(block + 8 * (argc + 1)), &units);
    argv[argc] = 0;
    *(int *)count = (int)argc;
    w2m_set_last_error(0);                               /* every row: the last error is 0 after success */
    return w2m_logged(W2M_API_CommandLineToArgvW, line, block);
}

/* LocalFree (contracts/windows.memory.local_free, Sprint 048): NULL and a block's start answer NULL (the block is
 * freed); a pointer 8 bytes past a 16-byte boundary, as UnityPlayer passes argv + 1, answers itself with
 * ERROR_INVALID_HANDLE and frees nothing; Windows ends the process for a 16-aligned pointer inside a block, which
 * is not on the game's path and stops the run here (heap.c checks the block). */
u64 k32_LocalFree(u64 block) {
    if ((block & 15) == 8) {
        w2m_set_last_error(6);
        return w2m_logged(W2M_API_LocalFree, block, block);
    }
    if (block) w2m_heap_release(W2M_API_LocalFree, block);
    return w2m_logged(W2M_API_LocalFree, block, 0);
}
