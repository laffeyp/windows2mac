// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* manifest.c — what the program's manifest asks of Windows (Sprint 054).
 *
 * Windows answers some questions by the running exe's manifest (its RT_MANIFEST resource, id 1): a program that
 * names Windows 10 among its supportedOS is told version 10.0, one that does not 6.2 build 9200 (contracts
 * windows.version.version and windows.system.information); a program that declares the PerMonitorV2 DPI awareness
 * sees the display unscaled. The game's manifest (games/hail-to-the-rainbow/manifest.xml) declares both. The exe is
 * mapped at PEB.ImageBaseAddress; its resource directory is read here as the loader reads it (PE Format, "The .rsrc
 * Section"), and the manifest's text searched for the two declarations.
 */
#include "runtime.h"

typedef unsigned short u16;

#define RT_MANIFEST 24u

static const char WINDOWS_10[] = "8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a";   /* the supportedOS Id of Windows 10 */
static const char PER_MONITOR_V2[] = "PerMonitorV2";

static char lower(char c) { return c >= 'A' && c <= 'Z' ? (char)(c + 32) : c; }

static int contains(const u8 *text, u32 size, const char *word) {
    u32 n = 0;
    while (word[n]) n++;
    for (u32 i = 0; i + n <= size; i++) {
        u32 k = 0;
        while (k < n && lower((char)text[i + k]) == lower(word[k])) k++;
        if (k == n) return 1;
    }
    return 0;
}

/* The entry of `id` in a resource directory at `dir` (the first entry when id is 0), or 0. */
static u32 entry(u64 root, u32 dir, u32 id) {
    const u8 *d = (const u8 *)(root + dir);
    u32 named = *(const u16 *)(d + 12), ids = *(const u16 *)(d + 14);
    for (u32 i = 0; i < named + ids; i++) {
        u32 key = *(const u32 *)(d + 16 + 8 * i), target = *(const u32 *)(d + 20 + 8 * i);
        if (!id || (!(key & 0x80000000u) && key == id)) return target;
    }
    return 0;
}

static u32 read_flags(void) {
    u64 base = *(const volatile u64 *)(W2M_PEB_VA + 0x10);            /* PEB.ImageBaseAddress */
    const u8 *nt = (const u8 *)base + *(const u32 *)(base + 0x3c);
    const u8 *opt = nt + 24;
    if (*(const u32 *)(opt + 108) <= 2) return 0;                    /* NumberOfRvaAndSizes */
    u32 rva = *(const u32 *)(opt + 112 + 2 * 8);
    if (!rva) return 0;
    u64 root = base + rva;
    u32 type = entry(root, 0, RT_MANIFEST);
    if (!(type & 0x80000000u)) return 0;
    u32 name = entry(root, type & 0x7FFFFFFFu, 1);
    if (!(name & 0x80000000u)) return 0;
    u32 lang = entry(root, name & 0x7FFFFFFFu, 0);
    if (!lang || (lang & 0x80000000u)) return 0;
    const u32 *data = (const u32 *)(root + lang);                     /* IMAGE_RESOURCE_DATA_ENTRY: RVA, size */
    const u8 *text = (const u8 *)(base + data[0]);
    return (contains(text, data[1], WINDOWS_10) ? W2M_MANIFEST_WINDOWS_10 : 0) |
           (contains(text, data[1], PER_MONITOR_V2) ? W2M_MANIFEST_PER_MONITOR_V2 : 0);
}

u32 w2m_manifest(void) {
    static u32 flags, read;
    if (!read) { flags = read_flags(); read = 1; }
    return flags;
}
