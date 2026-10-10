/* environment.c — the environment block a process sees (Sprint 044, family "environment").
 *
 * No C runtime; built as a GUI program, as the launcher is. The launcher's C runtime asks for its
 * environment after its code pages (frontier 2026-10-10, 18,036 instructions): GetEnvironmentStringsW,
 * then FreeEnvironmentStringsW, its only environment imports. Started by cmd (the parent), the binary
 * starts itself twice with no inherited handles and CREATE_UNICODE_ENVIRONMENT: once with a block of
 * five entries in no sorted order, one of them a drive variable ("=C:=..."), and once with an empty
 * block. Each child prints what it sees to a file the parent copies to its standard output:
 *   - the block's entries in order and its length in UTF-16 units, the final NUL included
 *   - a second call: a different pointer or the same, the same content, and whether a write into the
 *     first copy shows in the second
 *   - HeapSize(GetProcessHeap()) of the block, beside its length in bytes
 *   - FreeEnvironmentStringsW's return and last error (after SetLastError(0xDEAD))
 */
#include <windows.h>

#define SENTINEL 0xDEADu

static char out[16384];
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

static void wide_item(const WCHAR *w) {                   /* a JSON string: ASCII, '?' elsewhere */
    out[n++] = '"';
    for (; *w; w++) {
        WCHAR c = *w;
        if (c == '\\' || c == '"') out[n++] = '\\';
        out[n++] = (c >= 32 && c < 127) ? (char)c : '?';
    }
    out[n++] = '"';
}

static DWORD block_length(const WCHAR *b) {              /* units through the final NUL; 1 for an empty block */
    if (!b[0]) return 1;
    DWORD i = 0;
    while (b[i] || b[i + 1]) i++;
    return i + 2;
}

static const WCHAR SET_BLOCK[] = L"ZED=last\0alpha=1\0=C:=C:\\w2m\0SteamAppId=480\0Path=C:\\Windows\\system32\0";
static const WCHAR EMPTY_BLOCK[] = L"\0";

static int arg_is(const WCHAR *word) {
    const WCHAR *c = GetCommandLineW();
    DWORD len = 0, w = 0;
    while (c[len]) len++;
    while (word[w]) w++;
    if (len < w + 1 || c[len - w - 1] != ' ') return 0;
    for (DWORD i = 0; i < w; i++) if (c[len - w + i] != word[i]) return 0;
    return 1;
}

static void child(const WCHAR *rows_file) {
    SetLastError(SENTINEL);
    WCHAR *b = GetEnvironmentStringsW();
    DWORD err = GetLastError();
    DWORD len = b ? block_length(b) : 0;
    row("block"); flag("null", b == 0); field("err", err); field("length", len);
    put(",\"entries\":[");
    int first = 1;
    for (const WCHAR *p = b; p && *p; p += lstrlenW(p) + 1) {
        if (!first) put(",");
        wide_item(p);
        first = 0;
    }
    put("]"); end();

    WCHAR *b2 = GetEnvironmentStringsW();
    int same = b2 && block_length(b2) == len;
    for (DWORD i = 0; same && i < len; i++) same = b[i] == b2[i];
    row("second_call"); flag("same_pointer", b2 == b); flag("same_content", same); end();

    int shows = 0;
    if (b && b2 && b2 != b && b[0]) {
        WCHAR saved = b[0];
        b[0] = 'Q';
        WCHAR *b3 = GetEnvironmentStringsW();
        shows = b3 && b3[0] == 'Q';
        b[0] = saved;
        if (b3) FreeEnvironmentStringsW(b3);
    }
    row("copy"); flag("write_shows_in_next_call", shows); end();

    SIZE_T size = b ? HeapSize(GetProcessHeap(), 0, b) : 0;
    row("heap"); field("heap_size", size == (SIZE_T)-1 ? 0 : size); field("length_bytes", (unsigned long long)len * sizeof(WCHAR));
    end();

    SetLastError(SENTINEL);
    BOOL ok = FreeEnvironmentStringsW(b2);
    err = GetLastError();
    row("free"); field("ret", ok != 0); field("err", err); end();
    FreeEnvironmentStringsW(b);

    HANDLE f = CreateFileW(rows_file, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    DWORD w;
    WriteFile(f, out, n, &w, NULL);
    CloseHandle(f);
}

static DWORD run_child(const WCHAR *word, const WCHAR *block, const WCHAR *rows_file, char *dst, DWORD cap) {
    static WCHAR self[600], cmd[700];
    DWORD len = GetModuleFileNameW(NULL, self, 300), k = 0;
    cmd[k++] = '"';
    for (DWORD i = 0; i < len; i++) cmd[k++] = self[i];
    cmd[k++] = '"';
    cmd[k++] = ' ';
    for (DWORD i = 0; word[i]; i++) cmd[k++] = word[i];
    cmd[k] = 0;
    STARTUPINFOW si;
    unsigned char *b = (unsigned char *)&si;
    for (unsigned i = 0; i < sizeof si; i++) b[i] = 0;
    si.cb = sizeof si;
    PROCESS_INFORMATION pi;
    DeleteFileW(rows_file);
    if (!CreateProcessW(self, cmd, NULL, NULL, FALSE, CREATE_UNICODE_ENVIRONMENT, (LPVOID)block, NULL, &si, &pi))
        ExitProcess(2);
    WaitForSingleObject(pi.hProcess, 60000);
    CloseHandle(pi.hThread);
    CloseHandle(pi.hProcess);
    HANDLE f = CreateFileW(rows_file, GENERIC_READ, 0, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    if (f == INVALID_HANDLE_VALUE) ExitProcess(3);
    DWORD got = 0;
    ReadFile(f, dst, cap, &got, NULL);
    CloseHandle(f);
    DeleteFileW(rows_file);
    return got;
}

/* The child's rows, each name prefixed with its start ("set." or "empty."). */
static DWORD prefixed(const char *prefix, const char *rows, DWORD count, char *dst) {
    DWORD k = 0;
    for (DWORD i = 0; i < count; i++) {
        dst[k++] = rows[i];
        if (i >= 5 && rows[i - 5] == '{' && rows[i - 3] == 's' && rows[i - 1] == ':' && rows[i] == '"')
            for (const char *p = prefix; *p; p++) dst[k++] = *p;
    }
    return k;
}

void start(void) {
    if (arg_is(L"child-set")) { child(L"w2m-environment-set.jsonl"); ExitProcess(0); }
    if (arg_is(L"child-empty")) { child(L"w2m-environment-empty.jsonl"); ExitProcess(0); }
    static char rows[16384], all[40000];
    DWORD k = 0, got;
    got = run_child(L"child-set", SET_BLOCK, L"w2m-environment-set.jsonl", rows, sizeof rows);
    k += prefixed("set.", rows, got, all + k);
    got = run_child(L"child-empty", EMPTY_BLOCK, L"w2m-environment-empty.jsonl", rows, sizeof rows);
    k += prefixed("empty.", rows, got, all + k);
    DWORD w;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), all, k, &w, NULL);
    ExitProcess(0);
}
