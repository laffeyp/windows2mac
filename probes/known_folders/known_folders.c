// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* known_folders.c — SHGetKnownFolderPath and CoTaskMemFree (Sprint 048, family "known folders").
 *
 * No C runtime. UnityPlayer.dll gets SHGetKnownFolderPath from SHELL32 by GetProcAddress and calls it with
 * FOLDERID_LocalAppDataLow, flags 0 and no token (look-ahead 2026-10-10, 2,594,701 instructions); it imports
 * ole32's CoTaskMemFree, which frees the path. This probe makes that call, the same for three other per-user
 * folders, LocalLow with KF_FLAG_DONT_VERIFY and KF_FLAG_CREATE, and an unregistered GUID.
 * A runner's user is not w2m's, so every path printed has the profile folder's last component replaced by
 * "<user>" and every path under the profile begins "<profile>"; the rows hold on any machine.
 * Rows: the HRESULT, whether *ppszPath is NULL (it is set to a marker first), the path, whether it names an
 * existing directory, and the last error (after SetLastError(0xDEAD)).
 */
#include <windows.h>
#include <shlobj.h>

#define SENTINEL 0xDEADu

static char out[8192];
static unsigned n;
static WCHAR profile[MAX_PATH];
static unsigned profile_len, user_at;          /* user_at: where the profile's last component begins */

/* KNOWNFOLDERID values from Microsoft Learn's KNOWNFOLDERID page (updated 2023-08-30), spelled out so the
 * probe links without uuid.lib. */
static const GUID LOCAL_LOW = {0xA520A1A4, 0x1780, 0x4FF6, {0xBD, 0x18, 0x16, 0x73, 0x43, 0xC5, 0xAF, 0x16}};
static const GUID LOCAL = {0xF1B32785, 0x6FBA, 0x4FCF, {0x9D, 0x55, 0x7B, 0x8E, 0x7F, 0x15, 0x70, 0x91}};
static const GUID ROAMING = {0x3EB685DB, 0x65F9, 0x4CF6, {0xA0, 0x3A, 0xE3, 0xEF, 0x65, 0x72, 0x9F, 0x3D}};
static const GUID PROFILE = {0x5E6C858F, 0x0E22, 0x4760, {0x9A, 0xFE, 0xEA, 0x33, 0x17, 0xB6, 0x71, 0x73}};
static const GUID UNREGISTERED = {0x7732D2E1, 0x7E9A, 0x4C3B, {0x9D, 0x6E, 0x10, 0x21, 0x32, 0x43, 0x54, 0x65}};

static void put(const char *s) { while (*s) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void puth(unsigned long v) {
    put("\"0x");
    for (int s = 28; s >= 0; s -= 4) out[n++] = "0123456789abcdef"[(v >> s) & 15];
    put("\"");
}
static void raw(const WCHAR *s, unsigned len) {
    for (unsigned i = 0; i < len && s[i]; i++) {
        WCHAR c = s[i];
        if (c == '"' || c == '\\') { out[n++] = '\\'; out[n++] = (char)c; }
        else if (c >= 0x20 && c < 0x7F) out[n++] = (char)c;
        else put("?");
    }
}
static int same(const WCHAR *a, const WCHAR *b, unsigned len) {
    for (unsigned i = 0; i < len; i++) if (a[i] != b[i]) return 0;
    return 1;
}
/* A path as a JSON string: the profile becomes "<profile>", and the profile itself shows its user as "<user>". */
static void path(const WCHAR *s) {
    unsigned len = 0;
    while (s[len]) len++;
    out[n++] = '"';
    if (len == profile_len && same(s, profile, len)) { raw(s, user_at); put("<user>"); }
    else if (len > profile_len && same(s, profile, profile_len) && s[profile_len] == '\\') {
        put("<profile>"); raw(s + profile_len, len - profile_len);
    }
    else raw(s, len);
    out[n++] = '"';
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void end(void) { put("}\n"); }

static void known(const char *name, const GUID *id, DWORD flags) {
    PWSTR p = (PWSTR)1;                         /* a marker: the call's answer replaces it, or not */
    SetLastError(SENTINEL);
    HRESULT hr = SHGetKnownFolderPath(id, flags, NULL, &p);
    DWORD err = GetLastError();
    row(name); put(",\"hr\":"); puth((unsigned long)hr); field("err", err);
    field("untouched", p == (PWSTR)1); field("null", p == NULL);
    if (p && p != (PWSTR)1) {
        DWORD a = GetFileAttributesW(p);
        put(",\"path\":"); path(p);
        field("directory", a != INVALID_FILE_ATTRIBUTES && (a & FILE_ATTRIBUTE_DIRECTORY));
    }
    end();
    if (p != (PWSTR)1) CoTaskMemFree(p);
}

void start(void) {
    PWSTR p = NULL;
    if (SHGetKnownFolderPath(&PROFILE, 0, NULL, &p) != S_OK || !p) ExitProcess(2);
    while (p[profile_len]) { profile[profile_len] = p[profile_len]; profile_len++; }
    for (unsigned i = 0; i < profile_len; i++) if (profile[i] == '\\') user_at = i + 1;
    CoTaskMemFree(p);

    known("profile", &PROFILE, 0);
    known("local_low", &LOCAL_LOW, 0);           /* UnityPlayer's call */
    known("local", &LOCAL, 0);
    known("roaming", &ROAMING, 0);
    known("local_low.dont_verify", &LOCAL_LOW, KF_FLAG_DONT_VERIFY);
    known("local_low.create", &LOCAL_LOW, KF_FLAG_CREATE);
    known("unregistered", &UNREGISTERED, 0);
    known("unregistered.create", &UNREGISTERED, KF_FLAG_CREATE);

    HMODULE shell32 = GetModuleHandleW(L"shell32.dll");     /* UnityPlayer's route to it */
    FARPROC proc = GetProcAddress(shell32, "SHGetKnownFolderPath");
    row("proc"); field("is_import", proc == (FARPROC)SHGetKnownFolderPath); end();

    SetLastError(SENTINEL);
    CoTaskMemFree(NULL);
    row("free.null"); field("err", GetLastError()); end();
    PWSTR q = NULL;
    SHGetKnownFolderPath(&LOCAL_LOW, 0, NULL, &q);
    SetLastError(SENTINEL);
    CoTaskMemFree(q);
    row("free.path"); field("err", GetLastError()); end();

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
