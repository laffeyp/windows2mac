// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* registry.c — registry keys and values under HKEY_CURRENT_USER (Sprint 054, family "registry"; values Sprint 063).
 *
 * contracts/windows.registry.registry records Windows (Server 2025 and 2022, equal):
 *   - RegOpenKeyExW of a key that does not exist answers ERROR_FILE_NOT_FOUND and writes NULL to the result, one
 *     level or two below HKEY_CURRENT_USER\Software, by the root or by a handle to Software; names compare without
 *     case
 *   - Software opens (ERROR_SUCCESS, a new handle); a NULL or empty subkey gives the root's own predefined handle
 *   - a handle that is no key answers ERROR_INVALID_HANDLE with NULL written
 *   - RegCloseKey closes a key once (ERROR_SUCCESS, then ERROR_INVALID_HANDLE), closes a predefined root with
 *     ERROR_SUCCESS, and answers ERROR_INVALID_HANDLE for NULL or a handle that is no key
 *   - no function here touches the last error
 * Sprint 063, the game's writes (Unity's PlayerPrefs under Software\NSS\Hail to the Rainbow, game run 38206263976
 * seq 2734-3490) and their edges:
 *   - RegCreateKeyW makes the key and every missing level above it (ERROR_SUCCESS, a new handle with every right), and
 *     opens one that exists the same way
 *   - RegSetValueExA stores REG_BINARY, REG_DWORD and REG_SZ (the ANSI string stored with its NUL whether or not the
 *     caller counted one); through a handle opened for reading it answers ERROR_ACCESS_DENIED, as RegDeleteValueA does
 *   - value names compare without case, ASCII and Latin-1 alike (é and É name one value)
 *   - RegQueryValueExA of a missing value answers ERROR_FILE_NOT_FOUND and writes REG_NONE to the type, leaving the
 *     size and the data; of a present one the type, the size and the data; with no data pointer the size alone; with
 *     a short buffer ERROR_MORE_DATA, the type and the size, the data untouched; a data pointer without a size pointer
 *     is ERROR_INVALID_PARAMETER. A REG_SZ comes back as its UTF-16 copy converted to ANSI in place: the ANSI string
 *     with its NUL, and past it the rest of the UTF-16 bytes ("abc" leaves 61 62 63 00 63 00 00 00)
 *   - RegDeleteValueA removes a value (ERROR_SUCCESS), then answers ERROR_FILE_NOT_FOUND
 *   - RegDeleteKeyW refuses a key with a subkey (ERROR_ACCESS_DENIED) and removes a leaf with its values
 * w2m's registry starts each run with HKEY_CURRENT_USER and its Software key, empty: the game's keys do not exist on
 * a fresh machine (seq 1652), and the rows show what a run writes is there for the rest of it (seq 3490 reads a value
 * seq 3456 set). Nothing persists between runs: the runners start fresh, and the game's first run on a machine is the
 * one the reference records. Other roots, options, value types, sizes past VALUE_DATA, names past NAME units, ANSI
 * names with bytes 0x80-0x9F (code page 1252 maps them above Latin-1) and short buffers for REG_SZ are not in the rows
 * and stop the run.
 */
#include "runtime.h"

typedef unsigned short u16;

#define ERROR_SUCCESS 0u
#define ERROR_FILE_NOT_FOUND 2u
#define ERROR_ACCESS_DENIED 5u
#define ERROR_INVALID_HANDLE 6u
#define ERROR_INVALID_PARAMETER 87u
#define ERROR_MORE_DATA 234u
#define HKEY_CURRENT_USER 0xFFFFFFFF80000001ull       /* (HKEY)(ULONG_PTR)(LONG)0x80000001, sign-extended */
#define KEY_BASE 0x3000ull                           /* key handles: KEY_BASE + 4*slot */
#define KEYS 64
#define PATH 256
#define KEY_QUERY_VALUE 0x1u
#define KEY_SET_VALUE 0x2u
#define KEY_ALL_ACCESS 0xF003Fu
#define REG_NONE 0u
#define REG_SZ 1u
#define REG_BINARY 3u
#define REG_DWORD 4u
#define MADE 32                                      /* keys a run creates */
#define VALUES 256
#define NAME 128
#define VALUE_DATA 128

/* The keys that exist from the start, as paths below HKEY_CURRENT_USER in lower case ("" is the root). */
static const char *const existing[] = {"", "software"};

/* Paths are kept as UTF-16, a key's name folded to lower case for ASCII letters only: a unit above ASCII is never
 * narrowed (the review of Sprint 054 found U+0173 read as 's' and U+0100 as the name's end), so no such name meets
 * an ASCII key. */
