// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* system_info.c — what Windows says about the machine (Sprint 046, family "system information").
 *
 * No C runtime. UnityPlayer.dll's start-up stops at GetNativeSystemInfo (frontier 2026-10-10, 156,277
 * instructions); the family is GetNativeSystemInfo, GetSystemInfo, GetVersionExW, GlobalMemoryStatusEx,
 * GetTickCount and GetSystemDirectoryW, all UnityPlayer imports. Values a machine decides are relations:
 * processor level and revision against this CPU's CPUID (family; model and stepping), the processor count
 * against the active mask, memory amounts as orderings, the tick count against the system time.
 */
#include <windows.h>

#define SENTINEL 0xDEADu

/* GetVersionExW is deprecated in the Windows SDK (not in mingw's headers); UnityPlayer calls it, so the probe does */
#pragma clang diagnostic ignored "-Wdeprecated-declarations"

static char out[8192];
static unsigned n;

static void put(const char *s) { while (*s) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void end(void) { put("}\n"); }

static void cpuid(unsigned leaf, unsigned r[4]) {
    __asm__ volatile("cpuid" : "=a"(r[0]), "=b"(r[1]), "=c"(r[2]), "=d"(r[3]) : "a"(leaf), "c"(0));
}

static int popcount(unsigned long long v) { int c = 0; while (v) { c += (int)(v & 1); v >>= 1; } return c; }

static void info(const char *name, void (*get)(LPSYSTEM_INFO)) {
    SYSTEM_INFO si;
    unsigned char *b = (unsigned char *)&si;
    for (unsigned i = 0; i < sizeof si; i++) b[i] = 0xA5;
    SetLastError(SENTINEL);
    get(&si);
    DWORD err = GetLastError();
    unsigned r[4];
    cpuid(1, r);
    unsigned family = (r[0] >> 8) & 0xF, model = (r[0] >> 4) & 0xF, stepping = r[0] & 0xF;
    if (family == 0xF) family += (r[0] >> 20) & 0xFF;
    if (family == 0x6 || family >= 0xF) model |= ((r[0] >> 16) & 0xF) << 4;
    row(name); field("err", err); field("architecture", si.wProcessorArchitecture); field("reserved", si.wReserved);
    field("page_size", si.dwPageSize); field("minimum_address", (unsigned long long)si.lpMinimumApplicationAddress);
    field("maximum_address", (unsigned long long)si.lpMaximumApplicationAddress);
    flag("processors_match_mask", (int)si.dwNumberOfProcessors == popcount(si.dwActiveProcessorMask));
    flag("processors_positive", si.dwNumberOfProcessors > 0);
    field("processor_type", si.dwProcessorType); field("allocation_granularity", si.dwAllocationGranularity);
    flag("level_is_cpuid_family", si.wProcessorLevel == family);
    flag("revision_is_cpuid_model_stepping", si.wProcessorRevision == (model << 8 | stepping));
    end();
}

void start(void) {
    info("native_system_info", GetNativeSystemInfo);
    info("system_info", GetSystemInfo);
    SYSTEM_INFO a, b;
    GetNativeSystemInfo(&a);
    GetSystemInfo(&b);
    unsigned char *pa = (unsigned char *)&a, *pb = (unsigned char *)&b;
    int same = 1;
    for (unsigned i = 0; i < sizeof a; i++) same &= pa[i] == pb[i];
    row("system_info.same_as_native"); flag("value", same); end();

    OSVERSIONINFOEXW v;
    unsigned char *pv = (unsigned char *)&v;
    for (unsigned i = 0; i < sizeof v; i++) pv[i] = 0xA5;
    v.dwOSVersionInfoSize = sizeof v;
    SetLastError(SENTINEL);
    BOOL ok = GetVersionExW((LPOSVERSIONINFOW)&v);
    DWORD err = GetLastError();
    row("version_ex"); field("ret", ok != 0); field("err", err); field("major", v.dwMajorVersion);
    field("minor", v.dwMinorVersion); field("build", v.dwBuildNumber); field("platform", v.dwPlatformId);
    field("service_pack_major", v.wServicePackMajor); field("product_type", v.wProductType);
    flag("csd_empty", v.szCSDVersion[0] == 0); end();
    v.dwOSVersionInfoSize = 12;
    SetLastError(SENTINEL);
    ok = GetVersionExW((LPOSVERSIONINFOW)&v);
    err = GetLastError();
    row("version_ex.bad_size"); field("ret", ok != 0); field("err", err); end();

    MEMORYSTATUSEX m;
    unsigned char *pm = (unsigned char *)&m;
    for (unsigned i = 0; i < sizeof m; i++) pm[i] = 0xA5;
    m.dwLength = sizeof m;
    SetLastError(SENTINEL);
    ok = GlobalMemoryStatusEx(&m);
    err = GetLastError();
    row("memory_status_ex"); field("ret", ok != 0); field("err", err); field("length", m.dwLength);
    flag("load_percent", m.dwMemoryLoad <= 100); flag("avail_phys_le_total", m.ullAvailPhys <= m.ullTotalPhys);
    flag("total_phys_ge_1gb", m.ullTotalPhys >= (1ull << 30)); flag("avail_virtual_le_total", m.ullAvailVirtual <= m.ullTotalVirtual);
    field("total_virtual", m.ullTotalVirtual); field("avail_extended_virtual", m.ullAvailExtendedVirtual); end();
    m.dwLength = 0;
    SetLastError(SENTINEL);
    ok = GlobalMemoryStatusEx(&m);
    err = GetLastError();
    row("memory_status_ex.bad_length"); field("ret", ok != 0); field("err", err); end();

    SetLastError(SENTINEL);
    DWORD t0 = GetTickCount();
    err = GetLastError();
    FILETIME f0, f1;
    GetSystemTimeAsFileTime(&f0);
    DWORD t1 = t0;
    while (t1 == t0) t1 = GetTickCount();
    GetSystemTimeAsFileTime(&f1);
    unsigned long long elapsed = ((unsigned long long)f1.dwHighDateTime << 32 | f1.dwLowDateTime) -
                                 ((unsigned long long)f0.dwHighDateTime << 32 | f0.dwLowDateTime);
    row("tick_count"); field("err", err); flag("advances", t1 != t0); flag("step_under_100ms", t1 - t0 < 100);
    flag("system_time_under_100ms", elapsed < 1000000ull); end();

    WCHAR dir[MAX_PATH];
    SetLastError(SENTINEL);
    UINT len = GetSystemDirectoryW(dir, MAX_PATH);
    err = GetLastError();
    const WCHAR want[] = L"C:\\Windows\\system32";
    int equal = len == 19;
    for (UINT i = 0; equal && i < len; i++) equal = dir[i] == want[i];
    row("system_directory"); field("length", len); field("err", err); flag("is_c_windows_system32", equal); end();
    SetLastError(SENTINEL);
    len = GetSystemDirectoryW(dir, 5);
    err = GetLastError();
    row("system_directory.small"); field("ret", len); field("err", err); end();

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
