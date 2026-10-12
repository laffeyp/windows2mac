// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* registry.c — opening registry keys as the game first does (Sprint 054, family "registry"; specs/work_list.json).
 *
 * No C runtime. The batch trace's first registry call (seq 1653) is RegOpenKeyExW(HKEY_CURRENT_USER, <a key under
 * Software>, 0, KEY_READ) answering ERROR_FILE_NOT_FOUND: the game's keys do not exist on a fresh machine. The rows
 * open a missing key (one and two levels below Software), Software itself, the root by a NULL and an empty subkey,
 * a handle that is no key, and close keys, a closed key and no key. Each row keeps the code, what was written to
 * the result, and the last error after SetLastError(0xDEAD) (registry functions return their code).
 *
 * Sprint 063: the game's writes (game run 38206263976 seq 2734-3490, Unity's PlayerPrefs). RegCreateKeyW of its key
 * HKEY_CURRENT_USER\Software\NSS\Hail to the Rainbow, RegSetValueExA of REG_BINARY (2 and 20 bytes) and REG_DWORD
 * values under the game's own names, RegQueryValueExA in the game's two forms (type, data and size pointers; all
 * NULL) for missing and present values, RegDeleteValueA of missing ones. Then the edges the code takes: a short
 * buffer, a size-only query, data without a size pointer, REG_SZ with and without its NUL, names in another case
 * (ASCII and Latin-1), writes through a KEY_READ handle, a delete and a query after it, creating an existing key.
 * The probe deletes what it creates (RegDeleteKeyW), so a second run on the runner starts as the first did.
 */
#include <windows.h>

#define SENTINEL 0xDEADu
#define UNTOUCHED ((HKEY)(ULONG_PTR)0x1234)

static char out[8192];
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

static HKEY open(const char *name, HKEY root, const WCHAR *sub) {
    HKEY h = UNTOUCHED;
    SetLastError(SENTINEL);
    LONG r = RegOpenKeyExW(root, sub, 0, KEY_READ, &h);
    row(name); field("ret", (DWORD)r); field("err", GetLastError());
    flag("result_null", h == NULL); flag("result_untouched", h == UNTOUCHED);
    flag("result_is_root", h == root);
    end();
    return r == ERROR_SUCCESS ? h : NULL;
}

static void close(const char *name, HKEY h) {
    SetLastError(SENTINEL);
    LONG r = RegCloseKey(h);
    row(name); field("ret", (DWORD)r); field("err", GetLastError()); end();
}

#define GAME_KEY L"Software\\NSS\\Hail to the Rainbow"

static void bytes(const char *k, const unsigned char *b, DWORD count) {
    static const char hex[] = "0123456789abcdef";
    put(",\""); put(k); put("\":\"");
    for (DWORD i = 0; i < count; i++) { char c[3] = {hex[b[i] >> 4], hex[b[i] & 15], 0}; put(c); }
    put("\"");
}

static HKEY create(const char *name, HKEY root, const WCHAR *sub) {
    HKEY h = UNTOUCHED;
    SetLastError(SENTINEL);
    LONG r = RegCreateKeyW(root, sub, &h);
    row(name); field("ret", (DWORD)r); field("err", GetLastError());
    flag("result_null", h == NULL); flag("result_untouched", h == UNTOUCHED); flag("result_is_root", h == root); end();
    return r == ERROR_SUCCESS ? h : NULL;
}

static void set(const char *name, HKEY h, const char *value, DWORD type, const void *data, DWORD size) {
    SetLastError(SENTINEL);
    LONG r = RegSetValueExA(h, value, 0, type, (const BYTE *)data, size);
    row(name); field("ret", (DWORD)r); field("err", GetLastError()); end();
}

/* RegQueryValueExA in the game's form (type, data and size pointers) with a buffer of `room` bytes; the row keeps
 * the buffer's first room + 2 bytes, so a write past `room` would show */
static void query(const char *name, HKEY h, const char *value, DWORD room) {
    unsigned char data[64];
    for (unsigned i = 0; i < sizeof data; i++) data[i] = 0xA5;
    DWORD type = 0xA5A5, size = room;
    SetLastError(SENTINEL);
    LONG r = RegQueryValueExA(h, value, NULL, &type, data, &size);
    row(name); field("ret", (DWORD)r); field("err", GetLastError()); field("type", type); field("size", size);
    bytes("data", data, room + 2); end();
}

static void query_null(const char *name, HKEY h, const char *value) {       /* the game's all-NULL form */
    SetLastError(SENTINEL);
    LONG r = RegQueryValueExA(h, value, NULL, NULL, NULL, NULL);
    row(name); field("ret", (DWORD)r); field("err", GetLastError()); end();
}

static void query_size(const char *name, HKEY h, const char *value) {       /* lpData NULL: the size alone */
    DWORD type = 0xA5A5, size = 0xA5A5;
    SetLastError(SENTINEL);
    LONG r = RegQueryValueExA(h, value, NULL, &type, NULL, &size);
    row(name); field("ret", (DWORD)r); field("err", GetLastError()); field("type", type); field("size", size); end();
}

static void delete_value(const char *name, HKEY h, const char *value) {
    SetLastError(SENTINEL);
    LONG r = RegDeleteValueA(h, value);
    row(name); field("ret", (DWORD)r); field("err", GetLastError()); end();
}

static void delete_key(const char *name, HKEY root, const WCHAR *sub) {
    SetLastError(SENTINEL);
    LONG r = RegDeleteKeyW(root, sub);
    row(name); field("ret", (DWORD)r); field("err", GetLastError()); end();
}

