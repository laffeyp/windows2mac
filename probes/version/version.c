// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* version.c — the version check as the game makes it (Sprint 054, family "version"; specs/work_list.json).
 *
 * No C runtime; linked with the game's own manifest (games/hail-to-the-rainbow/manifest.xml: supportedOS Windows 10,
 * PerMonitorV2), because Windows answers the version questions by the running exe's manifest. The batch trace
 * (store/game-runs/38113342549, seq 1524-1529) holds UnityPlayer's form: five VerSetConditionMask calls (MAJOR,
 * MINOR, SERVICEPACKMAJOR, SERVICEPACKMINOR with VER_GREATER_EQUAL, PLATFORMID with VER_EQUAL; the condition byte
 * comes in a register whose upper bytes are not cleared, 0x23401), then ntdll's RtlVerifyVersionInfo through
 * GetProcAddress with type mask 0x33. SHELL32's DllGetVersion is reached the same way (seq 1660); VerifyVersionInfoW
 * is the play run's (six calls). The rows hold each answer; a build number is kept as relations to the two runner
 * builds (Server 2022: 20348, Server 2025: 26100), so the two runners give one set of rows.
 */
#include <windows.h>

#define SENTINEL 0xDEADu

typedef LONG (WINAPI *verify_fn)(OSVERSIONINFOEXW *, DWORD, ULONGLONG);
typedef struct { DWORD size, major, minor, build, platform; } dll_version;            /* DLLVERSIONINFO */
typedef struct { dll_version info; DWORD flags; ULONGLONG version; } dll_version2;    /* DLLVERSIONINFO2 */
typedef HRESULT (WINAPI *dll_get_version_fn)(void *);

static char out[16384];
static unsigned n;
static void put(const char *s) { while (*s && n < sizeof out - 64) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}
static void build_relations(DWORD build) {
    flag("build_at_least_20348", build >= 20348);
    flag("build_at_most_26100", build <= 26100);
    flag("build_is_a_runner_build", build == 20348 || build == 26100);
}

static verify_fn rtl_verify;

static void zero(void *p, unsigned size) { for (unsigned i = 0; i < size; i++) ((char *)p)[i] = 0; }

/* one question asked both ways: ntdll's RtlVerifyVersionInfo (an NTSTATUS) and kernel32's VerifyVersionInfoW */
static void verify(const char *name, DWORD major, DWORD minor, DWORD build, DWORD platform, WORD sp_major,
                   BYTE product, DWORD type, ULONGLONG mask) {
    OSVERSIONINFOEXW v;
    zero(&v, sizeof v);
    v.dwOSVersionInfoSize = sizeof v;
    v.dwMajorVersion = major; v.dwMinorVersion = minor; v.dwBuildNumber = build; v.dwPlatformId = platform;
    v.wServicePackMajor = sp_major; v.wProductType = product;
    row(name);
    field("status", (DWORD)rtl_verify(&v, type, mask));
    SetLastError(SENTINEL);
    BOOL ok = VerifyVersionInfoW(&v, type, mask);
    field("verify", ok != 0); field("err", GetLastError());
    end();
}

static ULONGLONG cond(DWORD type, BYTE c) { return VerSetConditionMask(0, type, c); }

