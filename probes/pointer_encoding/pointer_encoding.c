// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* pointer_encoding.c — EncodePointer and DecodePointer (Sprint 046, family "pointer encoding").
 *
 * No C runtime. UnityPlayer.dll's C runtime calls EncodePointer in its DllMain (scratch run of the
 * launcher, 2026-10-10). The encoding uses a per-process secret, so the rows are the relations any
 * process shows: round trips, determinism within the process, NULL's encoding, and the last error.
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
static void finish(void) {
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}

static const unsigned long long VALUES[] = {0, 1, 0x10, 0x140001000ull, 0x7FFE00001234ull, 0xFFFFFFFFFFFFFFFFull,
                                            0x8000000000000000ull, 0x123456789ABCDEF0ull};

void start(void) {
    for (unsigned i = 0; i < sizeof VALUES / sizeof VALUES[0]; i++) {
        void *p = (void *)VALUES[i];
        SetLastError(SENTINEL);
        void *e = EncodePointer(p);
        DWORD err_e = GetLastError();
        SetLastError(SENTINEL);
        void *d = DecodePointer(e);
        DWORD err_d = GetLastError();
        void *e2 = EncodePointer(p);
        static char name[16] = "value.";
        name[6] = (char)('0' + i); name[7] = 0;
        row(name); flag("round_trip", d == p); flag("same_twice", e2 == e); flag("changed", e != p);
        flag("decode_changes", DecodePointer(p) != p); field("encode_err", err_e); field("decode_err", err_d); end();
    }
    /* two different values never encode alike (a bijection), and encodings differ by more than XOR with one mask */
    void *a = EncodePointer((void *)1), *b = EncodePointer((void *)2), *c = EncodePointer((void *)3);
    row("encoding.shape"); flag("distinct", a != b && b != c && a != c);
    flag("xor_with_one_mask", ((unsigned long long)a ^ 1) == ((unsigned long long)b ^ 2) && ((unsigned long long)b ^ 2) == ((unsigned long long)c ^ 3));
    end();
    finish();
}
