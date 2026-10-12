// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* system_info.c — what the process learns about the machine (Sprint 046, family "system information").
 *
 * GetNativeSystemInfo, GetSystemInfo, GetVersionExW, GlobalMemoryStatusEx, GetTickCount and
 * GetSystemDirectoryW, as contracts/windows.system.information records Windows answering on twelve
 * runners (24 identical outputs):
 *   - SYSTEM_INFO: architecture 9 (x64), page 4096, addresses 0x10000 to 0x7FFFFFFEFFFF, type 8664,
 *     granularity 65536, level and revision from CPUID (family; model and stepping), the processor count
 *     equal to the mask's bits; GetSystemInfo equals GetNativeSystemInfo; the last error is left
 *   - GetVersionExW without a manifest: 6.2 build 9200 (with one naming Windows 10: 10.0, the runners' build, suite
 *     mask 0x190; contracts/windows.version.version, Sprint 054), platform 2, no service pack, product type 3 (the
 *     runners are Server; a desktop says 1); a size that is neither structure's: FALSE, ERROR_INSUFFICIENT_BUFFER
 *   - GlobalMemoryStatusEx: dwLength 64 or FALSE with ERROR_INVALID_PARAMETER; total virtual
 *     0x7FFFFFFE0000, extended 0; the rest as relations
 *   - GetTickCount advances in steps under 100 ms; GetSystemDirectoryW "C:\Windows\system32", and a short
 *     buffer gets the size it needs, NUL included, with the last error left
 * w2m's choices where the rows give relations: the runners' four processors (Sprint 061), 16 GiB of memory, the
 * tick count from the guest counter. Level and revision come from the guest CPUID profile.
 */
#include "runtime.h"

typedef unsigned short u16;

#define ERROR_INSUFFICIENT_BUFFER 122u
#define ERROR_INVALID_PARAMETER 87u
#define GIB (1ull << 30)

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

static void system_info(u64 si) {
    u32 r[4];
    w2m_cpuid_query(1, 0, r);
    u32 family = (r[0] >> 8) & 0xF, model = (r[0] >> 4) & 0xF, stepping = r[0] & 0xF;
    if (family == 0xF) family += (r[0] >> 20) & 0xFF;
    if (family == 0x6 || family >= 0xF) model |= ((r[0] >> 16) & 0xF) << 4;
    u8 *p = (u8 *)si;
    *(u16 *)(p + 0) = 9;                              /* PROCESSOR_ARCHITECTURE_AMD64 */
    *(u16 *)(p + 2) = 0;
    *(u32 *)(p + 4) = 4096;
    *(u64 *)(p + 8) = 0x10000;
    *(u64 *)(p + 16) = 0x7FFFFFFEFFFFull;
    *(u64 *)(p + 24) = W2M_PROCESSOR_MASK;            /* active processor mask: the runners' four (runtime.h) */
    *(u32 *)(p + 32) = W2M_PROCESSORS;
    *(u32 *)(p + 36) = 8664;                          /* PROCESSOR_AMD_X8664 */
    *(u32 *)(p + 40) = 65536;
    *(u16 *)(p + 44) = (u16)family;
    *(u16 *)(p + 46) = (u16)(model << 8 | stepping);
}

u64 k32_GetNativeSystemInfo(u64 si) { system_info(si); return w2m_logged(W2M_API_GetNativeSystemInfo, si, 0); }
u64 k32_GetSystemInfo(u64 si) { system_info(si); return w2m_logged(W2M_API_GetSystemInfo, si, 0); }