static void writes(void) {
    open("write.open_before", HKEY_CURRENT_USER, GAME_KEY);                    /* seq 1652: missing */
    HKEY game = create("write.create", HKEY_CURRENT_USER, GAME_KEY);           /* seq 2734 */
    if (!game) return;
    HKEY again = create("write.create_existing", HKEY_CURRENT_USER, GAME_KEY);
    HKEY reader = open("write.open_after", HKEY_CURRENT_USER, GAME_KEY);       /* seq 2890: present, KEY_READ */
    static const unsigned char two[2] = {0x31, 0x00};
    static const unsigned char twenty[20] = {1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20};
    DWORD quality = 4;
    set("set.binary_2", game, "unity.player_session_count_h922449978", REG_BINARY, two, 2);       /* seq 2740 */
    set("set.binary_20", game, "unity.player_sessionid_h1351336811", REG_BINARY, twenty, 20);     /* seq 2743 */
    set("set.dword", game, "UnityGraphicsQuality_h1669003810", REG_DWORD, &quality, 4);         /* seq 2981 */
    query("query.missing", reader, "_Level_Metka_h1161804037", 16);                             /* seq 2891 */
    query_null("query.missing_all_null", reader, "VolID_h225847549");                           /* seq 2965 */
    query("query.dword", reader, "UnityGraphicsQuality_h1669003810", 16);
    query("query.dword_case", reader, "UNITYGRAPHICSQUALITY_H1669003810", 16);
    query("query.binary_20", reader, "unity.player_sessionid_h1351336811", 32);
    query("query.binary_20_short", reader, "unity.player_sessionid_h1351336811", 8);
    query_size("query.binary_20_size_only", reader, "unity.player_sessionid_h1351336811");
    query_null("query.present_all_null", reader, "unity.player_session_count_h922449978");
    {
        unsigned char data[8];
        SetLastError(SENTINEL);
        LONG r = RegQueryValueExA(reader, "UnityGraphicsQuality_h1669003810", NULL, NULL, data, NULL);
        row("query.data_without_size"); field("ret", (DWORD)r); field("err", GetLastError()); end();
    }
    set("set.sz_with_nul", game, "w2m_sz_nul", REG_SZ, "abc", 4);
    set("set.sz_without_nul", game, "w2m_sz_bare", REG_SZ, "abc", 3);
    query("query.sz_with_nul", reader, "w2m_sz_nul", 16);
    query("query.sz_without_nul", reader, "w2m_sz_bare", 16);
    set("set.latin1", game, "caf\xe9", REG_DWORD, &quality, 4);
    query("query.latin1_case", reader, "CAF\xc9", 16);
    set("set.through_read_handle", reader, "w2m_denied", REG_DWORD, &quality, 4);
    delete_value("delete.through_read_handle", reader, "caf\xe9");
    delete_value("delete.missing", game, "CutScene_h877322457");                               /* seq 2957 */
    delete_value("delete.present", game, "UnityGraphicsQuality_h1669003810");
    query("query.after_delete", reader, "UnityGraphicsQuality_h1669003810", 16);
    delete_value("delete.again", game, "UnityGraphicsQuality_h1669003810");
    if (reader) close("write.close_reader", reader);
    if (again) close("write.close_again", again);
    close("write.close_game", game);
    delete_key("delete_key.parent_first", HKEY_CURRENT_USER, L"Software\\NSS");
    delete_key("delete_key.game", HKEY_CURRENT_USER, GAME_KEY);
    delete_key("delete_key.parent", HKEY_CURRENT_USER, L"Software\\NSS");
    open("write.open_after_delete", HKEY_CURRENT_USER, GAME_KEY);
}

void start(void) {
    open("open.missing", HKEY_CURRENT_USER, L"Software\\w2m-probe-missing-5d1c");
    open("open.missing_two_levels", HKEY_CURRENT_USER, L"Software\\w2m-probe-missing-5d1c\\Deeper");
    open("open.missing_case", HKEY_CURRENT_USER, L"SOFTWARE\\W2M-PROBE-MISSING-5D1C");
    HKEY software = open("open.software", HKEY_CURRENT_USER, L"Software");
    HKEY software_case = open("open.software_case", HKEY_CURRENT_USER, L"sOFTWARE");
    if (software) open("open.missing_below_software_handle", software, L"w2m-probe-missing-5d1c");
    HKEY by_null = open("open.root_null_subkey", HKEY_CURRENT_USER, NULL);
    HKEY by_empty = open("open.root_empty_subkey", HKEY_CURRENT_USER, L"");
    open("open.not_a_key", UNTOUCHED, L"Software");
    /* Sprint 056 (review of 054): names with letters above ASCII, which w2m had narrowed to char */
    open("open.u0173_oftware", HKEY_CURRENT_USER, L"\x0173oftware");
    open("open.software_u0100_x", HKEY_CURRENT_USER, L"Software\x0100x");
    open("open.long_s_oftware", HKEY_CURRENT_USER, L"\x017Foftware");
    open("open.software_dotless_i", HKEY_CURRENT_USER, L"Softwar\x0131e");
    if (software) close("close.software", software);
    if (software_case) close("close.software_case", software_case);
    if (software) close("close.software_again", software);
    if (by_null) close("close.root_null_subkey", by_null);
    if (by_empty) close("close.root_empty_subkey", by_empty);
    close("close.not_a_key", UNTOUCHED);
    close("close.null", NULL);
    writes();
    ExitProcess(0);
}
