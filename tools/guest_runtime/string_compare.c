/* SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 * Copyright (C) 2026 Peter Laffey
 *
 * PCMPISTRI run natively (2026-10-13, the speed work): translated code calls w2m_pcmpistri instead of leaving the
 * VM for the Python model (hvc #10), which cost about 1.3 ms an instruction and 2,129 s of the frontier's host time
 * (UnityPlayer's strstr: PCMPISTRI xmm, xmm, 0x0C at 0x18140b914). The same rules as src/w2m/string_compare.py,
 * which the class runs judge against Intel's CPU (Sprint 047): Intel SDM Vol. 2A §4.1 and Vol. 2B, PCMPISTRI.
 * imm8 bits 1:0 the elements (unsigned bytes, unsigned words, signed bytes, signed words), bits 3:2 the
 * aggregation, bits 5:4 the polarity, bit 6 the index's end. Lengths are implicit: an element's index before the
 * first zero element. The result: ECX in bits 4:0, then CF bit 8, ZF bit 9, SF bit 10, OF bit 11 (AF and PF 0).
 * The runtime is built with -mgeneral-regs-only, so this touches no vector register the translator holds XMM in.
 */
/* No include: the test builds this file alone for the Mac too, and a family's candidate hash follows every include
 * as a runtime header. */
typedef unsigned char u8;
typedef unsigned int u32;
typedef unsigned long long u64;

u64 w2m_pcmpistri(const u8 *a, const u8 *b, u64 imm);

enum { EQUAL_ANY, RANGES, EQUAL_EACH, EQUAL_ORDERED };

static long element(const u8 *v, u32 i, u64 imm) {
    if (imm & 1) {                                      /* words */
        u32 w = (u32)v[2 * i] | (u32)v[2 * i + 1] << 8;
        return (imm & 2) ? (long)(short)w : (long)w;
    }
    return (imm & 2) ? (long)(signed char)v[i] : (long)v[i];
}

static u32 implicit_length(const u8 *v, u64 imm, u32 n) {
    for (u32 i = 0; i < n; i++)
        if (element(v, i, imm) == 0) return i;
    return n;
}

/* SDM Table 4-7: xmm1's element valid1, xmm2's valid2. */
static int override(u32 aggregation, int valid1, int valid2, int result) {
    if (valid1 && valid2) return result;
    if (!valid1 && !valid2) return aggregation == EQUAL_EACH || aggregation == EQUAL_ORDERED;
    if (!valid1) return aggregation == EQUAL_ORDERED;
    return 0;
}

/* xmm2's element j against xmm1's element i, with the overrides */
static int over(const u8 *a, const u8 *b, u64 imm, u32 aggregation, u32 len1, u32 len2, u32 j, u32 i) {
    int result;
    if (aggregation == RANGES)
        result = (i % 2 == 0) ? element(b, j, imm) >= element(a, i, imm) : element(b, j, imm) <= element(a, i, imm);
    else
        result = element(b, j, imm) == element(a, i, imm);
    return override(aggregation, i < len1, j < len2, result);
}

u64 w2m_pcmpistri(const u8 *a, const u8 *b, u64 imm) {
    u32 n = (imm & 1) ? 8 : 16;
    u32 aggregation = (u32)(imm >> 2 & 3);
    u32 len1 = implicit_length(a, imm, n), len2 = implicit_length(b, imm, n);
    u32 res1 = 0;
    for (u32 j = 0; j < n; j++) {
        int bit = 0;
        if (aggregation == EQUAL_ANY) {
            for (u32 i = 0; i < n && !bit; i++) bit = over(a, b, imm, aggregation, len1, len2, j, i);
        } else if (aggregation == RANGES) {
            for (u32 i = 0; i < n && !bit; i += 2)
                bit = over(a, b, imm, aggregation, len1, len2, j, i) && over(a, b, imm, aggregation, len1, len2, j, i + 1);
        } else if (aggregation == EQUAL_EACH) {
            bit = over(a, b, imm, aggregation, len1, len2, j, j);
        } else {
            bit = 1;
            for (u32 i = 0; i < n - j && bit; i++) bit = over(a, b, imm, aggregation, len1, len2, j + i, i);
        }
        res1 |= (u32)bit << j;
    }
    u32 full = (1u << n) - 1, polarity = (u32)(imm >> 4 & 3), res2 = res1;
    if (polarity == 1) res2 = res1 ^ full;
    else if (polarity == 3) res2 = res1 ^ ((1u << len2) - 1);
    u32 ecx = n;
    if (res2) {
        if (imm >> 6 & 1) { ecx = 31; while (!(res2 >> ecx & 1)) ecx--; }
        else { ecx = 0; while (!(res2 >> ecx & 1)) ecx++; }
    }
    u64 flags = (u64)(res2 != 0) | (u64)(len2 < n) << 1 | (u64)(len1 < n) << 2 | (u64)(res2 & 1) << 3;
    return ecx | flags << 8;
}
