/* code_pages.c — code pages and character data as Windows answers (Sprint 044, family "code pages").
 *
 * No C runtime. The launcher's start-up reaches GetACP after its standard handles (frontier
 * 2026-10-10, 7,837 instructions). This family is GetACP, GetOEMCP, GetCPInfo, IsValidCodePage,
 * MultiByteToWideChar, WideCharToMultiByte, GetStringTypeW and LCMapStringW. Rows: the code pages, then
 * one row per byte of the ANSI code page (its UTF-16 unit, CT_CTYPE1 class, upper and lower case under
 * the user default locale and the invariant locale), then UTF-8 conversions and their errors, each
 * with the last error after SetLastError(0xDEAD).
 *
 * Two sweeps settle the rules the single rows leave open (first runs 2026-10-10: ED A0 80 decodes to two
 * U+FFFD, which is neither the WHATWG maximal-subpart count, three, nor one per sequence):
 *   - malformed UTF-8: every lead byte 80..FF, a second byte from the edges of the ranges in
 *     the Unicode Standard's Table 3-7, well-formed UTF-8 (41 80 8F 90 9F A0 BF C0), a third from (41 80 BF), then 80 41; the
 *     units out, and whether MB_ERR_INVALID_CHARS refuses it
 *   - WideCharToMultiByte(1252) of every UTF-16 unit 0080..FFFF that is not the ANSI table's own
 *     character: those that do not become the default character with used_default set (best fit)
 */
#include <windows.h>

#define SENTINEL 0xDEADu

static char out[1 << 20];
static unsigned n;