/* OSVERSIONINFOW is 276 bytes, OSVERSIONINFOEXW 284. */
u64 k32_GetVersionExW(u64 vi) {
    u8 *p = (u8 *)vi;
    u32 size = *(u32 *)p;
    if (size != 276 && size != 284) {
        set_error(ERROR_INSUFFICIENT_BUFFER);
        return w2m_logged(W2M_API_GetVersionExW, size, 0);
    }
    int windows_10 = (w2m_manifest() & W2M_MANIFEST_WINDOWS_10) != 0;   /* contracts windows.version.version (Sprint 054) */
    *(u32 *)(p + 4) = windows_10 ? 10 : 6;
    *(u32 *)(p + 8) = windows_10 ? 0 : 2;
    *(u32 *)(p + 12) = windows_10 ? 26100 : 9200;
    *(u32 *)(p + 16) = 2;                             /* VER_PLATFORM_WIN32_NT */
    for (int i = 0; i < 128; i++) *(u16 *)(p + 20 + 2 * i) = 0;   /* szCSDVersion: empty */
    if (size == 284) {
        *(u16 *)(p + 276) = 0;                        /* service pack major, minor */
        *(u16 *)(p + 278) = 0;
        *(u16 *)(p + 280) = 0x190;                    /* suite mask: the runners' (row get_version.ex) */
        p[282] = 3;                                   /* VER_NT_SERVER, as the runners answer */
        p[283] = 0;
    }
    return w2m_logged(W2M_API_GetVersionExW, size, 1);
}

u64 k32_GlobalMemoryStatusEx(u64 ms) {
    u8 *p = (u8 *)ms;
    if (*(u32 *)p != 64) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_GlobalMemoryStatusEx, *(u32 *)p, 0);
    }
    *(u32 *)(p + 4) = 25;                             /* dwMemoryLoad */
    *(u64 *)(p + 8) = 16 * GIB;                       /* ullTotalPhys */
    *(u64 *)(p + 16) = 12 * GIB;                      /* ullAvailPhys */
    *(u64 *)(p + 24) = 32 * GIB;                      /* ullTotalPageFile */
    *(u64 *)(p + 32) = 28 * GIB;                      /* ullAvailPageFile */
    *(u64 *)(p + 40) = 0x7FFFFFFE0000ull;             /* ullTotalVirtual, as every runner */
    *(u64 *)(p + 48) = 0x7FFFFFFE0000ull - GIB;       /* ullAvailVirtual */
    *(u64 *)(p + 56) = 0;                             /* ullAvailExtendedVirtual */
    return w2m_logged(W2M_API_GlobalMemoryStatusEx, 64, 1);
}

u64 k32_GetTickCount(void) {
    return w2m_logged(W2M_API_GetTickCount, 0, (u32)(w2m_counter_now() / 10000));   /* ms of the 10 MHz counter */
}

/* GetWindowsDirectoryW (contracts/windows.file.files_more, Sprint 049): C:\Windows, 10; a buffer under 11 units gets
 * nothing and the answer 11; the last error is left. */
u64 k32_GetWindowsDirectoryW(u64 buffer, u64 size) {
    static const u16 dir[] = {'C', ':', '\\', 'W', 'i', 'n', 'd', 'o', 'w', 's', 0};
    u32 n = 10;
    if ((u32)size < n + 1) return w2m_logged(W2M_API_GetWindowsDirectoryW, (u32)size, n + 1);
    u16 *out = (u16 *)buffer;
    for (u32 i = 0; i <= n; i++) out[i] = dir[i];
    return w2m_logged(W2M_API_GetWindowsDirectoryW, (u32)size, n);
}

u64 k32_GetSystemDirectoryW(u64 buffer, u64 size) {
    static const u16 dir[] = {'C', ':', '\\', 'W', 'i', 'n', 'd', 'o', 'w', 's', '\\', 's', 'y', 's', 't', 'e', 'm', '3', '2', 0};
    u32 n = 19;
    if ((u32)size < n + 1) return w2m_logged(W2M_API_GetSystemDirectoryW, (u32)size, n + 1);
    u16 *out = (u16 *)buffer;
    for (u32 i = 0; i <= n; i++) out[i] = dir[i];
    return w2m_logged(W2M_API_GetSystemDirectoryW, (u32)size, n);
}
