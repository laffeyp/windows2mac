/* approximations.c — RCPPS and RSQRTPS bit for bit (Sprint 046, family "approximations").
 *
 * No C runtime. UnityPlayer.dll's DllMain stops at RSQRTPS (frontier 2026-10-10, 292,735 instructions).
 * The SDM bounds both results' relative error by 1.5 * 2^-12 and leaves the bits to the implementation;
 * AMD's and Intel's differ, and w2m's CPU profile is Intel's, so the bits are measured here, exhaustively:
 *   - tables: every input mantissa (2^23) of RCPPS on [1,2) and of RSQRTPS on [1,2) and [2,4), grouped by
 *     the mantissa's top 12 bits; per group the result of its first input, and how many inputs in the whole
 *     table give another result than their group's first (0 if 12 bits decide the result)
 *   - scaling: for every normal input exponent and 64 groups, whether the result is the table's with the
 *     exponent moved as 1/x and 1/sqrt(x) move it; the count of inputs where it is not, and the first
 *   - specials: each special input's result bits
 * The CPU's brand is in cpu.txt: rows are judged on Intel only (AMD differs, by the SDM's leave).
 */
#include <windows.h>
#include <xmmintrin.h>

/* float code without the C runtime: MSVC's linker wants the symbol the runtime would define */
int _fltused = 0;

static char out[200000];
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
static void end(void) { put("}\n"); }

static unsigned rcp(unsigned bits) {
    union { unsigned u; float f; } x = {bits}, y;
    y.f = _mm_cvtss_f32(_mm_rcp_ss(_mm_set_ss(x.f)));
    return y.u;
}
static unsigned rsqrt(unsigned bits) {
    union { unsigned u; float f; } x = {bits}, y;
    y.f = _mm_cvtss_f32(_mm_rsqrt_ss(_mm_set_ss(x.f)));
    return y.u;
}

static unsigned table[4096];

static void tabulate(const char *name, unsigned (*f)(unsigned), unsigned exponent) {
    unsigned long long varies = 0;
    for (unsigned p = 0; p < 4096; p++) {
        unsigned base = exponent << 23 | p << 11;
        table[p] = f(base);
        for (unsigned low = 1; low < 2048; low++) varies += f(base | low) != table[p];
    }
    row(name); field("exponent", exponent); field("varies", varies);
    put(",\"out\":[");
    for (unsigned p = 0; p < 4096; p++) { if (p) put(","); putu(table[p]); }
    put("]"); end();
}

/* For exponent e, f(x * 2^(e - e0)) against the table at e0 (rcp: the result's exponent moves by -(e - e0);
 * rsqrt: by -(e - e0) / 2, e - e0 even). */
static void scaling(const char *name, unsigned (*f)(unsigned), unsigned e0, int halve) {
    unsigned long long mismatches = 0, first = 0;
    for (unsigned e = 1; e < 255; e++) {
        if (halve && (e - e0) % 2) continue;
        for (unsigned q = 0; q < 64; q++) {
            unsigned p = q * 64 + 13, low = q * 31 % 2048;
            unsigned at0 = f(e0 << 23 | p << 11 | low), at = f(e << 23 | p << 11 | low);
            int shift = halve ? -((int)e - (int)e0) / 2 : -((int)e - (int)e0);
            int ex = (int)((at0 >> 23) & 0xFF) + shift;
            unsigned want = ex >= 1 && ex <= 254 ? (at0 & 0x807FFFFFu) | (unsigned)ex << 23 : 0xFFFFFFFFu;
            if (want != 0xFFFFFFFFu && at != want) {
                if (!mismatches) first = (unsigned long long)e << 32 | (e << 23 | p << 11 | low);
                mismatches++;
            }
        }
    }
    row(name); field("mismatches", mismatches); field("first", first); end();
}

static const unsigned SPECIALS[] = {0x00000000, 0x80000000, 0x7F800000, 0xFF800000, 0x7FC00000, 0x7FA00000,
                                    0x00000001, 0x007FFFFF, 0x00800000, 0x7F7FFFFF, 0xBF800000, 0x3F800000,
                                    0x00400000, 0x7E800000, 0x7F000000, 0x01000000};

void start(void) {
    tabulate("rcp.table", rcp, 127);
    tabulate("rsqrt.table.even", rsqrt, 127);
    tabulate("rsqrt.table.odd", rsqrt, 128);
    scaling("rcp.scaling", rcp, 127, 0);
    scaling("rsqrt.scaling.even", rsqrt, 127, 1);
    scaling("rsqrt.scaling.odd", rsqrt, 128, 1);
    for (unsigned i = 0; i < sizeof SPECIALS / sizeof SPECIALS[0]; i++) {
        static char name[32] = "special.";
        unsigned v = SPECIALS[i], k = 8;
        for (int d = 28; d >= 0; d -= 4) name[k++] = "0123456789abcdef"[(v >> d) & 15];
        name[k] = 0;
        row(name); field("rcp", rcp(v)); field("rsqrt", rsqrt(v)); end();
    }
    unsigned r[4];
    __asm__ volatile("cpuid" : "=a"(r[0]), "=b"(r[1]), "=c"(r[2]), "=d"(r[3]) : "a"(1), "c"(0));
    row("cpu.signature"); field("eax", r[0]); end();
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