static struct { int open; u32 access; u16 path[PATH]; } keys[KEYS];
static struct { int used; u16 path[PATH]; } made[MADE];
/* a value: its key (made[] index), its name folded, its type and its data as the caller gave it (a REG_SZ's ANSI
 * bytes with the NUL) */
static struct { int used, key; u16 name[NAME]; u32 type, size; u8 data[VALUE_DATA]; } values[VALUES];

static u16 lower(u16 c) { return c >= 'A' && c <= 'Z' ? (u16)(c + 32) : c; }

static int same(const u16 *a, const u16 *b) {
    while (*a && *a == *b) a++, b++;
    return *a == *b;
}

/* path of handle h into out; 0: h names no open key */
static int path_of(u64 h, u16 *out) {
    if (h == HKEY_CURRENT_USER) { out[0] = 0; return 1; }
    if (h < KEY_BASE || (h - KEY_BASE) % 4 || (h - KEY_BASE) / 4 >= KEYS || !keys[(h - KEY_BASE) / 4].open) return 0;
    const u16 *p = keys[(h - KEY_BASE) / 4].path;
    int i = 0;
    while ((out[i] = p[i])) i++;
    return 1;
}

static int made_index(const u16 *path) {
    for (int k = 0; k < MADE; k++)
        if (made[k].used && same(made[k].path, path)) return k;
    return -1;
}

static int exists(const u16 *path) {
    for (unsigned k = 0; k < sizeof existing / sizeof *existing; k++) {
        const char *a = existing[k];
        const u16 *b = path;
        while (*a && (u16)(unsigned char)*a == *b) a++, b++;
        if (!*a && !*b) return 1;
    }
    return made_index(path) >= 0;
}

/* root's path, a separator and sub folded, into path; the API that asks names the stop */
static void join(u64 api, const u16 *root_path, const u16 *s, u16 *path) {
    int n = 0;
    while ((path[n] = root_path[n])) n++;
    if (*s == '\\') w2m_unimplemented(api, (u64)s);                                    /* a leading separator */
    if (n) path[n++] = '\\';
    for (; *s; s++) {
        if (n >= PATH - 1) w2m_unimplemented(api, PATH);
        path[n++] = lower(*s);
    }
    path[n] = 0;
}

static u64 open_handle(u64 api, const u16 *path, u32 access) {
    for (int k = 0; k < KEYS; k++)
        if (!keys[k].open) {
            keys[k].open = 1;
            keys[k].access = access;
            for (int i = 0; (keys[k].path[i] = path[i]); i++) {}
            return KEY_BASE + 4 * (u64)k;
        }
    w2m_unimplemented(api, KEYS);
    return 0;
}

/* RegOpenKeyExW(hKey, lpSubKey, ulOptions, samDesired, phkResult) */
u64 advapi32_RegOpenKeyExW(u64 root, u64 sub, u64 options, u64 access, u64 rsp) {
    u64 result = *(const u64 *)(rsp + 0x28);
    if ((u32)options) w2m_unimplemented(W2M_API_RegOpenKeyExW, options);        /* REG_OPTION_OPEN_LINK: not in the rows */
    if (root >= 0xFFFFFFFF80000000ull && root != HKEY_CURRENT_USER) w2m_unimplemented(W2M_API_RegOpenKeyExW, root);
    u16 path[PATH];
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
    u16 base[PATH];
    for (int i = 0; (base[i] = path[i]); i++) {}
    join(W2M_API_RegOpenKeyExW, base, s, path);
    if (!exists(path)) {
        *(u64 *)result = 0;
        return w2m_logged(W2M_API_RegOpenKeyExW, root, ERROR_FILE_NOT_FOUND);
    }
    *(u64 *)result = open_handle(W2M_API_RegOpenKeyExW, path, (u32)access);
    return w2m_logged(W2M_API_RegOpenKeyExW, root, ERROR_SUCCESS);
}

u64 advapi32_RegCloseKey(u64 h) {
    if (h == HKEY_CURRENT_USER) return w2m_logged(W2M_API_RegCloseKey, h, ERROR_SUCCESS);
    u16 path[PATH];
    if (!path_of(h, path)) return w2m_logged(W2M_API_RegCloseKey, h, ERROR_INVALID_HANDLE);
    keys[(h - KEY_BASE) / 4].open = 0;
    return w2m_logged(W2M_API_RegCloseKey, h, ERROR_SUCCESS);
}

