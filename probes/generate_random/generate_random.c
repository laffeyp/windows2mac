// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* generate_random.c — bcrypt's BCryptGenRandom (Sprint 047, family "generate random").
 *
 * No C runtime. UnityPlayer.dll calls BCryptGenRandom(NULL, buffer, 16, BCRYPT_USE_SYSTEM_PREFERRED_RNG) at
 * 0x180823587 (look-ahead 2026-10-10). Random bytes have no fixed rows, so the rows are each form's NTSTATUS
 * and last error (after SetLastError(0xDEAD)), whether the buffer changed, and relations on the bytes: two
 * draws differ, 64 KB of draws hold every byte value, and the bytes past the asked length stay as they were.
 */
#include <windows.h>
#include <bcrypt.h>

#define SENTINEL 0xDEADu

static char out[16384];
static unsigned n;
static unsigned char a[65536 + 16], b[64];

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

static void fill(unsigned char *p, unsigned len, unsigned char v) { for (unsigned i = 0; i < len; i++) p[i] = v; }
static int all(const unsigned char *p, unsigned len, unsigned char v) {
    for (unsigned i = 0; i < len; i++) if (p[i] != v) return 0;
    return 1;
}

static void form(const char *name, BCRYPT_ALG_HANDLE h, PUCHAR buf, ULONG len, ULONG flags) {
    fill(b, sizeof b, 0xA5);
    SetLastError(SENTINEL);
    NTSTATUS s = BCryptGenRandom(h, buf, len, flags);
    DWORD err = GetLastError();
    row(name); field("status", (unsigned)s); field("err", err);
    flag("changed", buf == b && !all(b, len, 0xA5)); flag("tail_kept", buf != b || all(b + len, sizeof b - len, 0xA5));
    end();
}

void start(void) {
    form("system_rng.16", NULL, b, 16, BCRYPT_USE_SYSTEM_PREFERRED_RNG);
    form("system_rng.1", NULL, b, 1, BCRYPT_USE_SYSTEM_PREFERRED_RNG);
    form("system_rng.0", NULL, b, 0, BCRYPT_USE_SYSTEM_PREFERRED_RNG);
    form("system_rng.null_buffer", NULL, NULL, 16, BCRYPT_USE_SYSTEM_PREFERRED_RNG);
    form("system_rng.null_buffer_0", NULL, NULL, 0, BCRYPT_USE_SYSTEM_PREFERRED_RNG);
    form("null_handle.no_flags", NULL, b, 16, 0);
    form("system_rng.entropy_flag", NULL, b, 16, BCRYPT_USE_SYSTEM_PREFERRED_RNG | BCRYPT_RNG_USE_ENTROPY_IN_BUFFER);
    form("system_rng.unknown_flag", NULL, b, 16, BCRYPT_USE_SYSTEM_PREFERRED_RNG | 0x100);
    form("bad_handle.system_rng", (BCRYPT_ALG_HANDLE)0x1234, b, 16, BCRYPT_USE_SYSTEM_PREFERRED_RNG);

    /* the bytes: two 16-byte draws differ; 64 KB hold all 256 values; none is all zero */
    unsigned char first[16];
    BCryptGenRandom(NULL, first, 16, BCRYPT_USE_SYSTEM_PREFERRED_RNG);
    BCryptGenRandom(NULL, b, 16, BCRYPT_USE_SYSTEM_PREFERRED_RNG);
    int differ = 0;
    for (int i = 0; i < 16; i++) differ |= first[i] != b[i];
    fill(a, sizeof a, 0xA5);
    NTSTATUS s = BCryptGenRandom(NULL, a, 65536, BCRYPT_USE_SYSTEM_PREFERRED_RNG);
    static unsigned seen[256];
    unsigned values = 0;
    for (unsigned i = 0; i < 65536; i++) seen[a[i]]++;
    for (unsigned v = 0; v < 256; v++) values += seen[v] != 0;
    unsigned most = 0;
    for (unsigned v = 0; v < 256; v++) if (seen[v] > most) most = seen[v];
    row("bytes"); flag("two_draws_differ", differ); field("status_64k", (unsigned)s); field("values_in_64k", values);
    flag("no_value_over_400_in_64k", most <= 400); flag("tail_kept_64k", all(a + 65536, 16, 0xA5)); end();

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
