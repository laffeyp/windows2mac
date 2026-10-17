// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* x87_fast.c — FLD m64, FSTP m64 and FSQRT in the VM, where the result is exact (Sprint 098).
 *
 * The batch run made 6,612 FLD, 6,612 FSTP and 6,508 FSQRT model steps (hvc #11), mono's JIT code taking square roots
 * through the x87 stack. vm_backend.py compile_insn calls these from the block; each answers 0, changing nothing,
 * whenever the case is not one it computes exactly, and the block then takes the model step as before
 * (src/w2m/x87.py is the specification and the class runs on the Xeon the judge). The floating-point work of FSQRT
 * is the block's own (FSQRT and FMSUB on V30 and V31, FPSR saved round them): this file is built with
 * -mgeneral-regs-only and touches no vector register, so the guest's XMM registers in V0-V15 survive a call.
 *
 * The x87 state (vm_backend.py x87()): the control word at the monitor page's FCW_OFF; the status word, the abridged
 * tags (one bit a physical register, 1 for not empty) in the state block; ST(0)-ST(7) in FXSAVE's order, ST(i) at
 * X87_ST_OFF + 16 i, 10 bytes each (Intel SDM Vol 1 ch. 8). A push makes the slots [new, ST0..ST6]; a pop makes them
 * [ST1..ST7, ST0], the popped register keeping its bits as FXSAVE shows them.
 */
#include "runtime.h"

#define IE 0x1u
#define PE 0x20u
#define C1 0x200u
#define ES 0x80u
#define J (1ull << 63)

struct x87_reg { u64 m; unsigned short se; };     /* significand, then sign and exponent (10 bytes of a slot) */

static unsigned short *fcw(void) { return (unsigned short *)(W2M_MONITOR_DATA_VA + W2M_FCW_OFF); }
static unsigned short *fsw(void) { return &w2m_state()->x87_status_word; }
static u8 *tags(void) { return &w2m_state()->x87_tag_abridged; }
static u8 *slot(int i) { return (u8 *)W2M_STATE_VA + W2M_X87_ST_OFF + 16 * i; }
static u32 top(void) { return *fsw() >> 11 & 7; }

static struct x87_reg read_slot(int i) {
    struct x87_reg r;
    __builtin_memcpy(&r.m, slot(i), 8);
    __builtin_memcpy(&r.se, slot(i) + 8, 2);
    return r;
}

static void write_slot(int i, struct x87_reg r) {
    __builtin_memcpy(slot(i), &r.m, 8);
    __builtin_memcpy(slot(i) + 8, &r.se, 2);
}

static int empty(u32 physical) { return !(*tags() >> (physical & 7) & 1); }

/* every exception masked and none pending: an unmasked one has no rows (x87.py raises X87Unmasked) */
static int quiet_state(void) { return (*fcw() & 0x3F) == 0x3F && !(*fsw() & ES); }

/* a double's bits as 80 bits, for a normal or a zero double; 0 for anything else */
static int from_double(u64 bits, struct x87_reg *r) {
    u32 s = (u32)(bits >> 63), e = (u32)(bits >> 52 & 0x7FF);
    u64 m = bits & ((1ull << 52) - 1);
    if (e == 0x7FF || (e == 0 && m)) return 0;       /* NaN, infinity, a denormal (DE): the model */
    if (e == 0) {
        r->m = 0;
        r->se = (unsigned short)(s << 15);
    } else {
        r->m = J | m << 11;
        r->se = (unsigned short)(s << 15 | (e - 1023 + 16383));
    }
    return 1;
}

/* FLD m64: push the double (already loaded by the block, so its fault came first). */
u64 w2m_x87_fld64(u64 bits) {
    struct x87_reg r;
    if (!quiet_state() || !from_double(bits, &r) || !empty(top() - 1)) return 0;   /* ST(7) in use: a stack fault */
    for (int i = 7; i > 0; i--) write_slot(i, read_slot(i - 1));
    write_slot(0, r);
    u32 t = (top() - 1) & 7;
    *fsw() = (unsigned short)((*fsw() & ~(7u << 11) & ~C1) | t << 11);
    *tags() |= (u8)(1u << t);
    return 1;
}

/* ST(0) as a double rounded to nearest, its flags (PE, and C1 for a magnitude rounded up) at out[1]; 0 where the
 * store would overflow, underflow, meet a NaN, an infinity, an unsupported pattern, an empty ST(0) or a rounding
 * mode other than nearest. Nothing changes: the block stores, then commits. */
u64 w2m_x87_st0_double(u64 *out) {
    if (!quiet_state() || (*fcw() >> 10 & 3) || empty(top())) return 0;
    struct x87_reg x = read_slot(0);
    u32 s = x.se >> 15, e = x.se & 0x7FFF;
    if (e == 0) {
        if (x.m) return 0;                           /* a denormal or pseudo-denormal */
        out[0] = (u64)s << 63;
        out[1] = 0;
        return 1;
    }
    if (e == 0x7FFF || !(x.m & J)) return 0;         /* infinity, NaN, an unnormal */
    u64 kept = x.m >> 11, rest = x.m & 0x7FF, flags = 0;
    if (rest) {
        flags = PE;
        if (rest > 0x400 || (rest == 0x400 && (kept & 1))) {       /* nearest, ties to even */
            kept++;
            flags |= C1;
        }
    }
    s32 ex = (s32)e - 16383;
    if (kept >> 53) {                                /* rounding carried into a new bit */
        kept >>= 1;
        ex++;
    }
    if (ex > 1023 || ex < -1022) return 0;           /* overflow, or a denormal or underflowed result */
    out[0] = (u64)s << 63 | (u64)(ex + 1023) << 52 | (kept & ((1ull << 52) - 1));
    out[1] = flags;
    return 1;
}

/* FSTP's end after its store: C1 from this instruction, the exception flags sticky, then the pop. */
u64 w2m_x87_fstp_commit(u64 flags) {
    u32 t = top();
    *fsw() = (unsigned short)((*fsw() & ~C1) | (flags & (C1 | PE)));
    struct x87_reg first = read_slot(0);
    for (int i = 0; i < 7; i++) write_slot(i, read_slot(i + 1));
    write_slot(7, first);
    *tags() &= (u8)~(1u << t);
    *fsw() = (unsigned short)((*fsw() & ~(7u << 11)) | ((t + 1) & 7) << 11);
    return 1;
}

/* FSQRT's input: ST(0) as a double when it is one exactly, positive or zero, with double precision (PC 10) and
 * rounding to nearest in both the control word and MXCSR (the block's FSQRT rounds in FPCR, which MXCSR's RC sets):
 * then the block's FSQRT gives the result x87.py's sqrt rounds to 53 bits. 0 otherwise. */
u64 w2m_x87_st0_exact(u64 *out) {
    u32 cw = *fcw();
    u32 mxcsr = *(const u32 *)(W2M_MONITOR_DATA_VA + W2M_MXCSR_OFF);
    if (!quiet_state() || (cw >> 8 & 3) != 2 || (cw >> 10 & 3) || (mxcsr >> 13 & 3) || empty(top())) return 0;
    struct x87_reg x = read_slot(0);
    u32 e = x.se & 0x7FFF;
    if (x.se >> 15) return 0;                       /* negative (or -0, which the model gives back as is) */
    if (e == 0) {
        if (x.m) return 0;
        out[0] = 0;
        return 1;
    }
    s32 ex = (s32)e - 16383;
    if (e == 0x7FFF || !(x.m & J) || (x.m & 0x7FF) || ex > 1023 || ex < -1022) return 0;
    out[0] = (u64)(ex + 1023) << 52 | (x.m >> 11 & ((1ull << 52) - 1));
    return 1;
}

/* FSQRT's end: the root r at out[0] and the remainder x - r*r at out[1] (fused, exact) from the block. A zero
 * remainder: exact; a negative one: r is above the root, rounded up (C1); a positive one: rounded down. */
u64 w2m_x87_sqrt_commit(const u64 *out) {
    struct x87_reg r;
    if (!from_double(out[0], &r)) return 0;          /* not reached: the root of a normal double is normal */
    u64 rem = out[1];
    u32 flags = 0;
    if (rem << 1) flags = (rem >> 63) ? PE | C1 : PE;
    write_slot(0, r);
    *fsw() = (unsigned short)((*fsw() & ~C1) | flags);
    return 1;
}