void start(void) {
    /* VerSetConditionMask: the game's chain, then each condition and type alone */
    ULONGLONG m = VerSetConditionMask(0, VER_MAJORVERSION, VER_GREATER_EQUAL);
    row("mask.game_major"); field("mask", m); end();
    m = VerSetConditionMask(m, VER_MINORVERSION, VER_GREATER_EQUAL);
    row("mask.game_minor"); field("mask", m); end();
    m = VerSetConditionMask(m, VER_SERVICEPACKMAJOR, VER_GREATER_EQUAL);
    row("mask.game_sp_major"); field("mask", m); end();
    m = VerSetConditionMask(m, VER_SERVICEPACKMINOR, VER_GREATER_EQUAL);
    row("mask.game_sp_minor"); field("mask", m); end();
    ULONGLONG game = VerSetConditionMask(m, VER_PLATFORMID, (BYTE)0x01);
    row("mask.game_platform"); field("mask", game); end();
    static const DWORD types[] = {VER_MINORVERSION, VER_MAJORVERSION, VER_BUILDNUMBER, VER_PLATFORMID,
                                  VER_SERVICEPACKMINOR, VER_SERVICEPACKMAJOR, VER_SUITENAME, VER_PRODUCT_TYPE};
    static char name[] = "mask.type_X_condition_Y";
    for (int t = 0; t < 8; t++)
        for (int c = 0; c <= 7; c++) {
            name[10] = (char)('0' + t); name[22] = (char)('0' + c);
            row(name); field("mask", VerSetConditionMask(0, types[t], (BYTE)c)); end();
        }
    row("mask.two_types_at_once"); field("mask", VerSetConditionMask(0, VER_MAJORVERSION | VER_MINORVERSION, VER_EQUAL)); end();
    row("mask.no_type"); field("mask", VerSetConditionMask(0x1234, 0, VER_EQUAL)); end();
    row("mask.replace"); field("mask", VerSetConditionMask(cond(VER_MAJORVERSION, VER_EQUAL), VER_MAJORVERSION, VER_LESS)); end();

    rtl_verify = (verify_fn)GetProcAddress(GetModuleHandleW(L"ntdll.dll"), "RtlVerifyVersionInfo");
    row("rtl_verify.found"); flag("value", rtl_verify != NULL); end();
    ULONGLONG ge_major_minor = VerSetConditionMask(cond(VER_MAJORVERSION, VER_GREATER_EQUAL), VER_MINORVERSION, VER_GREATER_EQUAL);
    verify("verify.game_10_0", 10, 0, 0, 2, 0, 0, 0x33, game);
    verify("verify.game_6_1", 6, 1, 0, 2, 0, 0, 0x33, game);
    verify("verify.game_6_2", 6, 2, 0, 2, 0, 0, 0x33, game);
    verify("verify.game_10_1", 10, 1, 0, 2, 0, 0, 0x33, game);
    verify("verify.game_11_0", 11, 0, 0, 2, 0, 0, 0x33, game);
    verify("verify.game_sp_1", 10, 0, 0, 2, 1, 0, 0x33, game);
    verify("verify.game_with_platform_1", 10, 0, 0, 1, 0, 0, 0x3B, game);
    verify("verify.game_with_platform_2", 10, 0, 0, 2, 0, 0, 0x3B, game);
    verify("verify.ge_10_0", 10, 0, 0, 0, 0, 0, VER_MAJORVERSION | VER_MINORVERSION, ge_major_minor);
    verify("verify.equal_major_10", 10, 0, 0, 0, 0, 0, VER_MAJORVERSION, cond(VER_MAJORVERSION, VER_EQUAL));
    verify("verify.equal_major_6", 6, 0, 0, 0, 0, 0, VER_MAJORVERSION, cond(VER_MAJORVERSION, VER_EQUAL));
    verify("verify.less_major_11", 11, 0, 0, 0, 0, 0, VER_MAJORVERSION, cond(VER_MAJORVERSION, VER_LESS));
    verify("verify.build_ge_20348", 0, 0, 20348, 0, 0, 0, VER_BUILDNUMBER, cond(VER_BUILDNUMBER, VER_GREATER_EQUAL));
    verify("verify.build_ge_99999", 0, 0, 99999, 0, 0, 0, VER_BUILDNUMBER, cond(VER_BUILDNUMBER, VER_GREATER_EQUAL));
    verify("verify.product_server", 0, 0, 0, 0, 0, VER_NT_SERVER, VER_PRODUCT_TYPE, cond(VER_PRODUCT_TYPE, VER_EQUAL));
    verify("verify.product_workstation", 0, 0, 0, 0, 0, VER_NT_WORKSTATION, VER_PRODUCT_TYPE, cond(VER_PRODUCT_TYPE, VER_EQUAL));
    verify("verify.no_type", 10, 0, 0, 2, 0, 0, 0, game);
    verify("verify.type_without_condition", 10, 0, 0, 0, 0, 0, VER_MAJORVERSION, 0);

    /* GetVersionExW under the manifest */
    OSVERSIONINFOEXW v;
    zero(&v, sizeof v);
    v.dwOSVersionInfoSize = sizeof v;
    SetLastError(SENTINEL);
    BOOL ok = GetVersionExW((OSVERSIONINFOW *)&v);
    row("get_version.ex"); field("ret", ok != 0); field("err", GetLastError());
    field("major", v.dwMajorVersion); field("minor", v.dwMinorVersion); build_relations(v.dwBuildNumber);
    field("platform", v.dwPlatformId); field("csd_first", v.szCSDVersion[0]); field("sp_major", v.wServicePackMajor);
    field("sp_minor", v.wServicePackMinor); field("suite", v.wSuiteMask); field("product", v.wProductType);
    end();
    OSVERSIONINFOW w;
    zero(&w, sizeof w);
    w.dwOSVersionInfoSize = sizeof w;
    ok = GetVersionExW(&w);
    row("get_version.plain"); field("ret", ok != 0); field("major", w.dwMajorVersion); field("minor", w.dwMinorVersion);
    build_relations(w.dwBuildNumber); end();

    /* SHELL32's DllGetVersion, through LoadLibraryW and GetProcAddress as UnityPlayer reaches it */
    HMODULE shell = LoadLibraryW(L"shell32.dll");
    dll_get_version_fn get = shell ? (dll_get_version_fn)GetProcAddress(shell, "DllGetVersion") : NULL;
    row("dll_get_version.found"); flag("library", shell != NULL); flag("function", get != NULL); end();
    if (get) {
        dll_version d;
        zero(&d, sizeof d);
        d.size = sizeof d;
        HRESULT hr = get(&d);
        row("dll_get_version.info"); field("hr", (DWORD)hr); field("size", d.size); field("major", d.major);
        field("minor", d.minor); build_relations(d.build); field("platform", d.platform); end();
        dll_version2 d2;
        zero(&d2, sizeof d2);
        d2.info.size = sizeof d2;
        hr = get(&d2);
        row("dll_get_version.info2"); field("hr", (DWORD)hr); field("major", d2.info.major); field("minor", d2.info.minor);
        build_relations(d2.info.build); field("platform", d2.info.platform); field("flags", d2.flags);
        field("version_major", d2.version >> 48); field("version_minor", d2.version >> 32 & 0xFFFF);
        flag("version_build_is_info_build", (d2.version >> 16 & 0xFFFF) == d2.info.build); end();
        dll_version bad;
        zero(&bad, sizeof bad);
        bad.size = 12;
        hr = get(&bad);
        row("dll_get_version.short_size"); field("hr", (DWORD)hr); field("major", bad.major); end();
        row("dll_get_version.free"); flag("ret", FreeLibrary(shell) != 0); end();
    }
    ExitProcess(0);
}
