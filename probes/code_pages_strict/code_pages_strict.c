// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* code_pages_strict.c — MultiByteToWideChar on the ANSI code page with MB_ERR_INVALID_CHARS (Sprint 065).
 *
 * The frontier of 2026-10-12 (49,662,520,368 instructions) stops at MultiByteToWideChar with flags 0x9
 * (MB_PRECOMPOSED | MB_ERR_INVALID_CHARS) on a code page other than UTF-8; contracts/windows.code_pages.ansi_1252
 * 0.2.0 has no row for those flags there. One JSON line per (code page, flags, byte): CP_ACP and 1252, flags 0x8 and
 * 0x9, every byte 0x00-0xFF alone (length 1, room for 4 units): the return, the last error after SetLastError(0xDEAD)
 * and the first unit written. Then the same four forms over "a\x81z" with length -1 and a size query, and flags 0x9
 * on CP_UTF8 (UTF-8 takes no MB_PRECOMPOSED) over "ab".
 */
#include <windows.h>
#include <stdio.h>

static void one(const char *name, UINT cp, DWORD flags, const char *in, int len, int cap) {
    WCHAR out[8] = {0xA5A5, 0xA5A5, 0xA5A5, 0xA5A5, 0xA5A5, 0xA5A5, 0xA5A5, 0xA5A5};
    SetLastError(0xDEAD);
    int r = MultiByteToWideChar(cp, flags, in, len, cap ? out : NULL, cap);
    DWORD e = GetLastError();
    printf("{\"row\": \"%s\", \"cp\": %u, \"flags\": %lu, \"ret\": %d, \"err\": %lu, \"units\": [%u, %u, %u, %u]}\n",
           name, cp, flags, r, e, out[0], out[1], out[2], out[3]);
}

int main(void) {
    static const UINT cps[] = {CP_ACP, 1252};
    static const DWORD flags[] = {MB_ERR_INVALID_CHARS, MB_PRECOMPOSED | MB_ERR_INVALID_CHARS};
    char name[64];
    for (int c = 0; c < 2; c++)
        for (int f = 0; f < 2; f++) {
            for (int b = 0; b < 256; b++) {
                char in[1] = {(char)b};
                snprintf(name, sizeof name, "byte.%02x", b);
                one(name, cps[c], flags[f], in, 1, 4);
            }
            one("mixed", cps[c], flags[f], "a\x81z", -1, 8);
            one("mixed.size", cps[c], flags[f], "a\x81z", -1, 0);
        }
    one("utf8.precomposed_strict", CP_UTF8, MB_PRECOMPOSED | MB_ERR_INVALID_CHARS, "ab", 2, 4);
    printf("{\"acp\": %u}\n", GetACP());
    return 0;
}
