// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* version.c — the version check (Sprint 054, family "version"; specs/work_list.json).
 *
 * contracts/windows.version.version records Windows (Server 2025 and 2022, equal) under the game's manifest:
 *   - VerSetConditionMask keeps three bits per type, MINOR at bit 0, MAJOR 3, BUILDNUMBER 6, PLATFORMID 9,
 *     SERVICEPACKMINOR 12, SERVICEPACKMAJOR 15, SUITENAME 18, PRODUCT_TYPE 21, ORed into the mask given, and sets
 *     bit 63; with several types only the highest takes the condition; a type mask of 0 answers 0
 *   - RtlVerifyVersionInfo (ntdll) compares major, minor and the service pack as one version, by the condition of the
 *     highest of them in the type mask ("at least 6.2" holds on 10.0), and build, platform and product type each by
 *     its own; a type whose condition is 0 is not compared; STATUS_SUCCESS, STATUS_REVISION_MISMATCH (0xC0000059),
 *     or STATUS_INVALID_PARAMETER (0xC000000D) for a type mask of 0
 *   - VerifyVersionInfoW answers the same as TRUE, or FALSE with ERROR_OLD_WIN_VERSION (1150) or ERROR_BAD_ARGUMENTS
 *     (160); TRUE leaves the last error
 * The version is w2m's profile, Windows Server 2025: 10.0 build 26100, no service pack, platform 2, product type 3
 * (server), suite mask 0x190. ntdll answers it to every program; kernel32 tells it only to a program whose manifest
 * names Windows 10 (manifest.c), the rest 6.2 build 9200, whose VerifyVersionInfoW no row records (it stops the run).
 * The suite conditions (VER_AND, VER_OR) are not in the rows and stop the run.
 */
#include "runtime.h"

typedef unsigned short u16;

#define VER_MINORVERSION 0x1u
#define VER_MAJORVERSION 0x2u
#define VER_BUILDNUMBER 0x4u
#define VER_PLATFORMID 0x8u
#define VER_SERVICEPACKMINOR 0x10u
#define VER_SERVICEPACKMAJOR 0x20u
#define VER_SUITENAME 0x40u
#define VER_PRODUCT_TYPE 0x80u
#define VER_EQUAL 1u
#define VER_GREATER 2u
#define VER_GREATER_EQUAL 3u
#define VER_LESS 4u
#define VER_LESS_EQUAL 5u
#define STATUS_SUCCESS 0u
#define STATUS_INVALID_PARAMETER 0xC000000Du
#define STATUS_REVISION_MISMATCH 0xC0000059u
#define ERROR_BAD_ARGUMENTS 160u
#define ERROR_OLD_WIN_VERSION 1150u

/* w2m's Windows: Server 2025 (the runners' profile, build 26100) */
#define MAJOR 10u
#define MINOR 0u
#define BUILD 26100u
#define PLATFORM 2u
#define PRODUCT_TYPE 3u

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

/* the bit position of each type's three condition bits, highest type first */
static const struct { u32 type, shift; } FIELDS[] = {
    {VER_PRODUCT_TYPE, 21}, {VER_SUITENAME, 18}, {VER_SERVICEPACKMAJOR, 15}, {VER_SERVICEPACKMINOR, 12},
    {VER_PLATFORMID, 9}, {VER_BUILDNUMBER, 6}, {VER_MAJORVERSION, 3}, {VER_MINORVERSION, 0}};

u64 k32_VerSetConditionMask(u64 mask, u64 types, u64 condition) {
    u32 t = (u32)types, c = (u8)condition;
    if (c > 7) w2m_unimplemented(W2M_API_VerSetConditionMask, c);                   /* not in the rows */
    if (!t) return w2m_logged(W2M_API_VerSetConditionMask, t, 0);
    for (unsigned i = 0; i < sizeof FIELDS / sizeof *FIELDS; i++)
        if (t & FIELDS[i].type) {
            mask |= (u64)c << FIELDS[i].shift;
            break;
        }
    return w2m_logged(W2M_API_VerSetConditionMask, t, mask | 1ull << 63);
}

static u32 condition_of(u64 mask, u32 type) {
    for (unsigned i = 0; i < sizeof FIELDS / sizeof *FIELDS; i++)
        if (FIELDS[i].type == type) return (u32)(mask >> FIELDS[i].shift) & 7;
    return 0;
}

static int holds(u32 have, u32 want, u32 c, u64 api) {
    switch (c) {
    case VER_EQUAL: return have == want;
    case VER_GREATER: return have > want;
    case VER_GREATER_EQUAL: return have >= want;
    case VER_LESS: return have < want;
    case VER_LESS_EQUAL: return have <= want;
    default: w2m_unimplemented(api, c);                                             /* VER_AND, VER_OR */
    }
    return 0;
}