/* RegCreateKeyW(hKey, lpSubKey, phkResult): the key and each missing level above it */
u64 advapi32_RegCreateKeyW(u64 root, u64 sub, u64 result) {
    if (root >= 0xFFFFFFFF80000000ull && root != HKEY_CURRENT_USER) w2m_unimplemented(W2M_API_RegCreateKeyW, root);
    const u16 *s = (const u16 *)sub;
    if (!s || !*s) w2m_unimplemented(W2M_API_RegCreateKeyW, sub);                  /* the key's own handle: no row */
    u16 base[PATH], path[PATH];
    if (!path_of(root, base)) w2m_unimplemented(W2M_API_RegCreateKeyW, root);      /* no key: no row */
    join(W2M_API_RegCreateKeyW, base, s, path);
    for (int n = 0;; n++) {                                                       /* each level, the key last */
        if (path[n] != '\\' && path[n] != 0) continue;
        u16 c = path[n];
        path[n] = 0;
        if (!exists(path)) {
            int k = 0;
            while (k < MADE && made[k].used) k++;
            if (k == MADE) w2m_unimplemented(W2M_API_RegCreateKeyW, MADE);
            made[k].used = 1;
            for (int i = 0; (made[k].path[i] = path[i]); i++) {}
        }
        path[n] = c;
        if (!c) break;
    }
    *(u64 *)result = open_handle(W2M_API_RegCreateKeyW, path, KEY_ALL_ACCESS);
    return w2m_logged(W2M_API_RegCreateKeyW, root, ERROR_SUCCESS);
}

/* RegDeleteKeyW(hKey, lpSubKey): a leaf the run made, with its values */
u64 advapi32_RegDeleteKeyW(u64 root, u64 sub) {
    u16 base[PATH], path[PATH];
    const u16 *s = (const u16 *)sub;
    if (!s || !*s || !path_of(root, base)) w2m_unimplemented(W2M_API_RegDeleteKeyW, sub);   /* no row */
    join(W2M_API_RegDeleteKeyW, base, s, path);
    int k = made_index(path);
    if (k < 0) w2m_unimplemented(W2M_API_RegDeleteKeyW, sub);          /* missing, or Software itself: no row */
    int n = 0;
    while (path[n]) n++;
    for (int j = 0; j < MADE; j++) {                                    /* a subkey: path, a separator, more */
        if (!made[j].used || j == k) continue;
        int i = 0;
        while (i < n && made[j].path[i] == path[i]) i++;
        if (i == n && made[j].path[n] == '\\') return w2m_logged(W2M_API_RegDeleteKeyW, root, ERROR_ACCESS_DENIED);
    }
    made[k].used = 0;
    for (int v = 0; v < VALUES; v++)
        if (values[v].used && values[v].key == k) values[v].used = 0;
    return w2m_logged(W2M_API_RegDeleteKeyW, root, ERROR_SUCCESS);
}

/* An ANSI (code page 1252) value name folded into name: ASCII and Latin-1 letters to lower case (é and É are one
 * name, row query.latin1_case; Unicode's simple case mapping of U+00C0-U+00DE, U+00D7 aside). Bytes 0x80-0x9F map
 * above Latin-1 and stop the run. */
static void value_name(u64 api, u64 ansi, u16 *name) {
    const u8 *a = (const u8 *)ansi;
    if (!a || !*a) w2m_unimplemented(api, ansi);                       /* the key's default value: no row */
    int n = 0;
    for (; *a; a++) {
        u16 c = *a;
        if (c >= 0x80 && c < 0xA0) w2m_unimplemented(api, c);
        if (n >= NAME - 1) w2m_unimplemented(api, NAME);
        if ((c >= 'A' && c <= 'Z') || (c >= 0xC0 && c <= 0xDE && c != 0xD7)) c = (u16)(c + 32);
        name[n++] = c;
    }
    name[n] = 0;
}

/* the open key h with `right`; a key the run made (values live only there); the status otherwise */
static int value_key(u64 api, u64 h, u32 right, u32 *status) {
    if (h < KEY_BASE || (h - KEY_BASE) % 4 || (h - KEY_BASE) / 4 >= KEYS || !keys[(h - KEY_BASE) / 4].open)
        w2m_unimplemented(api, h);                                         /* a root or no key: no row */
    int slot = (int)((h - KEY_BASE) / 4);
    if (!(keys[slot].access & right)) { *status = ERROR_ACCESS_DENIED; return -1; }
    int k = made_index(keys[slot].path);
    if (k < 0) w2m_unimplemented(api, h);                                 /* Software's own values: no row */
    *status = ERROR_SUCCESS;
    return k;
}

static int find(int key, const u16 *name) {
    for (int v = 0; v < VALUES; v++)
        if (values[v].used && values[v].key == key && same(values[v].name, name)) return v;
    return -1;
}

