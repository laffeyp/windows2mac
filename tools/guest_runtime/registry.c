// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* registry.c — opening and closing registry keys (Sprint 054, family "registry"; specs/work_list.json).
 *
 * contracts/windows.registry.registry records Windows (Server 2025 and 2022, equal):
 *   - RegOpenKeyExW of a key that does not exist answers ERROR_FILE_NOT_FOUND and writes NULL to the result, one
 *     level or two below HKEY_CURRENT_USER\Software, by the root or by a handle to Software; names compare without
 *     case
 *   - Software opens (ERROR_SUCCESS, a new handle); a NULL or empty subkey gives the root's own predefined handle
 *   - a handle that is no key answers ERROR_INVALID_HANDLE with NULL written
 *   - RegCloseKey closes a key once (ERROR_SUCCESS, then ERROR_INVALID_HANDLE), closes a predefined root with
 *     ERROR_SUCCESS, and answers ERROR_INVALID_HANDLE for NULL or a handle that is no key
 *   - neither function touches the last error
 * w2m's registry holds HKEY_CURRENT_USER and its Software key, empty: the game's keys do not exist on a fresh
 * machine (batch trace seq 1653). Other roots, options, and writes (RegCreateKeyW and the value functions, first at
 * seq 2661) are not in the rows and stop the run.
 */
#include "runtime.h"

typedef unsigned short u16;

#define ERROR_SUCCESS 0u
#define ERROR_FILE_NOT_FOUND 2u
#define ERROR_INVALID_HANDLE 6u
#define HKEY_CURRENT_USER 0xFFFFFFFF80000001ull       /* (HKEY)(ULONG_PTR)(LONG)0x80000001, sign-extended */
#define KEY_BASE 0x3000ull                           /* key handles: KEY_BASE + 4*slot */
#define KEYS 64
#define PATH 256

/* The keys that exist, as paths below HKEY_CURRENT_USER in lower case ("" is the root). */
static const char *const existing[] = {"", "software"};

static struct { int open; char path[PATH]; } keys[KEYS];

static char lower(u16 c) { return c >= 'A' && c <= 'Z' ? (char)(c + 32) : (char)c; }

/* path of handle h into out; 0: h names no open key */
static int path_of(u64 h, char *out) {
    if (h == HKEY_CURRENT_USER) { out[0] = 0; return 1; }
    if (h < KEY_BASE || (h - KEY_BASE) % 4 || (h - KEY_BASE) / 4 >= KEYS || !keys[(h - KEY_BASE) / 4].open) return 0;
    const char *p = keys[(h - KEY_BASE) / 4].path;
    int i = 0;
    while ((out[i] = p[i])) i++;
    return 1;
}

static int exists(const char *path) {
    for (unsigned k = 0; k < sizeof existing / sizeof *existing; k++) {
        const char *a = existing[k], *b = path;
        while (*a && *a == *b) a++, b++;
        if (!*a && !*b) return 1;
    }
    return 0;
}

/* RegOpenKeyExW(hKey, lpSubKey, ulOptions, samDesired, phkResult) */
u64 advapi32_RegOpenKeyExW(u64 root, u64 sub, u64 options, u64 access, u64 rsp) {
    (void)access;
    u64 result = *(const u64 *)(rsp + 0x28);
    if ((u32)options) w2m_unimplemented(W2M_API_RegOpenKeyExW, options);        /* REG_OPTION_OPEN_LINK: not in the rows */
    if (root >= 0xFFFFFFFF80000000ull && root != HKEY_CURRENT_USER) w2m_unimplemented(W2M_API_RegOpenKeyExW, root);
    char path[PATH];
    if (!path_of(root, path)) {
        *(u64 *)result = 0;
        return w2m_logged(W2M_API_RegOpenKeyExW, root, ERROR_INVALID_HANDLE);
    }
    const u16 *s = (const u16 *)sub;
    if (!s || !*s) {
        if (root != HKEY_CURRENT_USER) w2m_unimplemented(W2M_API_RegOpenKeyExW, root);   /* a key's own: not in the rows */
        *(u64 *)result = root;
        return w2m_logged(W2M_API_RegOpenKeyExW, root, ERROR_SUCCESS);
    }
    int n = 0;
    while (path[n]) n++;
    if (*s == '\\') w2m_unimplemented(W2M_API_RegOpenKeyExW, sub);                  /* a leading separator: not in the rows */
    if (n) path[n++] = '\\';
    for (; *s; s++) {
        if (n >= PATH - 1) w2m_unimplemented(W2M_API_RegOpenKeyExW, PATH);
        path[n++] = lower(*s);
    }
    path[n] = 0;
    if (!exists(path)) {
        *(u64 *)result = 0;
        return w2m_logged(W2M_API_RegOpenKeyExW, root, ERROR_FILE_NOT_FOUND);
    }
    for (int k = 0; k < KEYS; k++)
        if (!keys[k].open) {
            keys[k].open = 1;
            for (int i = 0; (keys[k].path[i] = path[i]); i++) {}
            *(u64 *)result = KEY_BASE + 4 * (u64)k;
            return w2m_logged(W2M_API_RegOpenKeyExW, root, ERROR_SUCCESS);
        }
    w2m_unimplemented(W2M_API_RegOpenKeyExW, KEYS);
    return 0;
}

u64 advapi32_RegCloseKey(u64 h) {
    if (h == HKEY_CURRENT_USER) return w2m_logged(W2M_API_RegCloseKey, h, ERROR_SUCCESS);
    char path[PATH];
    if (!path_of(h, path)) return w2m_logged(W2M_API_RegCloseKey, h, ERROR_INVALID_HANDLE);
    keys[(h - KEY_BASE) / 4].open = 0;
    return w2m_logged(W2M_API_RegCloseKey, h, ERROR_SUCCESS);
}