/* OSVERSIONINFOEXW against the version (major, minor, build), as RtlVerifyVersionInfo answers. */
static u32 verify(u64 api, const u8 *info, u32 types, u64 mask, u32 major, u32 minor, u32 build) {
    if (!types) return STATUS_INVALID_PARAMETER;
    if (types & ~0xFFu || types & VER_SUITENAME) w2m_unimplemented(api, types);    /* not in the rows */
    const u32 *v = (const u32 *)info;                     /* size, major, minor, build, platform */
    const u16 *sp = (const u16 *)(info + 276);            /* service pack major, minor; suite mask; product type */
    u32 product = info[282];
    /* major, minor, service pack major and minor as one version, by the condition of the highest present */
    static const u32 order[] = {VER_MAJORVERSION, VER_MINORVERSION, VER_SERVICEPACKMAJOR, VER_SERVICEPACKMINOR};
    u32 have[] = {major, minor, 0, 0}, want[] = {v[1], v[2], sp[0], sp[1]};
    u32 c = 0;
    for (int i = 0; i < 4 && !c; i++)
        if (types & order[i]) c = condition_of(mask, order[i]);
    if (c) {
        int decided = 0, ok = 0;
        for (int i = 0; i < 4 && !decided; i++)
            if (types & order[i] && have[i] != want[i]) {
                ok = holds(have[i], want[i], c, api);
                decided = 1;
            }
        if (!decided) ok = holds(0, 0, c, api);
        if (!ok) return STATUS_REVISION_MISMATCH;
    }
    u32 c_build = condition_of(mask, VER_BUILDNUMBER), c_platform = condition_of(mask, VER_PLATFORMID);
    u32 c_product = condition_of(mask, VER_PRODUCT_TYPE);
    if (types & VER_BUILDNUMBER && c_build && !holds(build, v[3], c_build, api)) return STATUS_REVISION_MISMATCH;
    if (types & VER_PLATFORMID && c_platform && !holds(PLATFORM, v[4], c_platform, api)) return STATUS_REVISION_MISMATCH;
    if (types & VER_PRODUCT_TYPE && c_product && !holds(PRODUCT_TYPE, product, c_product, api))
        return STATUS_REVISION_MISMATCH;
    return STATUS_SUCCESS;
}

/* RtlVerifyVersionInfo(VersionInfo, TypeMask, ConditionMask): ntdll's, the version itself */
u64 ntdll_RtlVerifyVersionInfo(u64 info, u64 types, u64 mask) {
    u32 status = verify(W2M_API_RtlVerifyVersionInfo, (const u8 *)info, (u32)types, mask, MAJOR, MINOR, BUILD);
    return w2m_logged(W2M_API_RtlVerifyVersionInfo, (u32)types, status);
}

/* VerifyVersionInfoW(lpVersionInformation, dwTypeMask, dwlConditionMask): kernel32's, the manifest's version */
u64 k32_VerifyVersionInfoW(u64 info, u64 types, u64 mask) {
    if (!(w2m_manifest() & W2M_MANIFEST_WINDOWS_10)) w2m_unimplemented(W2M_API_VerifyVersionInfoW, 0);   /* 6.2: no row */
    u32 status = verify(W2M_API_VerifyVersionInfoW, (const u8 *)info, (u32)types, mask, MAJOR, MINOR, BUILD);
    if (status == STATUS_SUCCESS) return w2m_logged(W2M_API_VerifyVersionInfoW, (u32)types, 1);
    set_error(status == STATUS_INVALID_PARAMETER ? ERROR_BAD_ARGUMENTS : ERROR_OLD_WIN_VERSION);
    return w2m_logged(W2M_API_VerifyVersionInfoW, (u32)types, 0);
}

/* SHELL32's DllGetVersion(pdvi), reached through GetProcAddress: DLLVERSIONINFO (20 bytes) or DLLVERSIONINFO2 (32)
 * with 10.0 build 10586 on both runners (shell32's own number, not the system's); DLLVERSIONINFO2's ullVersion ends
 * in shell32's file revision, which differs by image (33438 on Server 2025, w2m's profile; 5499 on 2022); another
 * size is E_INVALIDARG with nothing written. */
#define SHELL32_BUILD 10586u
#define SHELL32_REVISION 33438u
#define E_INVALIDARG 0x80070057u
u64 shell32_DllGetVersion(u64 info) {
    u32 *d = (u32 *)info;
    if (d[0] != 20 && d[0] != 32) return w2m_logged(W2M_API_DllGetVersion, d[0], E_INVALIDARG);
    d[1] = 10; d[2] = 0; d[3] = SHELL32_BUILD; d[4] = PLATFORM;
    if (d[0] == 32) {
        d[5] = 0;                                                   /* dwFlags */
        *(u64 *)(info + 24) = 10ull << 48 | (u64)SHELL32_BUILD << 16 | SHELL32_REVISION;
    }
    return w2m_logged(W2M_API_DllGetVersion, d[0], 0);
}
