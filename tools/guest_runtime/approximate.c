// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* approximate.c — RCPPS, RCPSS, RSQRTPS and RSQRTSS as the reference CPU computes them, run natively (Sprint 068 item
 * C: a 10-billion-instruction frontier with worker threads on time ran RSQRTPS 18,041 and RCPPS 6,985 times in the
 * model). The rules are src/w2m/approximate.py's, line for line; the tables are specs/approximations.json's (one Intel
 * Xeon 8573C), generated into approximation_tables.h. Translated code calls w2m_approximate with the destination and
 * the source in the monitor data page (vm_backend.NATIVE_OPERANDS); these instructions raise no SIMD floating-point
 * exception (SDM), so MXCSR is not read or written. */
#include "runtime.h"
#include "approximation_tables.h"

#define DEFAULT_NAN 0xFFC00000u

/* zeros and denormals give an infinity of their sign, infinities a signed zero (RSQRT of -inf the default NaN), NaNs
 * are quieted, negative RSQRT inputs give the default NaN; 1 when x is one of these */
static int special(u32 x, int rsqrt, u32 *out) {
    u32 sign = x >> 31, e = (x >> 23) & 0xFF, m = x & 0x7FFFFF;
    if (e == 0xFF && m) { *out = x | 0x400000; return 1; }
    if (e == 0xFF) { *out = rsqrt && sign ? DEFAULT_NAN : sign << 31; return 1; }
    if (e == 0) { *out = sign << 31 | 0x7F800000; return 1; }
    if (rsqrt && sign) { *out = DEFAULT_NAN; return 1; }
    return 0;
}

static u32 rcp(u32 x) {
    u32 s;
    if (special(x, 0, &s)) return s;
    u32 t = w2m_rcp[(x >> 11) & 0xFFF];
    int e = (int)((t >> 23) & 0xFF) - ((int)((x >> 23) & 0xFF) - 127);
    return (x & 0x80000000u) | (1 <= e && e <= 254 ? (t & 0x7FFFFF) | (u32)e << 23 : 0);
}

static u32 rsqrt(u32 x) {
    u32 s;
    if (special(x, 1, &s)) return s;
    int e_in = (int)((x >> 23) & 0xFF);
    int e0 = ((e_in - 127) & 1) == 0 ? 127 : 128;
    u32 t = (e0 == 127 ? w2m_rsqrt_even : w2m_rsqrt_odd)[(x >> 11) & 0xFFF];
    int e = (int)((t >> 23) & 0xFF) - (e_in - e0) / 2;
    return 1 <= e && e <= 254 ? (t & 0x7FFFFF) | (u32)e << 23 : 0;
}

/* lanes 0..n-1 of out from in's; RCPSS and RSQRTSS pass n = 1 and keep the destination's other lanes */
void w2m_approximate(u32 *out, const u32 *in, u64 is_rsqrt, u64 n) {
    for (u64 i = 0; i < n; i++) out[i] = is_rsqrt ? rsqrt(in[i]) : rcp(in[i]);
}