/* RegSetValueExA(hKey, lpValueName, Reserved, dwType, lpData, cbData) */
u64 advapi32_RegSetValueExA(u64 h, u64 value, u64 reserved, u64 type, u64 rsp) {
    u64 data = *(const u64 *)(rsp + 0x28);
    u32 size = *(const u32 *)(rsp + 0x30);
    if ((u32)reserved) w2m_unimplemented(W2M_API_RegSetValueExA, reserved);
    u32 status;
    int key = value_key(W2M_API_RegSetValueExA, h, KEY_SET_VALUE, &status);
    if (key < 0) return w2m_logged(W2M_API_RegSetValueExA, h, status);
    u16 name[NAME];
    value_name(W2M_API_RegSetValueExA, value, name);
    u32 t = (u32)type, stored = size;
    if (t == REG_SZ) {                                                     /* the ANSI string, its NUL added if missing */
        const u8 *d = (const u8 *)data;
        stored = 0;
        while (stored < size && d[stored]) stored++;
        if (stored < size && stored + 1 != size) w2m_unimplemented(W2M_API_RegSetValueExA, size);   /* inner NUL: no row */
        stored++;
    } else if (!(t == REG_BINARY || (t == REG_DWORD && size == 4))) {
        w2m_unimplemented(W2M_API_RegSetValueExA, type);                   /* other types and sizes: no row */
    }
    if (stored > VALUE_DATA) w2m_unimplemented(W2M_API_RegSetValueExA, size);
    int v = find(key, name);
    if (v < 0) {
        v = 0;
        while (v < VALUES && values[v].used) v++;
        if (v == VALUES) w2m_unimplemented(W2M_API_RegSetValueExA, VALUES);
        values[v].used = 1;
        values[v].key = key;
        for (int i = 0; (values[v].name[i] = name[i]); i++) {}
    }
    values[v].type = t;
    values[v].size = stored;
    const u8 *d = (const u8 *)data;
    for (u32 i = 0; i < stored; i++) values[v].data[i] = (t == REG_SZ && i == stored - 1) ? 0 : d[i];
    return w2m_logged(W2M_API_RegSetValueExA, h, ERROR_SUCCESS);
}

/* RegQueryValueExA(hKey, lpValueName, lpReserved, lpType, lpData, lpcbData) */
u64 advapi32_RegQueryValueExA(u64 h, u64 value, u64 reserved, u64 type, u64 rsp) {
    u8 *data = *(u8 *const *)(rsp + 0x28);
    u32 *size = *(u32 *const *)(rsp + 0x30);
    if (reserved) w2m_unimplemented(W2M_API_RegQueryValueExA, reserved);
    u32 status;
    int key = value_key(W2M_API_RegQueryValueExA, h, KEY_QUERY_VALUE, &status);
    if (key < 0) w2m_unimplemented(W2M_API_RegQueryValueExA, h);          /* a handle without the right: no row */
    if (data && !size) return w2m_logged(W2M_API_RegQueryValueExA, h, ERROR_INVALID_PARAMETER);
    u16 name[NAME];
    value_name(W2M_API_RegQueryValueExA, value, name);
    int v = find(key, name);
    if (v < 0) {
        if (type) *(u32 *)type = REG_NONE;
        return w2m_logged(W2M_API_RegQueryValueExA, h, ERROR_FILE_NOT_FOUND);
    }
    u32 n = values[v].size;
    if (type) *(u32 *)type = values[v].type;
    if (!data) {
        if (size) *size = n;
        return w2m_logged(W2M_API_RegQueryValueExA, h, ERROR_SUCCESS);
    }
    if (values[v].type == REG_SZ) {
        if (*size < 2 * n) w2m_unimplemented(W2M_API_RegQueryValueExA, *size);   /* the UTF-16 copy does not fit: no row */
        for (u32 i = 0; i < n; i++) data[2 * i] = values[v].data[i], data[2 * i + 1] = 0;
        for (u32 i = 0; i < n; i++) data[i] = values[v].data[i];
    } else {
        if (*size < n) {
            *size = n;
            return w2m_logged(W2M_API_RegQueryValueExA, h, ERROR_MORE_DATA);
        }
        for (u32 i = 0; i < n; i++) data[i] = values[v].data[i];
    }
    *size = n;
    return w2m_logged(W2M_API_RegQueryValueExA, h, ERROR_SUCCESS);
}

/* RegDeleteValueA(hKey, lpValueName) */
u64 advapi32_RegDeleteValueA(u64 h, u64 value) {
    u32 status;
    int key = value_key(W2M_API_RegDeleteValueA, h, KEY_SET_VALUE, &status);
    if (key < 0) return w2m_logged(W2M_API_RegDeleteValueA, h, status);
    u16 name[NAME];
    value_name(W2M_API_RegDeleteValueA, value, name);
    int v = find(key, name);
    if (v < 0) return w2m_logged(W2M_API_RegDeleteValueA, h, ERROR_FILE_NOT_FOUND);
    values[v].used = 0;
    return w2m_logged(W2M_API_RegDeleteValueA, h, ERROR_SUCCESS);
}