static void put(const char *s) { while (*s) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void puti(long long v) { if (v < 0) { out[n++] = '-'; putu((unsigned long long)-v); } else putu((unsigned long long)v); }
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void rowx(const char *prefix, unsigned v) {
    static const char d[] = "0123456789abcdef";
    put("{\"s\":\""); put(prefix); out[n++] = d[(v >> 4) & 15]; out[n++] = d[v & 15]; put("\"");
}
static void field(const char *k, long long v) { put(",\""); put(k); put("\":"); puti(v); }
static void list(const char *k, const unsigned short *w, int count) {
    put(",\""); put(k); put("\":[");
    for (int i = 0; i < count; i++) { if (i) put(","); putu(w[i]); }
    put("]");
}
static void blist(const char *k, const unsigned char *b, int count) {
    put(",\""); put(k); put("\":[");
    for (int i = 0; i < count; i++) { if (i) put(","); putu(b[i]); }
    put("]");
}
static void end(void) { put("}\n"); }

static DWORD err;

static void cpinfo(const char *name, UINT cp) {
    CPINFO ci;
    unsigned char *b = (unsigned char *)&ci;
    for (unsigned i = 0; i < sizeof ci; i++) b[i] = 0xA5;
    SetLastError(SENTINEL);
    BOOL ok = GetCPInfo(cp, &ci);
    err = GetLastError();
    row(name); field("ret", ok != 0); field("err", err);
    if (ok) { field("max_char_size", ci.MaxCharSize); blist("default_char", ci.DefaultChar, 2); blist("lead_bytes", ci.LeadByte, 4); }
    end();
}

static void m2w(const char *name, UINT cp, DWORD flags, const char *bytes, int len, int cap) {
    static WCHAR w[64];
    for (int i = 0; i < 64; i++) w[i] = 0x7777;
    SetLastError(SENTINEL);
    int r = MultiByteToWideChar(cp, flags, bytes, len, cap ? w : NULL, cap);
    err = GetLastError();
    row(name); field("ret", r); field("err", err);
    if (cap) list("out", w, r > 0 && r < 64 ? r : 0);
    end();
}

static void w2m_(const char *name, UINT cp, const WCHAR *ws, int len, int cap, int with_used) {
    static char b[64];
    for (int i = 0; i < 64; i++) b[i] = (char)0x77;
    BOOL used = 7;
    SetLastError(SENTINEL);
    int r = WideCharToMultiByte(cp, 0, ws, len, cap ? b : NULL, cap, NULL, with_used ? &used : NULL);
    err = GetLastError();
    row(name); field("ret", r); field("err", err);
    if (cap) blist("out", (const unsigned char *)b, r > 0 && r < 64 ? r : 0);
    if (with_used) field("used_default", used);
    end();
}

void start(void) {
    SetLastError(SENTINEL);
    UINT acp = GetACP();
    err = GetLastError();
    row("acp"); field("value", acp); field("err", err); end();
    row("oemcp"); field("value", GetOEMCP()); end();
    static const UINT pages[] = {CP_ACP, CP_OEMCP, 1252, 437, CP_UTF8, 932, 12345};
    static const char *const names[] = {"valid.cp_acp", "valid.cp_oemcp", "valid.1252", "valid.437", "valid.utf8",
                                        "valid.932", "valid.12345"};
    for (int i = 0; i < 7; i++) { row(names[i]); field("value", IsValidCodePage(pages[i]) != 0); end(); }
    cpinfo("cpinfo.acp", CP_ACP);
    cpinfo("cpinfo.1252", 1252);
    cpinfo("cpinfo.utf8", CP_UTF8);
    cpinfo("cpinfo.437", 437);
    cpinfo("cpinfo.12345", 12345);

    /* every byte of the ANSI code page */
    for (unsigned b = 0; b < 256; b++) {
        char c = (char)b;
        WCHAR u = 0;
        int r = MultiByteToWideChar(CP_ACP, 0, &c, 1, &u, 1);
        WORD t1 = 0;
        BOOL okt = GetStringTypeW(CT_CTYPE1, &u, 1, &t1);
        WCHAR up_user = 0, lo_user = 0, up_inv = 0, lo_inv = 0;
        int ru = LCMapStringW(LOCALE_USER_DEFAULT, LCMAP_UPPERCASE, &u, 1, &up_user, 1);
        int rl = LCMapStringW(LOCALE_USER_DEFAULT, LCMAP_LOWERCASE, &u, 1, &lo_user, 1);
        int iu = LCMapStringW(LOCALE_INVARIANT, LCMAP_UPPERCASE, &u, 1, &up_inv, 1);
        int il = LCMapStringW(LOCALE_INVARIANT, LCMAP_LOWERCASE, &u, 1, &lo_inv, 1);
        rowx("acp.", b); field("ret", r); field("u", u); field("ctype1", okt ? t1 : -1);
        field("upper_user", ru == 1 ? up_user : -1); field("lower_user", rl == 1 ? lo_user : -1);
        field("upper_invariant", iu == 1 ? up_inv : -1); field("lower_invariant", il == 1 ? lo_inv : -1); end();
    }

    /* MultiByteToWideChar */
    m2w("m2w.acp.size_query", CP_ACP, 0, "hello", 5, 0);
    m2w("m2w.acp.with_nul", CP_ACP, 0, "hi", -1, 16);
    m2w("m2w.acp.too_small", CP_ACP, 0, "hello", 5, 2);
    m2w("m2w.utf8.ascii", CP_UTF8, 0, "abc", 3, 16);
    m2w("m2w.utf8.two_byte", CP_UTF8, 0, "\xc3\xa9", 2, 16);
    m2w("m2w.utf8.three_byte", CP_UTF8, 0, "\xe2\x82\xac", 3, 16);
    m2w("m2w.utf8.four_byte", CP_UTF8, 0, "\xf0\x9f\x98\x80", 4, 16);
    m2w("m2w.utf8.invalid", CP_UTF8, 0, "a\xffz", 3, 16);
    m2w("m2w.utf8.invalid_strict", CP_UTF8, MB_ERR_INVALID_CHARS, "a\xffz", 3, 16);
    m2w("m2w.utf8.truncated", CP_UTF8, 0, "a\xe2\x82", 3, 16);
    m2w("m2w.utf8.overlong", CP_UTF8, 0, "\xc0\xaf", 2, 16);
    m2w("m2w.utf8.surrogate_encoded", CP_UTF8, 0, "\xed\xa0\x80", 3, 16);
    m2w("m2w.zero_length", CP_UTF8, 0, "abc", 0, 16);
    m2w("m2w.bad_flags_utf8", CP_UTF8, MB_PRECOMPOSED, "a", 1, 16);
    m2w("m2w.invalid_cp", 12345, 0, "a", 1, 16);

    /* WideCharToMultiByte */
    static const WCHAR hello[] = {'h', 'e', 'l', 'l', 'o', 0};
    static const WCHAR euro[] = {0x20AC, 0};
    static const WCHAR cjk[] = {'a', 0x4E2D, 'b', 0};
    static const WCHAR smile[] = {0xD83D, 0xDE00, 0};
    static const WCHAR lone[] = {'x', 0xD800, 'y', 0};
    w2m_("w2m.acp.size_query", CP_ACP, hello, 5, 0, 0);
    w2m_("w2m.acp.with_nul", CP_ACP, hello, -1, 16, 0);
    w2m_("w2m.acp.euro", CP_ACP, euro, 1, 16, 1);
    w2m_("w2m.acp.unmappable", CP_ACP, cjk, 3, 16, 1);
    w2m_("w2m.acp.too_small", CP_ACP, hello, 5, 2, 0);
    w2m_("w2m.utf8.euro", CP_UTF8, euro, 1, 16, 0);
    w2m_("w2m.utf8.pair", CP_UTF8, smile, 2, 16, 0);
    w2m_("w2m.utf8.lone_surrogate", CP_UTF8, lone, 3, 16, 0);
    w2m_("w2m.utf8.used_default_param", CP_UTF8, hello, 5, 16, 1);

    /* malformed UTF-8 */
    static const unsigned char seconds[] = {0x41, 0x80, 0x8F, 0x90, 0x9F, 0xA0, 0xBF, 0xC0};
    static const unsigned char thirds[] = {0x41, 0x80, 0xBF};
    for (unsigned l = 0x80; l < 0x100; l++)
        for (int si = 0; si < 8; si++)
            for (int ti = 0; ti < 3; ti++) {
                char in[5] = {(char)l, (char)seconds[si], (char)thirds[ti], (char)0x80, 'A'};
                static WCHAR w[16];
                int r = MultiByteToWideChar(CP_UTF8, 0, in, 5, w, 16);
                SetLastError(SENTINEL);
                int rs = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, in, 5, NULL, 0);
                err = GetLastError();
                rowx("u8.", l); out[n - 1] = '.';
                { static const char d[] = "0123456789abcdef";
                  out[n++] = d[seconds[si] >> 4]; out[n++] = d[seconds[si] & 15];
                  out[n++] = d[thirds[ti] >> 4]; out[n++] = d[thirds[ti] & 15]; out[n++] = '"'; }
                list("out", w, r > 0 && r <= 16 ? r : 0); field("strict", rs); field("strict_err", err); end();
            }

    /* best fit into the ANSI code page */
    static WCHAR back[256];
    for (unsigned b = 0; b < 256; b++) { char c = (char)b; MultiByteToWideChar(1252, 0, &c, 1, &back[b], 1); }
    for (unsigned u = 0x80; u < 0x10000; u++) {
        int own = 0;
        for (unsigned b = 0; b < 256 && !own; b++) own = back[b] == u;
        if (own) continue;
        WCHAR wc = (WCHAR)u;
        char c = 0;
        BOOL used = 0;
        int r = WideCharToMultiByte(1252, 0, &wc, 1, &c, 1, NULL, &used);
        if (r == 1 && used && c == '?') continue;
        row("bestfit"); field("u", u); field("ret", r); field("byte", (unsigned char)c); field("used_default", used); end();
    }

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
