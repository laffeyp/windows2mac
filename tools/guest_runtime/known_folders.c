// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* known_folders.c — SHELL32's SHGetKnownFolderPath and ole32's CoTaskMemFree (Sprint 048, family "known folders").
 *
 * As contracts/windows.shell.known_folder_path records Windows answering (Server 2025 and 2022, AMD and Intel):
 *   - the profile is C:\Users\<user>; LocalAppDataLow, LocalAppData and RoamingAppData are the profile's
 *     AppData\LocalLow, AppData\Local and AppData\Roaming, each an existing directory, no trailing backslash
 *   - KF_FLAG_DONT_VERIFY and KF_FLAG_CREATE give the same path for a folder that exists
 *   - the path is a new block (CoTaskMemFree frees it); CoTaskMemFree leaves the last error, and NULL is no block
 * w2m's user is the runners' "runneradmin": C:\Users\runneradmin (Sprint 061, Decision 2026-10-11 user: match the
 * reference game run), which the file bridge holds on drive C: (file_bridge.py). Any other folder
 * GUID stops the run: which GUIDs a Windows system registers varies (Microsoft Learn, SHGetKnownFolderPath:
 * "Not all KNOWNFOLDERID values are present on all systems"), and an unregistered one answers 0x80070002 with
 * *ppszPath NULL. The last error after success is left as it was: Windows leaves 1008, 0 or 183 by folder and
 * flag, with no rule the rows show.
 */
#include "runtime.h"

typedef unsigned short u16;

#define KF_FLAG_DONT_VERIFY 0x4000u
#define KF_FLAG_CREATE 0x8000u
#define DIRECTORY 2                                  /* FILE_STAT's kind */

u64 w2m_file_stat_full(const u16 *full, u64 out);       /* file.c */

struct folder { u32 a; u16 b, c; u8 d[8]; const u16 *under; };

static const u16 PROFILE[] = {'C', ':', '\\', 'U', 's', 'e', 'r', 's', '\\', 'r', 'u', 'n', 'n', 'e', 'r', 'a', 'd', 'm', 'i', 'n', 0};
static const u16 NONE[] = {0};
static const u16 LOCAL_LOW[] = {'\\', 'A', 'p', 'p', 'D', 'a', 't', 'a', '\\', 'L', 'o', 'c', 'a', 'l', 'L', 'o', 'w', 0};
static const u16 LOCAL[] = {'\\', 'A', 'p', 'p', 'D', 'a', 't', 'a', '\\', 'L', 'o', 'c', 'a', 'l', 0};
static const u16 ROAMING[] = {'\\', 'A', 'p', 'p', 'D', 'a', 't', 'a', '\\', 'R', 'o', 'a', 'm', 'i', 'n', 'g', 0};

/* KNOWNFOLDERID values (Microsoft Learn, KNOWNFOLDERID, updated 2023-08-30). */
static const struct folder FOLDERS[] = {
    {0x5E6C858F, 0x0E22, 0x4760, {0x9A, 0xFE, 0xEA, 0x33, 0x17, 0xB6, 0x71, 0x73}, NONE},        /* Profile */
    {0xA520A1A4, 0x1780, 0x4FF6, {0xBD, 0x18, 0x16, 0x73, 0x43, 0xC5, 0xAF, 0x16}, LOCAL_LOW},   /* LocalAppDataLow */
    {0xF1B32785, 0x6FBA, 0x4FCF, {0x9D, 0x55, 0x7B, 0x8E, 0x7F, 0x15, 0x70, 0x91}, LOCAL},       /* LocalAppData */
    {0x3EB685DB, 0x65F9, 0x4CF6, {0xA0, 0x3A, 0xE3, 0xEF, 0x65, 0x72, 0x9F, 0x3D}, ROAMING},     /* RoamingAppData */
};

static const struct folder *find(const u8 *id) {
    for (u64 f = 0; f < sizeof FOLDERS / sizeof FOLDERS[0]; f++) {
        const struct folder *k = &FOLDERS[f];
        u8 g[16] = {(u8)k->a, (u8)(k->a >> 8), (u8)(k->a >> 16), (u8)(k->a >> 24),
                    (u8)k->b, (u8)(k->b >> 8), (u8)k->c, (u8)(k->c >> 8)};
        for (int i = 0; i < 8; i++) g[8 + i] = k->d[i];
        int same = 1;
        for (int i = 0; i < 16; i++) if (g[i] != id[i]) same = 0;
        if (same) return k;
    }
    return 0;
}

/* SHGetKnownFolderPath(rfid, dwFlags, hToken, ppszPath) */
u64 shell32_SHGetKnownFolderPath(u64 id, u64 flags, u64 token, u64 out) {
    if (!id || !out) w2m_unimplemented(W2M_API_SHGetKnownFolderPath, 0);                 /* not in the rows */
    if (token) w2m_unimplemented(W2M_API_SHGetKnownFolderPath, token);                   /* another user's folders */
    if ((u32)flags & ~(KF_FLAG_DONT_VERIFY | KF_FLAG_CREATE)) w2m_unimplemented(W2M_API_SHGetKnownFolderPath, flags);
    const struct folder *k = find((const u8 *)id);
    if (!k) w2m_unimplemented(W2M_API_SHGetKnownFolderPath, *(const u64 *)id);         /* any may be registered */
    u64 n = 0, m = 0;
    while (PROFILE[n]) n++;
    while (k->under[m]) m++;
    u64 block = w2m_heap_take(2 * (n + m + 1));
    if (!block) w2m_unimplemented(W2M_API_SHGetKnownFolderPath, 2 * (n + m + 1));
    u16 *s = (u16 *)block;
    for (u64 i = 0; i < n; i++) s[i] = PROFILE[i];
    for (u64 i = 0; i <= m; i++) s[n + i] = k->under[i];
    u64 st[4];
    if (w2m_file_stat_full(s, (u64)st) != DIRECTORY)       /* the bridge makes them all; a missing one is not in the rows */
        w2m_unimplemented(W2M_API_SHGetKnownFolderPath, m);
    *(u64 *)out = block;
    return w2m_logged(W2M_API_SHGetKnownFolderPath, id, 0);                              /* S_OK */
}

/* CoTaskMemFree(pv): frees a block w2m's services return; NULL is no block; the last error is left (rows free.*). */
W2M_VOID u64 ole32_CoTaskMemFree(u64 p) {
    if (p) w2m_heap_release(W2M_API_CoTaskMemFree, p);
    return w2m_logged(W2M_API_CoTaskMemFree, p, 0);
}
