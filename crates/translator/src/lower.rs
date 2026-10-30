// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! One lifted instruction to AArch64 words (Sprint 151), carried from `src/w2m/vm_backend.py` `compile_insn`
//! (1417-1885) and the helpers it calls: the flag setters (482-519, 559-710), the NZCV conditions and flag liveness
//! (815-920), the condition reads (973-1010), the temporaries (1013-1060), operand forms, address folding and fused
//! moves (1186-1348). The ops carried are those link 5's first group lifts; any other op, or a flags kind no rule of the
//! group emits, is a `CompileError` naming it. Each item cites the lines it carries.
//!
//! The constants the Python switched (FLAGS_FROM_NZCV, FOLD_ADDRESSES, OPERAND_FORMS, FUSE_LOAD_STORE, FLAG_LIVENESS)
//! were all on in the frozen host, and the C stencils were derived from that code: here they are always on.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use a64_encoder::assembler::{Assembler, Target};
use a64_encoder::{self as a64, LSL, LSR, ZR};

use crate::lift::{Lifted, Loc, Op, SseLoc, Temp};

/// vm_backend 357-371.
pub const TEMP_REGS: [u32; 6] = [18, 25, 26, 27, 29, 30];
pub const REG_CF: u32 = 19;
pub const REG_ZF: u32 = 20;
pub const REG_SF: u32 = 21;
pub const REG_OF: u32 = 22;
pub const REG_PF: u32 = 23;
pub const REG_UNDEFINED_FLAGS: u32 = 24;
pub const U_CF: u32 = 1;
pub const U_ZF: u32 = 2;
pub const U_SF: u32 = 4;
pub const U_OF: u32 = 8;
pub const U_PF: u32 = 16;
pub const U_AF: u32 = 32;
/// vm_backend 851.
pub const ALL_FLAGS: u32 = U_CF | U_ZF | U_SF | U_OF | U_PF | U_AF;
/// MONITOR_DATA + these, reached through x28 (vm_backend 89, 93, 94).
const AF_OPERANDS: u32 = 0x20;
const PF_RESULT: u32 = AF_OPERANDS + 16;
/// vm_backend 1125: one instruction run whole by the model.
pub const HVC_SLOW_STEP: u32 = 11;
/// HVC_SLOW_SSE (vm_backend 1118): one SSE instruction run by the model.
pub const HVC_SLOW_SSE: u32 = 10;
/// FLAGS_SSE (vm_backend 856): the SSE classes that write all six flags.
const FLAGS_SSE: [&str; 6] = ["COMISS", "UCOMISS", "COMISD", "UCOMISD", "PCMPISTRI", "PCMPISTRI64"];
/// vm_backend 97: the state block's gs_base (the TEB).
const STATE_GS_BASE: u64 = 0xb0;
// MONITOR_DATA (x28) + these (vm_backend 94-119): DF, MXCSR's saved value, the x87 control word, the register a lowering
// borrows, the PAUSE count, the slice's limit
const DF_BYTE: u32 = 0x38;
const MXCSR_OFF: u32 = 0x50;
const FCW_OFF: u32 = 0x70;
const SPILL: u32 = 0x78;
const PAUSE_COUNT: u32 = 0xA8;
const SLICE_UNTIL: u32 = 0xC0;
/// NATIVE_OPERANDS (vm_backend 126): two 16-byte operands for a native helper call (PCMPISTRI, the approximations).
const NATIVE_OPERANDS: u32 = 0x300;
/// MONITOR_VA + RUNTIME_STACK + RUNTIME_STACK_SIZE (vm_backend 81, 136-143): the top of the runtime's AArch64 stack,
/// where CPUID's call into the guest runtime runs.
pub const RUNTIME_STACK_TOP: u64 = 0x7F00_0000_0000 + 0x60000 + 0x400_0000 + 0x100000 + 0x10000;
/// The guest's registers in the A64 registers of the same index (vm_backend's reconstruction): RAX, RCX, RDX, RBX,
/// RSI, RDI.
const RAX: u32 = 0;
const RCX: u32 = 1;
const RDX: u32 = 2;
const RSI: u32 = 6;
const RDI: u32 = 7;
/// REG_BUDGET (vm_backend 368): the instructions left in the slice (x23, PF's until PF went lazy).
const REG_BUDGET: u32 = 23;
/// vm_backend 1124: the NZCV kinds by `vmc_flags.nzcv_from`'s index (vm_compiler.h 25): [none, NZCV_CONDITIONS...].
pub const NZCV_KINDS: [&str; 5] = ["sub", "add", "inc", "logic", "dec"];

/// vm_backend 392: an operation or form the compiler cannot lower.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// refused, in words
    Refused(String),
    /// a lowering of a later group, refused by name: its class, which the model steps meanwhile (Sprint 179;
    /// reviewer-2's High 2 of its review of 175-177: the judges matched this by substring)
    LaterGroup { class: String, why: String },
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::Refused(why) | CompileError::LaterGroup { why, .. } => f.write_str(why),
        }
    }
}

impl std::error::Error for CompileError {}

fn bit(f: char) -> u32 {
    match f {
        'C' => 0,
        'Z' => 1,
        'S' => 2,
        'O' => 3,
        'P' => 4,
        _ => 5,
    }
}

fn flag_reg(f: char) -> u32 {
    match f {
        'C' => REG_CF,
        'Z' => REG_ZF,
        'S' => REG_SF,
        'O' => REG_OF,
        _ => REG_PF,
    }
}

/// `_mask` (482-486).
fn mask(a: &mut Assembler, r: u32, bits: u32) {
    if bits == 32 {
        a.emit(&[a64::mov32(r, r)]);
    } else if bits < 64 {
        a.emit(&[a64::ubfx(r, r, 0, bits)]);
    }
}

/// `_nzcv` (497-509): NZCV as the add or subtract of `bits`-wide ra and rb sets it; an 8- or 16-bit operation runs in
/// the top of a 32-bit one. Uses x16 for the narrow forms.
fn nzcv(a: &mut Assembler, add: bool, ra: u32, rb: u32, bits: u32) {
    if bits == 64 {
        a.emit(&[if add {
            a64::adds(ZR, ra, rb)
        } else {
            a64::subs(ZR, ra, rb)
        }]);
    } else if bits == 32 {
        a.emit(&[if add {
            a64::adds32(ZR, ra, rb, 0)
        } else {
            a64::subs32(ZR, ra, rb, 0)
        }]);
    } else {
        let s = 32 - bits;
        a.emit(&[
            a64::lsl32(16, ra, s),
            if add {
                a64::adds32(ZR, 16, rb, s)
            } else {
                a64::subs32(ZR, 16, rb, s)
            },
        ]);
    }
}

/// `_adds_subs` (512-515).
fn adds_subs(add: bool, rd: u32, rn: u32, rm: u32, bits: u32) -> u32 {
    let w = if add {
        a64::adds(rd, rn, rm)
    } else {
        a64::subs(rd, rn, rm)
    };
    if bits == 64 { w } else { w & !(1 << 31) }
}

/// `_set_pf` (559-561): PF lazily, the result stored at PF_RESULT.
fn set_pf(a: &mut Assembler, r: u32) {
    a.emit(&[a64::str(8, r, 28, PF_RESULT)]);
}

/// `_pf_into_x16` (569-573).
fn pf_into_x16(a: &mut Assembler) {
    a.emit(&[
        a64::ldr(8, 16, 28, PF_RESULT),
        a64::ubfx(16, 16, 0, 8),
        a64::eor(16, 16, 16, LSR, 4),
        a64::eor(16, 16, 16, LSR, 2),
        a64::eor(16, 16, 16, LSR, 1),
        a64::ubfx(16, 16, 0, 1),
        a64::cmpi(16, 0),
        a64::cset(16, "eq"),
    ]);
}

/// `_mark_defined` (489-490).
fn mark_defined(a: &mut Assembler, bits: u32) {
    a.emit(&[
        a64::movz(16, u64::from(bits), 0),
        a64::bic(REG_UNDEFINED_FLAGS, REG_UNDEFINED_FLAGS, 16),
    ]);
}

/// `_mark_undefined` (493-494).
fn mark_undefined(a: &mut Assembler, bits: u32) {
    a.emit(&[
        a64::movz(16, u64::from(bits), 0),
        a64::orr(REG_UNDEFINED_FLAGS, REG_UNDEFINED_FLAGS, 16),
    ]);
}

/// `_set_zf_sf_pf` (578-580).
fn set_zf_sf_pf(a: &mut Assembler, r: u32, bits: u32) {
    a.emit(&[
        a64::cmpi(r, 0),
        a64::cset(REG_ZF, "eq"),
        a64::ubfx(REG_SF, r, bits - 1, 1),
    ]);
    set_pf(a, r);
}

/// `_shift_flags` (712-770): SHR SHL SAR ROL ROR's flags (SDM SAL/SAR/SHL/SHR, RCL/RCR/ROL/ROR): a zero masked count
/// changes nothing; CF is the last bit out (undefined for SHL and SHR when the count >= the size); OF only for a 1-bit
/// count; shifts set ZF SF PF and leave AF undefined; rotates touch only CF and OF.
fn shift_flags(a: &mut Assembler, kind: &str, ra: u32, rb: u32, rr: u32, bits: u32) {
    let (done, of_undef, after_of) = (a.label(), a.label(), a.label());
    let width = if bits == 64 { 6 } else { 5 };
    a.emit(&[a64::ubfx(16, rb, 0, width)]);
    a.branch(|off| a64::cbz(16, off), done);
    if kind == "rol" || kind == "ror" {
        a.emit(&[if kind == "rol" {
            a64::ubfx(REG_CF, rr, 0, 1)
        } else {
            a64::ubfx(REG_CF, rr, bits - 1, 1)
        }]);
        mark_defined(a, U_CF); // (uses x16: re-read the count below)
        a.emit(&[a64::ubfx(16, rb, 0, width), a64::cmpi(16, 1)]);
        a.branch(|off| a64::bcond("ne", off), of_undef);
        a.emit(&[a64::ubfx(16, rr, bits - 1, 1)]);
        a.emit(&[if kind == "rol" {
            a64::eor(REG_OF, 16, REG_CF, LSL, 0)
        } else {
            a64::ubfx(17, rr, bits - 2, 1)
        }]);
        if kind == "ror" {
            a.emit(&[a64::eor(REG_OF, 16, 17, LSL, 0)]);
        }
        mark_defined(a, U_OF);
        a.branch(a64::b, done);
        a.bind(of_undef);
        mark_undefined(a, U_OF);
        a.bind(done);
        return;
    }
    let cf_undef = a.label();
    if kind == "sar" {
        // CF from the sign-extended value, any count
        a.emit(&[
            a64::sbfx(17, ra, 0, bits),
            a64::subi(16, 16, 1),
            a64::asrv(17, 17, 16),
            a64::addi(16, 16, 1),
            a64::ubfx(REG_CF, 17, 0, 1),
        ]);
        mark_defined(a, U_CF);
    } else {
        a.emit(&[a64::cmpi(16, bits)]);
        a.branch(|off| a64::bcond("hs", off), cf_undef); // count >= size: CF undefined (SHL, SHR)
        if kind == "shr" {
            a.emit(&[a64::subi(17, 16, 1), a64::lsrv(17, ra, 17)]);
        } else {
            // SHL: bit (size - count) of the operand
            a.emit(&[a64::sub(17, ZR, 16), a64::addi(17, 17, bits), a64::lsrv(17, ra, 17)]);
        }
        a.emit(&[a64::ubfx(REG_CF, 17, 0, 1)]);
        mark_defined(a, U_CF);
    }
    a.emit(&[a64::ubfx(16, rb, 0, width), a64::cmpi(16, 1)]); // _mark_defined used x16
    a.branch(|off| a64::bcond("ne", off), of_undef);
    if kind == "shr" {
        a.emit(&[a64::ubfx(REG_OF, ra, bits - 1, 1)]);
    } else if kind == "shl" {
        a.emit(&[a64::ubfx(17, rr, bits - 1, 1), a64::eor(REG_OF, 17, REG_CF, LSL, 0)]);
    } else {
        a.emit(&[a64::mov(REG_OF, ZR)]); // SAR: OF cleared for a 1-bit shift
    }
    mark_defined(a, U_OF);
    a.branch(a64::b, after_of);
    a.bind(cf_undef);
    mark_undefined(a, U_CF);
    a.bind(of_undef);
    mark_undefined(a, U_OF);
    a.bind(after_of);
    mark_defined(a, U_ZF | U_SF | U_PF);
    mark_undefined(a, U_AF);
    set_zf_sf_pf(a, rr, bits);
    a.bind(done);
}

/// `_shift_flags_const` (773-812): `_shift_flags` for a count known when compiling: the same rules, decided here, and
/// the undefined-flag mask set in one instruction where every flag's state is known.
fn shift_flags_const(a: &mut Assembler, kind: &str, ra: u32, count: u64, rr: u32, bits: u32) {
    let m = (count & if bits == 64 { 63 } else { 31 }) as u32;
    if m == 0 {
        return; // a zero masked count changes nothing
    }
    if kind == "rol" || kind == "ror" {
        // rotates touch only CF and OF
        a.emit(&[if kind == "rol" {
            a64::ubfx(REG_CF, rr, 0, 1)
        } else {
            a64::ubfx(REG_CF, rr, bits - 1, 1)
        }]);
        if m == 1 {
            a.emit(&[a64::ubfx(16, rr, bits - 1, 1)]);
            a.emit(&[if kind == "rol" {
                a64::eor(REG_OF, 16, REG_CF, LSL, 0)
            } else {
                a64::ubfx(17, rr, bits - 2, 1)
            }]);
            if kind == "ror" {
                a.emit(&[a64::eor(REG_OF, 16, 17, LSL, 0)]);
            }
            mark_defined(a, U_CF | U_OF);
        } else {
            mark_defined(a, U_CF);
            mark_undefined(a, U_OF);
        }
        return;
    }
    let mut undefined = U_AF | if m == 1 { 0 } else { U_OF };
    if kind == "sar" {
        // CF from the sign-extended value, any count
        if bits < 64 {
            a.emit(&[a64::sbfx(17, ra, 0, bits), a64::ubfx(REG_CF, 17, (m - 1).min(63), 1)]);
        } else {
            a.emit(&[a64::ubfx(REG_CF, ra, m - 1, 1)]);
        }
    } else if m >= bits {
        undefined |= U_CF; // SHL, SHR: count >= size, CF undefined
    } else if kind == "shr" {
        a.emit(&[a64::ubfx(REG_CF, ra, m - 1, 1)]);
    } else {
        a.emit(&[a64::ubfx(REG_CF, ra, bits - m, 1)]); // SHL: bit (size - count) of the operand
    }
    if m == 1 {
        if kind == "shr" {
            a.emit(&[a64::ubfx(REG_OF, ra, bits - 1, 1)]);
        } else if kind == "shl" {
            a.emit(&[a64::ubfx(16, rr, bits - 1, 1), a64::eor(REG_OF, 16, REG_CF, LSL, 0)]);
        } else {
            a.emit(&[a64::mov(REG_OF, ZR)]); // SAR: OF cleared for a 1-bit shift
        }
    }
    a.emit(&[a64::movz(REG_UNDEFINED_FLAGS, u64::from(undefined), 0)]);
    set_zf_sf_pf(a, rr, bits);
}

/// `_set_flags` (583-709), with FLAGS_FROM_NZCV on: every kind the rules emit. `live` holds the flags a later
/// instruction reads; `nzcv_out` asks for NZCV as well (the next instruction's Jcc branches on it).
#[allow(clippy::too_many_arguments)]
fn set_flags(
    a: &mut Assembler,
    kind: &str,
    ra: u32,
    rb: u32,
    rr: u32,
    bits: u32,
    nzcv_ready: bool,
    live: u32,
    nzcv_out: bool,
) -> Result<(), CompileError> {
    match kind {
        "mul" => {
            // SDM MUL: SF ZF AF PF undefined
            a.emit(&[a64::cmpi(rr, 0), a64::cset(REG_CF, "ne"), a64::mov(REG_OF, REG_CF)]);
            mark_defined(a, U_CF | U_OF);
            mark_undefined(a, U_ZF | U_SF | U_PF | U_AF);
            return Ok(());
        }
        "shr" | "shl" | "sar" | "rol" | "ror" => {
            shift_flags(a, kind, ra, rb, rr, bits);
            return Ok(());
        }
        "bt" => {
            // SDM BT: CF the bit; OF SF AF PF undefined; ZF kept
            a.emit(&[a64::ubfx(REG_CF, rr, 0, 1)]);
            mark_defined(a, U_CF);
            mark_undefined(a, U_OF | U_SF | U_AF | U_PF);
            return Ok(());
        }
        "popcnt" => {
            // SDM POPCNT: ZF the source was 0; CF OF SF AF PF 0
            a.emit(&[
                if bits < 64 {
                    a64::ubfx(16, ra, 0, bits)
                } else {
                    a64::mov(16, ra)
                },
                a64::cmpi(16, 0),
                a64::cset(REG_ZF, "eq"),
                a64::mov(REG_CF, ZR),
                a64::mov(REG_OF, ZR),
                a64::mov(REG_SF, ZR),
                a64::movz(16, 1, 0),
                a64::stp(ZR, ZR, 28, i64::from(AF_OPERANDS)),
                a64::str(8, 16, 28, PF_RESULT), // PF_RESULT 1: PF clear
                a64::movz(REG_UNDEFINED_FLAGS, 0, 0),
            ]);
            return Ok(());
        }
        "bsf" => {
            // SDM BSF/BSR: ZF the source was 0; the rest undefined
            a.emit(&[
                if bits < 64 {
                    a64::ubfx(16, ra, 0, bits)
                } else {
                    a64::mov(16, ra)
                },
                a64::cmpi(16, 0),
                a64::cset(REG_ZF, "eq"),
            ]);
            mark_defined(a, U_ZF);
            mark_undefined(a, U_CF | U_OF | U_SF | U_AF | U_PF);
            return Ok(());
        }
        "adc" | "sbb" => {
            // SDM ADC, SBB: carry (borrow) and overflow from a, b, r
            if kind == "adc" {
                a.emit(&[
                    a64::eor(16, ra, rb, LSL, 0),
                    a64::eor(17, ra, rr, LSL, 0),
                    a64::bic(17, 17, 16),
                    a64::ubfx(REG_OF, 17, bits - 1, 1),
                ]);
                a.emit(&[
                    a64::and(16, ra, rb),
                    a64::orr(17, ra, rb),
                    a64::bic(17, 17, rr),
                    a64::orr(16, 16, 17),
                    a64::ubfx(REG_CF, 16, bits - 1, 1),
                ]);
            } else {
                a.emit(&[
                    a64::eor(16, ra, rb, LSL, 0),
                    a64::eor(17, ra, rr, LSL, 0),
                    a64::and(17, 17, 16),
                    a64::ubfx(REG_OF, 17, bits - 1, 1),
                ]);
                a.emit(&[
                    a64::bic(16, rb, ra),
                    a64::eor(17, ra, rb, LSL, 0),
                    a64::bic(17, rr, 17),
                    a64::orr(16, 16, 17),
                    a64::ubfx(REG_CF, 16, bits - 1, 1),
                ]);
            }
            mark_defined(a, U_CF | U_ZF | U_SF | U_OF | U_PF);
            a.emit(&[
                a64::stp(ra, rb, 28, i64::from(AF_OPERANDS)),
                a64::str(8, rr, 28, AF_OPERANDS + 16),
            ]);
            mark_defined(a, U_AF);
            set_zf_sf_pf(a, rr, bits);
            return Ok(());
        }
        "imul" => {
            // SDM IMUL: CF=OF on truncation; SF ZF AF PF undefined
            if bits < 64 {
                a.emit(&[
                    a64::sbfx(16, ra, 0, bits),
                    a64::sbfx(17, rb, 0, bits),
                    a64::mul(16, 16, 17),
                    a64::sbfx(17, 16, 0, bits),
                ]);
            } else {
                a.emit(&[a64::smulh(16, ra, rb), a64::sbfx(17, rr, 63, 1)]);
            }
            a.emit(&[a64::cmp(16, 17), a64::cset(REG_CF, "ne"), a64::mov(REG_OF, REG_CF)]);
            mark_defined(a, U_CF | U_OF);
            mark_undefined(a, U_ZF | U_SF | U_PF | U_AF);
            return Ok(());
        }
        "andn" => {
            // SDM ANDN: SF ZF by the result, CF OF 0, AF PF undefined
            a.emit(&[a64::mov(REG_CF, ZR), a64::mov(REG_OF, ZR)]);
            set_zf_sf_pf(a, rr, bits);
            mark_defined(a, U_CF | U_ZF | U_SF | U_OF);
            mark_undefined(a, U_AF | U_PF);
            return Ok(());
        }
        _ => {}
    }
    if matches!(kind, "add" | "inc" | "sub" | "dec") {
        // NZCV from one ADDS or SUBS gives CF ZF SF OF; INC and DEC keep CF (SDM INC, DEC)
        if !nzcv_ready && (live & (U_CF | U_ZF | U_SF | U_OF) != 0 || nzcv_out) {
            nzcv(a, matches!(kind, "add" | "inc"), ra, rb, bits);
        }
        for (flag, reg, cond) in [(U_ZF, REG_ZF, "eq"), (U_SF, REG_SF, "mi"), (U_OF, REG_OF, "vs")] {
            if live & flag != 0 {
                a.emit(&[a64::cset(reg, cond)]);
            }
        }
        if matches!(kind, "add" | "sub") && live & U_CF != 0 {
            a.emit(&[a64::cset(REG_CF, if kind == "add" { "cs" } else { "lo" })]);
        }
        if live != 0 {
            // MOV X24, XZR: every flag defined; AND X24, X24, #1: every flag defined but CF, which keeps its state
            a.emit(&[if matches!(kind, "add" | "sub") {
                a64::mov(REG_UNDEFINED_FLAGS, ZR)
            } else {
                a64::and_low(REG_UNDEFINED_FLAGS, REG_UNDEFINED_FLAGS, 1)
            }]);
        }
        if live & U_AF != 0 {
            a.emit(&[a64::stp(ra, rb, 28, i64::from(AF_OPERANDS))]); // AF's operands; AF reads the result too
        }
        if live & (U_AF | U_PF) != 0 {
            a.emit(&[a64::str(8, rr, 28, PF_RESULT)]); // PF's result
        }
        return Ok(());
    }
    if kind == "logic" {
        // SDM AND/OR/XOR/TEST: CF OF 0, AF undefined
        if live & U_CF != 0 {
            a.emit(&[a64::mov(REG_CF, ZR)]);
        }
        if live & U_OF != 0 {
            a.emit(&[a64::mov(REG_OF, ZR)]);
        }
        if live != 0 {
            a.emit(&[a64::movz(REG_UNDEFINED_FLAGS, u64::from(U_AF), 0)]);
        }
        if live & U_ZF != 0 || nzcv_out {
            a.emit(&[a64::cmpi(rr, 0)]);
        }
        if live & U_ZF != 0 {
            a.emit(&[a64::cset(REG_ZF, "eq")]);
        }
        if live & U_SF != 0 {
            a.emit(&[a64::ubfx(REG_SF, rr, bits - 1, 1)]);
        }
        if live & U_PF != 0 {
            set_pf(a, rr);
        }
        return Ok(());
    }
    Err(CompileError::Refused(format!("flags kind {kind}: a later group")))
}

/// `NZCV_CONDITIONS` (820-829): a conditional branch right after the flags' setter branches on NZCV itself.
pub fn nzcv_condition(kind: &str, cc: &str) -> Option<&'static str> {
    let sub: &[(&str, &str)] = &[
        ("z", "eq"),
        ("nz", "ne"),
        ("b", "lo"),
        ("nb", "hs"),
        ("be", "ls"),
        ("nbe", "hi"),
        ("l", "lt"),
        ("nl", "ge"),
        ("le", "le"),
        ("nle", "gt"),
        ("s", "mi"),
        ("ns", "pl"),
        ("o", "vs"),
        ("no", "vc"),
    ];
    let add: &[(&str, &str)] = &[
        ("z", "eq"),
        ("nz", "ne"),
        ("b", "cs"),
        ("nb", "cc"),
        ("l", "lt"),
        ("nl", "ge"),
        ("le", "le"),
        ("nle", "gt"),
        ("s", "mi"),
        ("ns", "pl"),
        ("o", "vs"),
        ("no", "vc"),
    ];
    let inc: &[(&str, &str)] = &[
        ("z", "eq"),
        ("nz", "ne"),
        ("l", "lt"),
        ("nl", "ge"),
        ("le", "le"),
        ("nle", "gt"),
        ("s", "mi"),
        ("ns", "pl"),
        ("o", "vs"),
        ("no", "vc"),
    ];
    let logic: &[(&str, &str)] = &[("z", "eq"), ("nz", "ne")];
    let table = match kind {
        "sub" => sub,
        "add" => add,
        "inc" | "dec" => inc,
        "logic" => logic,
        _ => return None,
    };
    table.iter().find(|(k, _)| *k == cc).map(|(_, v)| *v)
}

/// `INVERSE` (830-831): each Arm condition's inverse, named as `a64.COND` first names it, never `cs` or `cc`. A name
/// it does not hold is refused, as the Python's dict raises KeyError (Sprint 166; it gave "gt" before).
fn inverse(c: &str) -> Result<&'static str, CompileError> {
    Ok(match c {
        "eq" => "ne",
        "ne" => "eq",
        "cs" | "hs" => "lo",
        "cc" | "lo" => "hs",
        "mi" => "pl",
        "pl" => "mi",
        "vs" => "vc",
        "vc" => "vs",
        "hi" => "ls",
        "ls" => "hi",
        "ge" => "lt",
        "lt" => "ge",
        "gt" => "le",
        "le" => "gt",
        _ => return Err(CompileError::Refused(format!("no inverse of the condition {c:?}"))),
    })
}

/// `nzcv_setter` (834-839): the flags kind whose NZCV a conditional branch right after `ins` may branch on.
pub fn nzcv_setter(ins: &Lifted) -> Option<&'static str> {
    match ins.ops.last() {
        Some(Op::Flags { kind, .. }) if matches!(*kind, "sub" | "add" | "inc" | "logic" | "dec") => Some(kind),
        _ => None,
    }
}

/// `COND_FLAGS` (852-853). A condition it does not name is refused, as the Python's dict raises KeyError (Sprint 166;
/// it gave `le`'s flags before).
fn cond_flags(cc: &str) -> Result<u32, CompileError> {
    Ok(match cc.strip_prefix('n').unwrap_or(cc) {
        "o" => U_OF,
        "b" => U_CF,
        "z" => U_ZF,
        "s" => U_SF,
        "p" => U_PF,
        "be" => U_CF | U_ZF,
        "l" => U_SF | U_OF,
        "le" => U_ZF | U_SF | U_OF,
        _ => return Err(CompileError::Refused(format!("no flags for the condition {cc:?}"))),
    })
}

/// `FLAGS_WRITTEN` (854-855).
fn flags_written(kind: &str) -> u32 {
    match kind {
        "add" | "sub" | "adc" | "sbb" | "logic" | "andn" | "mul" | "imul" | "bsf" | "popcnt" => ALL_FLAGS,
        "inc" | "dec" => ALL_FLAGS & !U_CF,
        "bt" => ALL_FLAGS & !U_ZF,
        _ => 0,
    }
}

/// `_op_flags` (864-889): (flags read, flags written) by one lifted op.
fn op_flags(op: &Op, constant: &BTreeMap<Temp, u64>) -> Result<(u32, u32), CompileError> {
    Ok(match op {
        Op::Flags {
            kind, b: count, bits, ..
        } => {
            if matches!(*kind, "shr" | "shl" | "sar" | "rol" | "ror") {
                let m = if *bits == 64 { 63 } else { 31 };
                match constant.get(count) {
                    Some(c) if c & m != 0 => (
                        0,
                        if matches!(*kind, "rol" | "ror") {
                            U_CF | U_OF
                        } else {
                            ALL_FLAGS
                        },
                    ),
                    _ => (0, 0),
                }
            } else {
                (0, flags_written(kind))
            }
        }
        Op::Alu { op, .. } => (if matches!(*op, "adc" | "sbb") { U_CF } else { 0 }, 0),
        Op::Br { cond, .. } => (if *cond == "always" { 0 } else { cond_flags(cond)? }, 0),
        Op::Set { cond, .. } | Op::Cmov { cond, .. } => (cond_flags(cond)?, 0),
        Op::Cx16 { .. } => (0, U_ZF),
        // FLAGS_SSE (856): each writes all six
        Op::Sse { ic, .. } => (0, if FLAGS_SSE.contains(&ic.as_str()) { ALL_FLAGS } else { 0 }),
        Op::Div { .. } => (ALL_FLAGS, ALL_FLAGS), // #DE before any write; every flag undefined after
        // FLAGS_ALL_READ (859): the model's steps read the whole state
        Op::Ud | Op::Xgetbv | Op::X86 { .. } | Op::X87 { .. } => (ALL_FLAGS, 0),
        // FLAGS_NONE (857-858)
        Op::Imm { .. }
        | Op::Addr { .. }
        | Op::Ld { .. }
        | Op::St { .. }
        | Op::Sel { .. }
        | Op::Cst { .. }
        | Op::Cstz { .. }
        | Op::Mxcsr { .. }
        | Op::Fence { .. }
        | Op::Rep { .. }
        | Op::Cpuid
        | Op::Df { .. }
        | Op::Pause
        | Op::Fcw { .. }
        | Op::Mmx { .. }
        | Op::Vex { .. }
        | Op::Vexs { .. }
        | Op::Fma { .. }
        | Op::Vzeroupper => (0, 0),
    })
}

/// `_op_flag_masks` (892-904): `_op_flags` of each op; a Jcc that branches on the previous instruction's NZCV reads
/// no flag register.
fn op_flag_masks(ins: &Lifted, nzcv_kind: Option<&str>) -> Result<Vec<(u32, u32)>, CompileError> {
    let mut constant = BTreeMap::new();
    let mut out = Vec::new();
    for op in &ins.ops {
        if let Op::Imm { t, value } = op {
            constant.insert(*t, *value);
        }
        let (mut r, w) = op_flags(op, &constant)?;
        if let Op::Br { cond, .. } = op
            && nzcv_kind.is_some_and(|k| nzcv_condition(k, cond).is_some())
        {
            r = 0;
        }
        out.push((r, w));
    }
    Ok(out)
}

/// `insn_flags` (907-913): (flags `ins` reads before writing them, flags it writes).
pub fn insn_flags(ins: &Lifted, nzcv_kind: Option<&str>) -> Result<(u32, u32), CompileError> {
    let (mut read, mut written) = (0, 0);
    for (r, w) in op_flag_masks(ins, nzcv_kind)? {
        read |= r & !written;
        written |= w;
    }
    Ok((read, written))
}

/// `nzcv_branch` (916-919): `nxt` is a Jcc that branches on `setter`'s NZCV.
pub fn nzcv_branch(setter: &Lifted, nxt: &Lifted) -> bool {
    let Some(kind) = nzcv_setter(setter) else {
        return false;
    };
    nxt.ops
        .iter()
        .any(|op| matches!(op, Op::Br { cond, .. } if nzcv_condition(kind, cond).is_some()))
}

/// `_exit_if_undefined` (973-975): exit with HVC 0x40|bit when flag `f` is undefined.
fn exit_if_undefined(a: &mut Assembler, f: char) {
    a.emit(&[a64::tbz(REG_UNDEFINED_FLAGS, bit(f), 8), a64::hvc(0x40 | bit(f))]);
}

/// `_eval_condition` (978-1010): x16 = condition `cc`, reading flags in `runtime.Runtime._cond`'s order.
fn eval_condition(a: &mut Assembler, cc: &str) -> Result<(), CompileError> {
    let neg = cc.starts_with('n');
    let key = if neg { &cc[1..] } else { cc };
    let single = |k: &str| match k {
        "o" => Some('O'),
        "b" => Some('C'),
        "z" => Some('Z'),
        "s" => Some('S'),
        "p" => Some('P'),
        _ => None,
    };
    if key == "p" {
        exit_if_undefined(a, 'P');
        pf_into_x16(a);
    } else if let Some(f) = single(key) {
        exit_if_undefined(a, f);
        a.emit(&[a64::mov(16, flag_reg(f))]);
    } else if key == "be" || key == "le" {
        let end = a.label();
        let first = if key == "be" { 'C' } else { 'Z' };
        exit_if_undefined(a, first);
        a.emit(&[a64::mov(16, flag_reg(first))]);
        a.branch(|off| a64::cbnz(16, off), end);
        if key == "be" {
            exit_if_undefined(a, 'Z');
            a.emit(&[a64::mov(16, REG_ZF)]);
        } else {
            exit_if_undefined(a, 'S');
            exit_if_undefined(a, 'O');
            a.emit(&[a64::eor(16, REG_SF, REG_OF, LSL, 0)]);
        }
        a.bind(end);
    } else if key == "l" {
        exit_if_undefined(a, 'S');
        exit_if_undefined(a, 'O');
        a.emit(&[a64::eor(16, REG_SF, REG_OF, LSL, 0)]);
    } else {
        return Err(CompileError::Refused(format!("condition {cc}")));
    }
    if neg {
        a.emit(&[a64::cmpi(16, 0), a64::cset(16, "eq")]);
    }
    Ok(())
}

/// `_temps_read` (1013-1055) for the ops carried.
fn temps_read(op: &Op) -> Vec<Temp> {
    match op {
        Op::Ld {
            loc: Loc::M { addr, .. },
            ..
        } => vec![*addr],
        Op::Ld { .. } => vec![],
        Op::St { loc, t } => match loc {
            Loc::M { addr, .. } => vec![*addr, *t],
            Loc::R { .. } => vec![*t],
        },
        Op::Alu { a, b, .. } => vec![*a, *b],
        Op::Flags { a, b, r, .. } => vec![*a, *b, *r],
        Op::Br { t, .. } => vec![*t],
        Op::Div { t, .. } => vec![*t],
        Op::Set { loc, .. } => match loc {
            Loc::M { addr, .. } => vec![*addr],
            Loc::R { .. } => vec![],
        },
        Op::Sel { c, x, y, .. } => vec![*c, *x, *y],
        Op::Cst { c, loc, t } | Op::Cstz { c, loc, t } => match loc {
            Loc::M { addr, .. } => vec![*c, *t, *addr],
            Loc::R { .. } => vec![*c, *t],
        },
        Op::Cmov { loc, t, .. } => match loc {
            Loc::M { addr, .. } => vec![*t, *addr],
            Loc::R { .. } => vec![*t],
        },
        Op::Cx16 { addr } => vec![*addr],
        // ("sse", ic, dst, src, ...): its memory operands' addresses (1015-1016)
        Op::Sse { dst, src, .. } => [dst.addr(), src.addr()].into_iter().flatten().collect(),
        // ("x87", ic, opcode, reg, st, mem): its address (1043-1044); ("mmx", ic, dst, src, imm): its memory operands'
        // addresses (1045-1046) (Sprint 177)
        Op::X87 { mem, .. } => match mem {
            Some(Loc::M { addr, .. }) => vec![*addr],
            _ => vec![],
        },
        Op::Mmx { dst, src, .. } => [dst.addr(), src.addr()].into_iter().flatten().collect(),
        // ("vex", ...) and ("vexs", ...): d's and s2's addresses; ("fma", ...): s3's (1017-1022; Sprint 182)
        Op::Vex { d, s2, .. } | Op::Vexs { d, s2, .. } => [d.addr(), s2.addr()].into_iter().flatten().collect(),
        Op::Fma { s3, .. } => s3.addr().into_iter().collect(),
        Op::Vzeroupper => vec![],
        // ("mxcsr", "st"|"ld", m32) and ("fcw", "st", m16): lowered in A64
        Op::Mxcsr {
            loc: Loc::M { addr, .. },
            ..
        }
        | Op::Fcw {
            loc: Loc::M { addr, .. },
        } => vec![*addr],
        Op::X86 { dst, srcs, .. } => {
            let mut v = srcs.clone();
            if let Some(Loc::M { addr, .. }) = dst {
                v.push(*addr);
            }
            v
        }
        Op::Imm { .. }
        | Op::Addr { .. }
        | Op::Ud
        | Op::Mxcsr { .. }
        | Op::Fcw { .. }
        | Op::Df { .. }
        | Op::Cpuid
        | Op::Rep { .. }
        | Op::Fence { .. }
        | Op::Pause
        | Op::Xgetbv => vec![],
    }
}

/// `_temps_written` (1058-1059).
fn temps_written(op: &Op) -> Vec<Temp> {
    match op {
        Op::Imm { t, .. } | Op::Addr { t, .. } | Op::Ld { t, .. } | Op::Alu { t, .. } | Op::Sel { t, .. } => vec![*t],
        _ => vec![],
    }
}

/// An address's Arm form (vm_backend 1192): LDR/STR's scaled offset, LDUR/STUR's -256..255, or a register offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddressForm {
    Imm { base: u32, disp: i64 },
    Unscaled { base: u32, disp: i64 },
    Reg { base: u32, index: u32, scaled: bool },
}

/// `_reads_low32` (1206-1216): `o` uses only the low 32 bits of temporary `t`.
fn reads_low32(o: &Op, t: Temp) -> bool {
    match o {
        Op::Alu { op, bits, .. } => *bits == 32 && matches!(*op, "add" | "sub" | "and" | "or" | "xor"),
        Op::Flags { kind, bits, .. } => *bits == 32 && matches!(*kind, "add" | "sub" | "inc" | "dec" | "logic"),
        Op::St { loc, t: v } if *v == t => match loc {
            Loc::M { addr, bytes } => *bytes == 4 && *addr != t,
            Loc::R { bits, hi, .. } => *bits == 32 && !hi,
        },
        _ => false,
    }
}

/// What an op writes as a location (`_operand_forms`' `writes`).
fn writes(o: &Op) -> Option<Loc> {
    match o {
        Op::St { loc, .. }
        | Op::Set { loc, .. }
        | Op::Cmov { loc, .. }
        | Op::Cst { loc, .. }
        | Op::Cstz { loc, .. } => Some(*loc),
        _ => None,
    }
}

/// `FORWARD_AFTER` (1202) over the ops carried.
fn forward_after(o: &Op) -> bool {
    matches!(
        o,
        Op::Alu { .. }
            | Op::Flags { .. }
            | Op::Imm { .. }
            | Op::Br { .. }
            | Op::St { .. }
            | Op::Sel { .. }
            | Op::Set { .. }
            | Op::Cmov { .. }
            | Op::Cst { .. }
            | Op::Cstz { .. }
    )
}

type Forms = (BTreeMap<Temp, u32>, BTreeMap<Temp, u64>, BTreeMap<Temp, u32>);

/// `_operand_forms` (1219-1268): (temporaries read as the guest register they load, constants kept out of registers,
/// results computed straight into the guest register they are stored to). The ops at `dead` emit nothing and read
/// nothing.
fn operand_forms(ops: &[Op], last: &BTreeMap<Temp, usize>, dead: &BTreeSet<usize>) -> Forms {
    let (mut alias, mut quiet, mut forward) = (BTreeMap::new(), BTreeMap::new(), BTreeMap::new());
    let mut readers: BTreeMap<Temp, Vec<usize>> = BTreeMap::new();
    for (i, op) in ops.iter().enumerate() {
        if dead.contains(&i) {
            continue;
        }
        for t in temps_read(op) {
            readers.entry(t).or_default().push(i);
        }
    }
    for (i, op) in ops.iter().enumerate() {
        match op {
            Op::Ld {
                t,
                loc: Loc::R { idx, bits, hi: false },
            } if matches!(*bits, 32 | 64) && last.contains_key(t) => {
                let rs = readers.get(t).cloned().unwrap_or_default();
                if *bits == 32 && !rs.iter().all(|&j| reads_low32(&ops[j], *t)) {
                    continue;
                }
                let written = ops[i + 1..last[t]]
                    .iter()
                    .any(|o| matches!(writes(o), Some(Loc::R { idx: w, .. }) if w == *idx));
                if !written {
                    alias.insert(*t, u32::from(*idx));
                }
            }
            Op::Imm { t, value } if readers.contains_key(t) => {
                let rs = &readers[t];
                let all_add_sub = rs.iter().all(|&j| {
                    matches!(&ops[j], Op::Alu { op, a, b, .. } if matches!(*op, "add" | "sub") && *b == *t && *a != *t)
                });
                // an ADD or SUB immediate (0-4095), or a constant only a branch reads (into x17 directly)
                let branch_only = rs.len() == 1 && matches!(&ops[rs[0]], Op::Br { t: bt, .. } if *bt == *t);
                if (*value < 4096 && all_add_sub) || branch_only {
                    quiet.insert(*t, *value);
                }
            }
            _ => {}
        }
    }
    for (i, op) in ops.iter().enumerate() {
        let (t, width) = match op {
            Op::Alu { t, bits, .. } => (*t, *bits),
            Op::Ld {
                t,
                loc: Loc::M { bytes, .. },
            } if !alias.contains_key(t) => (*t, 8 * bytes),
            _ => continue,
        };
        let stores: Vec<usize> = readers
            .get(&t)
            .map(|rs| {
                rs.iter()
                    .copied()
                    .filter(|&j| !matches!(ops[j], Op::Flags { .. }))
                    .collect()
            })
            .unwrap_or_default();
        if stores.len() != 1 || !matches!(&ops[stores[0]], Op::St { t: v, .. } if *v == t) {
            continue;
        }
        let Op::St {
            loc: Loc::R { idx, bits, hi: false },
            ..
        } = ops[stores[0]]
        else {
            continue;
        };
        if bits != width || !matches!(width, 32 | 64) {
            continue;
        }
        let idx = u32::from(idx);
        let ok = ops
            .iter()
            .enumerate()
            .filter(|(j, _)| *j > i && !dead.contains(j))
            .all(|(j, o)| {
                forward_after(o)
                    && match writes(o) {
                        None => true,
                        Some(Loc::R { idx: w, .. }) => u32::from(w) != idx || j == stores[0],
                        Some(Loc::M { .. }) => false,
                    }
                    && !temps_read(o).iter().any(|r| alias.get(r) == Some(&idx))
            });
        if ok {
            forward.insert(t, idx);
        }
    }
    (alias, quiet, forward)
}

/// `_folded_addresses` (1271-1302): the `addr` temporaries whose every use is a plain load or store, each with the Arm
/// form that reaches it. 64-bit addresses with no segment only.
fn folded_addresses(ops: &[Op]) -> BTreeMap<Temp, AddressForm> {
    let mut users: BTreeMap<Temp, Vec<u32>> = BTreeMap::new();
    let mut other: BTreeSet<Temp> = BTreeSet::new();
    for op in ops {
        match op {
            Op::Ld {
                loc: Loc::M { addr, bytes },
                ..
            } => users.entry(*addr).or_default().push(*bytes),
            Op::St {
                loc: Loc::M { addr, bytes },
                ..
            } => {
                users.entry(*addr).or_default().push(*bytes);
                other.extend(temps_read(op).into_iter().filter(|t| t != addr));
            }
            _ => other.extend(temps_read(op)),
        }
    }
    let mut folded = BTreeMap::new();
    for op in ops {
        let Op::Addr {
            t,
            base,
            index,
            scale,
            disp,
            abits,
            gs,
        } = op
        else {
            continue;
        };
        let (Some(base), false, 64, Some(sizes)) = (base, *gs, *abits, users.get(t)) else {
            continue;
        };
        if other.contains(t) {
            continue;
        }
        let sizes: BTreeSet<u32> = sizes.iter().copied().collect();
        let base = u32::from(*base);
        match index {
            None => {
                let scaled = sizes.iter().all(|&s| {
                    let s = i64::from(s);
                    disp.rem_euclid(s) == 0 && (0..4096).contains(&disp.div_euclid(s))
                });
                if scaled {
                    folded.insert(*t, AddressForm::Imm { base, disp: *disp });
                } else if (-256..=255).contains(disp) {
                    folded.insert(*t, AddressForm::Unscaled { base, disp: *disp });
                }
            }
            Some(index) => {
                if *disp == 0 && (*scale == 1 || sizes == BTreeSet::from([*scale])) {
                    folded.insert(
                        *t,
                        AddressForm::Reg {
                            base,
                            index: u32::from(*index),
                            scaled: *scale != 1,
                        },
                    );
                }
            }
        }
    }
    folded
}

/// `_mem` (1305-1311): the load or store of `size` bytes between rt and a folded address.
fn mem(load: bool, size: u32, rt: u32, form: AddressForm) -> u32 {
    match form {
        AddressForm::Imm { base, disp } => {
            if load {
                a64::ldr(size, rt, base, disp as u32)
            } else {
                a64::str(size, rt, base, disp as u32)
            }
        }
        AddressForm::Unscaled { base, disp } => {
            if load {
                a64::ldur(size, rt, base, disp)
            } else {
                a64::stur(size, rt, base, disp)
            }
        }
        AddressForm::Reg { base, index, scaled } => {
            if load {
                a64::ldr_reg(size, rt, base, index, scaled)
            } else {
                a64::str_reg(size, rt, base, index, scaled)
            }
        }
    }
}

/// `_fused_move` (1314-1346): a load whose temporary only the next op stores, as one instruction straight between the
/// guest's register and memory or register. False when the pair is none of the forms.
fn fused_move(
    a: &mut Assembler,
    ops: &[Op],
    i: usize,
    last: &BTreeMap<Temp, usize>,
    reg: &BTreeMap<Temp, u32>,
    folded: &BTreeMap<Temp, AddressForm>,
) -> bool {
    let Op::Ld { t, loc: src } = ops[i] else {
        return false;
    };
    let Some(Op::St { loc: dst, t: v }) = ops.get(i + 1) else {
        return false;
    };
    if *v != t || last.get(&t) != Some(&(i + 1)) {
        return false;
    }
    match (src, *dst) {
        (
            Loc::R {
                idx: s,
                bits: sbits,
                hi: shi,
            },
            Loc::R {
                idx: d,
                bits: dbits,
                hi: dhi,
            },
        ) => {
            if shi || dhi || sbits != dbits || !matches!(sbits, 32 | 64) {
                return false;
            }
            let (d, s) = (u32::from(d), u32::from(s));
            a.emit(&[if sbits == 64 { a64::mov(d, s) } else { a64::mov32(d, s) }]);
            true
        }
        (
            Loc::M { addr, bytes },
            Loc::R {
                idx: d,
                bits: dbits,
                hi: dhi,
            },
        ) => {
            if dhi || !matches!(dbits, 32 | 64) || bytes * 8 > dbits {
                return false;
            }
            let d = u32::from(d);
            a.emit(&[match folded.get(&addr) {
                Some(f) => mem(true, bytes, d, *f),
                None => a64::ldr(bytes, d, reg[&addr], 0),
            }]);
            true
        }
        (
            Loc::R {
                idx: s,
                bits: sbits,
                hi: shi,
            },
            Loc::M { addr, bytes },
        ) => {
            if shi || sbits != bytes * 8 {
                return false;
            }
            let s = u32::from(s);
            a.emit(&[match folded.get(&addr) {
                Some(f) => mem(false, bytes, s, *f),
                None => a64::str(bytes, s, reg[&addr], 0),
            }]);
            true
        }
        _ => false,
    }
}

/// The guest runtime's functions the lowering calls (compile_insn's `symbols`, vm_backend 1417-1421): `w2m_cpuid`, which
/// CPUID calls (0 without a runtime image, as `symbols["w2m_cpuid"] if symbols else 0`); `w2m_pcmpistri` and
/// `w2m_approximate`, which PCMPISTRI and RCPPS, RSQRTPS, RCPSS, RSQRTSS call when the image defines them (else the model
/// runs them, 1790, 1813).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeCalls {
    pub cpuid: u64,
    pub pcmpistri: Option<u64>,
    pub approximate: Option<u64>,
    /// `tools/guest_runtime/x87_fast.c`'s five functions, when the image defines `w2m_x87_fld64` (Sprint 177;
    /// vm_backend 1663: `_x87_fast` runs only then): fld64, st0_double, fstp_commit, st0_exact, sqrt_commit
    pub x87_fast: Option<[Option<u64>; 5]>,
}

impl RuntimeCalls {
    /// From a runtime image's symbol table, read through `symbol`.
    pub fn from_symbols(symbol: impl Fn(&str) -> Option<u64>) -> RuntimeCalls {
        RuntimeCalls {
            cpuid: symbol("w2m_cpuid").unwrap_or(0),
            pcmpistri: symbol("w2m_pcmpistri"),
            approximate: symbol("w2m_approximate"),
            x87_fast: symbol("w2m_x87_fld64").map(|_| {
                [
                    "w2m_x87_fld64",
                    "w2m_x87_st0_double",
                    "w2m_x87_fstp_commit",
                    "w2m_x87_st0_exact",
                    "w2m_x87_sqrt_commit",
                ]
                .map(&symbol)
            }),
        }
    }
}

/// `compile_insn` (1417-1885): append `ins` to `a`. A taken branch puts the x86 target in x17 and jumps to `exit_va`.
/// `nzcv_from`: the flags kind of the previous instruction's NZCV; `live_after`: the flags a later instruction reads;
/// `nzcv_out`: the next instruction's Jcc branches on this one's NZCV. Returns true when control never falls through.
/// `state_va` is the process area's state block (PROCESS_VA + STATE_OFF), for a GS-relative address and CPUID's saved
/// registers. `runtime` holds the compiling VM's runtime image's functions the lowering calls (the Python's `symbols`).
#[allow(clippy::too_many_arguments)]
pub fn compile_insn(
    a: &mut Assembler,
    ins: &Lifted,
    exit_va: u64,
    state_va: u64,
    runtime: &RuntimeCalls,
    nzcv_from: Option<&str>,
    live_after: u32,
    nzcv_out: bool,
) -> Result<bool, CompileError> {
    let ops = &ins.ops;
    // flag liveness (1425-1434): each flags op with the flags read after it
    let mut flags_live: BTreeMap<usize, u32> = BTreeMap::new();
    let mut live = live_after;
    if ops.iter().any(|op| matches!(op, Op::Flags { .. })) {
        let masks = op_flag_masks(ins, nzcv_from)?;
        for i in (0..ops.len()).rev() {
            if matches!(ops[i], Op::Flags { .. }) {
                flags_live.insert(i, live);
            }
            live = live & !masks[i].1 | masks[i].0;
        }
    }
    let last_flags = flags_live.keys().next_back().copied();
    let dead: BTreeSet<usize> = flags_live
        .iter()
        .filter(|(i, v)| **v == 0 && !(nzcv_out && Some(**i) == last_flags))
        .map(|(i, _)| *i)
        .collect();
    let mut last: BTreeMap<Temp, usize> = BTreeMap::new();
    for (i, op) in ops.iter().enumerate() {
        if !dead.contains(&i) {
            for t in temps_read(op) {
                last.insert(t, i);
            }
        }
    }
    let mut free: Vec<u32> = TEMP_REGS.to_vec();
    let mut reg: BTreeMap<Temp, u32> = BTreeMap::new();
    let mut ends = false;
    let mut nzcv_ready = false; // an ALU op fused with the flags op that follows
    let mut fused_store = false; // a load fused with the store that follows
    let mut constant: BTreeMap<Temp, u64> = BTreeMap::new(); // temporaries an `imm` of this instruction set
    // 1466-1468: a locked form folds no address and fuses no move: its loads and stores are the exclusive pair
    let locked = ins.iclass.ends_with("_LOCK");
    let folded = if locked { BTreeMap::new() } else { folded_addresses(ops) };
    let mut lock_retry: Vec<Target> = Vec::new();
    // 1469-1471: a locked form, or an instruction with an op outside OPERAND_SAFE (1198: model steps, REP, DIV, CPUID,
    // the SSE and VEX lowerings), keeps every temporary in a register
    let safe = !locked
        && ops.iter().all(|op| {
            matches!(
                op,
                Op::Imm { .. }
                    | Op::Addr { .. }
                    | Op::Ld { .. }
                    | Op::St { .. }
                    | Op::Alu { .. }
                    | Op::Flags { .. }
                    | Op::Br { .. }
                    | Op::Set { .. }
                    | Op::Cmov { .. }
                    | Op::Sel { .. }
                    | Op::Cst { .. }
                    | Op::Cstz { .. }
                    | Op::Fence { .. }
                    | Op::Pause
            )
        });
    let (alias, quiet, forward) = if safe {
        operand_forms(ops, &last, &dead)
    } else {
        Default::default()
    };
    let mut pinned = alias.clone(); // temporaries that are guest registers, never freed
    pinned.extend(forward.iter().map(|(k, v)| (*k, *v)));
    let at = |t: Temp, reg: &BTreeMap<Temp, u32>| -> Result<u32, CompileError> {
        reg.get(&t)
            .copied()
            .ok_or_else(|| CompileError::Refused(format!("temporary {t} has no register at 0x{:x}", ins.address)))
    };

    for (i, op) in ops.iter().enumerate() {
        for t in temps_written(op) {
            if let Some(r) = pinned.get(&t) {
                reg.insert(t, *r);
                continue;
            }
            if quiet.contains_key(&t) {
                continue;
            }
            if free.is_empty() {
                return Err(CompileError::Refused(format!(
                    "more than {} live temporaries at 0x{:x}",
                    TEMP_REGS.len(),
                    ins.address
                )));
            }
            reg.insert(t, free.remove(0));
        }
        if !matches!(op, Op::Imm { .. }) {
            for t in temps_written(op) {
                constant.remove(&t);
            }
        }
        match op {
            Op::Imm { t, value } => {
                if !quiet.contains_key(t) {
                    a.emit(&a64::mov_imm(at(*t, &reg)?, *value));
                }
                constant.insert(*t, *value);
            }
            Op::Addr { t, .. } if folded.contains_key(t) => {} // the loads and stores reach it in their own form
            Op::Addr {
                t,
                base,
                index,
                scale,
                disp,
                abits,
                gs,
            } => {
                let rd = at(*t, &reg)?;
                let log = scale.trailing_zeros();
                match base {
                    Some(base) if -4096 < *disp && *disp < 4096 => {
                        // base (+ index) and a small displacement as ADD/SUB immediates, not a MOV of it
                        let base = u32::from(*base);
                        match index {
                            Some(index) => a.emit(&[a64::add(rd, base, u32::from(*index), LSL, log)]),
                            None => a.emit(&[a64::mov(rd, base)]),
                        }
                        if *disp > 0 {
                            a.emit(&[a64::addi(rd, rd, *disp as u32)]);
                        } else if *disp < 0 {
                            a.emit(&[a64::subi(rd, rd, (-*disp) as u32)]);
                        }
                    }
                    _ => {
                        a.emit(&a64::mov_imm(rd, *disp as u64));
                        if let Some(base) = base {
                            a.emit(&[a64::add(rd, rd, u32::from(*base), LSL, 0)]);
                        }
                        if let Some(index) = index {
                            a.emit(&[a64::add(rd, rd, u32::from(*index), LSL, log)]);
                        }
                    }
                }
                mask(a, rd, *abits);
                if *gs {
                    // + the thread's GS base: state block gs_base (the TEB)
                    a.emit(&a64::mov_imm(16, state_va + STATE_GS_BASE));
                    a.emit(&[a64::ldr(8, 16, 16, 0), a64::add(rd, rd, 16, LSL, 0)]);
                }
            }
            Op::Ld { .. } if !locked && fused_move(a, ops, i, &last, &reg, &folded) => {
                fused_store = true; // the store that follows is done
            }
            Op::Ld { t, .. } if alias.contains_key(t) => {} // read as the guest register itself
            Op::Ld { t, loc } => {
                let rt = at(*t, &reg)?;
                match loc {
                    Loc::R { idx, bits, hi } => {
                        let idx = u32::from(*idx);
                        a.emit(&[if *hi {
                            a64::ubfx(rt, idx, 8, 8)
                        } else if *bits == 64 {
                            a64::mov(rt, idx)
                        } else if *bits == 32 {
                            a64::mov32(rt, idx)
                        } else {
                            a64::ubfx(rt, idx, 0, *bits)
                        }]);
                    }
                    Loc::M { addr, bytes } if locked => {
                        // LOCK (1531-1545): a load-exclusive / store-exclusive loop. The commit rule leaves no guest
                        // effect between the load and the store, so a failed store-exclusive repeats only temporaries.
                        // Exclusives need natural alignment; a misaligned operand runs in the model (hvc #11).
                        let (size, addr_reg) = (*bytes, at(*addr, &reg)?);
                        if size > 1 {
                            let aligned = a.label();
                            a.emit(&[a64::ubfx(16, addr_reg, 0, size.ilog2())]);
                            a.branch(|off| a64::cbz(16, off), aligned);
                            a.emit(&[a64::hvc(HVC_SLOW_STEP)]);
                            a.bind(aligned);
                        }
                        let retry = a.label();
                        a.bind(retry);
                        lock_retry.push(retry);
                        a.emit(&[a64::ldaxr(size, rt, addr_reg)]);
                    }
                    Loc::M { addr, bytes } => match folded.get(addr) {
                        Some(f) => a.emit(&[mem(true, *bytes, rt, *f)]),
                        None => a.emit(&[a64::ldr(*bytes, rt, at(*addr, &reg)?, 0)]),
                    },
                }
            }
            Op::St {
                loc: Loc::M { addr, bytes },
                t,
            } if locked => {
                // 1550-1552: the store-exclusive, retried from the load-exclusive while it fails
                let retry = *lock_retry.last().ok_or_else(|| {
                    CompileError::Refused(format!("a locked store with no load at 0x{:x}", ins.address))
                })?;
                a.emit(&[a64::stlxr(*bytes, 16, at(*t, &reg)?, at(*addr, &reg)?)]);
                a.branch(|off| a64::cbnz(16, off), retry);
            }
            Op::St { .. } if fused_store => fused_store = false,
            Op::St {
                loc: Loc::R { idx, .. },
                t,
            } if forward.get(t) == Some(&u32::from(*idx)) => {} // computed in place
            Op::St { loc, t } => {
                let rt = at(*t, &reg)?;
                store(a, loc, rt, &folded, &reg)?;
            }
            Op::Alu {
                t,
                op: o,
                a: x,
                b: y,
                bits,
            } => {
                let (rd, rx) = (at(*t, &reg)?, at(*x, &reg)?);
                let ry = reg.get(y).copied();
                // the flags op this ALU op feeds: the next op past any store of the result
                let feeds = (i + 1..ops.len()).find(|&j| !matches!(ops[j], Op::St { .. }));
                let nxt = feeds.filter(|j| !dead.contains(j)).map(|j| &ops[j]);
                if matches!(*o, "add" | "sub")
                    && matches!(*bits, 32 | 64)
                    && let Some(Op::Flags {
                        kind,
                        a: fa,
                        b: fb,
                        r: fr,
                        bits: fbits,
                    }) = nxt
                    && (if *o == "add" {
                        matches!(*kind, "add" | "inc")
                    } else {
                        matches!(*kind, "sub" | "dec")
                    })
                    && (*fa, *fb, *fr, *fbits) == (*x, *y, *t, *bits)
                {
                    // the result and NZCV from one ADDS or SUBS; the 32-bit form zero-extends as x86 does, so no mask
                    // follows, and the flags op that comes next skips its own
                    a.emit(&[adds_subs(*o == "add", rd, rx, ry.unwrap_or(u32::MAX), *bits)]);
                    nzcv_ready = true;
                    continue; // as the Python's `continue`: no temporary is released after this op
                }
                let mut bits = *bits;
                if let Some(v) = quiet.get(y) {
                    // ADD or SUB with its immediate
                    let v = *v as u32;
                    a.emit(&[match (*o == "add", bits == 32) {
                        (true, true) => a64::addi32(rd, rx, v),
                        (false, true) => a64::subi32(rd, rx, v),
                        (true, false) => a64::addi(rd, rx, v),
                        (false, false) => a64::subi(rd, rx, v),
                    }]);
                    if bits == 32 {
                        bits = 64; // the 32-bit form zero-extends: no mask
                    }
                } else if matches!(*o, "add" | "sub" | "and" | "or" | "xor") {
                    let ry =
                        ry.ok_or_else(|| CompileError::Refused(format!("alu {o}: no register for its operand")))?;
                    let w = match *o {
                        "add" => a64::add(rd, rx, ry, LSL, 0),
                        "sub" => a64::sub(rd, rx, ry),
                        "and" => a64::and(rd, rx, ry),
                        "or" => a64::orr(rd, rx, ry),
                        _ => a64::eor(rd, rx, ry, LSL, 0),
                    };
                    if bits == 32 {
                        a.emit(&[w & !(1 << 31)]); // the W form zero-extends as x86 does: no mask
                        bits = 64;
                    } else {
                        a.emit(&[w]);
                    }
                } else {
                    let ry =
                        || ry.ok_or_else(|| CompileError::Refused(format!("alu {o}: no register for its operand")));
                    let count_width = if bits == 64 { 6 } else { 5 };
                    match *o {
                        "shr" => {
                            a.emit(&[a64::ubfx(16, ry()?, 0, count_width)]);
                            if bits < 64 {
                                a.emit(&[if bits != 32 {
                                    a64::ubfx(17, rx, 0, bits)
                                } else {
                                    a64::mov32(17, rx)
                                }]);
                                a.emit(&[a64::lsrv(rd, 17, 16)]);
                            } else {
                                a.emit(&[a64::lsrv(rd, rx, 16)]);
                            }
                        }
                        "mul_lo" => a.emit(&[a64::mul(rd, rx, ry()?)]),
                        "mul_hi" => a.emit(&[a64::umulh(rd, rx, ry()?)]),
                        "sext32" => a.emit(&[a64::sxtw(rd, rx)]),
                        "sext8" | "sext16" => a.emit(&[a64::sbfx(rd, rx, 0, if *o == "sext8" { 8 } else { 16 })]),
                        "bswap" => a.emit(&[a64::rev(rd, rx, bits)]),
                        "popcnt" => {
                            // V31 is scratch (the SSE lowerings' too)
                            a.emit(&[
                                if bits < 64 {
                                    a64::ubfx(16, rx, 0, bits)
                                } else {
                                    a64::mov(16, rx)
                                },
                                a64::fmov_to_v(8, 31, 16),
                                a64::cnt_8b(31, 31),
                                a64::addv_8b(31, 31),
                                a64::fmov_from_v(4, rd, 31),
                            ]);
                        }
                        "adc" | "sbb" => {
                            // a + b + CF, a - b - CF; CF must be defined
                            exit_if_undefined(a, 'C');
                            if *o == "adc" {
                                a.emit(&[a64::add(rd, rx, ry()?, LSL, 0), a64::add(rd, rd, REG_CF, LSL, 0)]);
                            } else {
                                a.emit(&[a64::sub(rd, rx, ry()?), a64::sub(rd, rd, REG_CF)]);
                            }
                        }
                        "bsf" | "bsr" => {
                            // the result is stored only for a nonzero source
                            a.emit(&[if bits < 64 {
                                a64::ubfx(16, rx, 0, bits)
                            } else {
                                a64::mov(16, rx)
                            }]);
                            if *o == "bsf" {
                                a.emit(&[a64::rbit(16, 16), a64::clz(rd, 16)]);
                            } else {
                                a.emit(&[a64::clz(16, 16), a64::movz(17, 63, 0), a64::sub(rd, 17, 16)]);
                            }
                        }
                        "shl" | "sar" => {
                            a.emit(&[a64::ubfx(16, ry()?, 0, count_width)]);
                            if *o == "shl" {
                                a.emit(&[a64::lslv(rd, rx, 16)]);
                            } else {
                                a.emit(&[a64::sbfx(17, rx, 0, bits), a64::asrv(rd, 17, 16)]);
                            }
                        }
                        "rol" | "ror" => {
                            a.emit(&[a64::ubfx(16, ry()?, 0, count_width), a64::ubfx(16, 16, 0, bits.ilog2())]);
                            if bits == 32 || bits == 64 {
                                if *o == "rol" {
                                    a.emit(&[a64::sub(16, ZR, 16)]); // rotate left by k = right by -k
                                }
                                a.emit(&[a64::rorv(rd, rx, 16, bits)]);
                            } else {
                                // 8 and 16 bits: two shifts and an OR
                                a.emit(&[a64::ubfx(17, rx, 0, bits)]);
                                a.emit(&[if *o == "rol" {
                                    a64::lslv(rd, 17, 16)
                                } else {
                                    a64::lsrv(rd, 17, 16)
                                }]);
                                a.emit(&[a64::sub(16, ZR, 16), a64::addi(16, 16, bits)]);
                                a.emit(&[if *o == "rol" {
                                    a64::lsrv(17, 17, 16)
                                } else {
                                    a64::lslv(17, 17, 16)
                                }]);
                                a.emit(&[a64::orr(rd, rd, 17)]);
                            }
                        }
                        "smul_lo" | "smul_hi" => {
                            if bits == 64 {
                                a.emit(&[if *o == "smul_lo" {
                                    a64::mul(rd, rx, ry()?)
                                } else {
                                    a64::smulh(rd, rx, ry()?)
                                }]);
                            } else {
                                a.emit(&[
                                    a64::sbfx(16, rx, 0, bits),
                                    a64::sbfx(17, ry()?, 0, bits),
                                    a64::mul(rd, 16, 17),
                                ]);
                                if *o == "smul_hi" {
                                    a.emit(&[a64::ubfx(rd, rd, bits, bits)]);
                                }
                            }
                        }
                        _ => return Err(CompileError::Refused(format!("alu {o}"))),
                    }
                }
                mask(a, rd, bits);
            }
            Op::Flags {
                kind,
                a: x,
                b: y,
                r,
                bits,
            } => {
                let (live, nzcv) = (flags_live[&i], nzcv_out && Some(i) == last_flags);
                if live == 0 && !nzcv {
                    // no later instruction reads them (flag liveness)
                } else if matches!(*kind, "shr" | "shl" | "sar" | "rol" | "ror")
                    && let Some(count) = constant.get(y)
                {
                    let (rx, rr) = (at(*x, &reg)?, at(*r, &reg)?);
                    shift_flags_const(a, kind, rx, *count, rr, *bits);
                } else {
                    let (rx, ry, rr) = (at(*x, &reg)?, at(*y, &reg)?, at(*r, &reg)?);
                    set_flags(a, kind, rx, ry, rr, *bits, nzcv_ready, live, nzcv)?;
                }
                nzcv_ready = false;
            }
            Op::Br { cond, t } => {
                let target: Vec<u32> = match quiet.get(t) {
                    Some(v) => a64::mov_imm(17, *v),
                    None => vec![a64::mov(17, at(*t, &reg)?)],
                };
                if *cond == "always" {
                    a.emit(&target);
                    a.branch(a64::b, Target::Va(exit_va));
                    ends = true;
                } else {
                    let skip = a.label();
                    match nzcv_from.and_then(|k| nzcv_condition(k, cond)) {
                        Some(arm) => {
                            // the previous instruction's NZCV
                            let inv = inverse(arm)?;
                            a.branch(move |off| a64::bcond(inv, off), skip);
                        }
                        None => {
                            eval_condition(a, cond)?;
                            a.branch(|off| a64::cbz(16, off), skip);
                        }
                    }
                    a.emit(&target);
                    a.branch(a64::b, Target::Va(exit_va));
                    a.bind(skip);
                }
            }
            Op::Set { cond, loc } => {
                eval_condition(a, cond)?;
                store(a, loc, 16, &folded, &reg)?;
            }
            Op::Sel { t, c, x, y } => {
                // t = x if c is nonzero, else y
                let (rd, end) = (at(*t, &reg)?, a.label());
                a.emit(&[a64::mov(rd, at(*y, &reg)?)]);
                let rc = at(*c, &reg)?;
                a.branch(move |off| a64::cbz(rc, off), end);
                a.emit(&[a64::mov(rd, at(*x, &reg)?)]);
                a.bind(end);
            }
            Op::Cst { c, loc, t } | Op::Cstz { c, loc, t } => {
                // store when c is nonzero (cst) or zero (cstz)
                let end = a.label();
                let rc = at(*c, &reg)?;
                if matches!(op, Op::Cst { .. }) {
                    a.branch(move |off| a64::cbz(rc, off), end);
                } else {
                    a.branch(move |off| a64::cbnz(rc, off), end);
                }
                store(a, loc, at(*t, &reg)?, &folded, &reg)?;
                a.bind(end);
            }
            Op::Cmov { cond, loc, t } => {
                // SDM CMOVcc: a 32-bit destination is zero-extended whether or not the condition holds
                let (other, end) = (a.label(), a.label());
                eval_condition(a, cond)?;
                a.branch(|off| a64::cbz(16, off), other);
                store(a, loc, at(*t, &reg)?, &folded, &reg)?;
                a.branch(a64::b, end);
                a.bind(other);
                if let Loc::R { idx, bits: 32, .. } = loc {
                    a.emit(&[a64::mov32(u32::from(*idx), u32::from(*idx))]);
                }
                a.bind(end);
            }
            Op::Div { bits, t, signed } => {
                // DIV: the quotient overflows exactly when the dividend's high half >= the divisor, which includes a
                // zero divisor; those, a 64-bit dividend with RDX != 0 and the 8- and 16-bit forms run in the model
                // (hvc #11), which raises #DE as Windows does. Every check comes before any write. Flags: all undefined.
                let (rs, fast, done) = (at(*t, &reg)?, a.label(), a.label());
                const RAX: u32 = 0;
                const RDX: u32 = 2;
                if *signed {
                    if *bits == 64 {
                        // IDIV: fast only for a dividend that is RAX sign-extended and a divisor neither 0 nor -1
                        a.emit(&[a64::sbfx(16, RAX, 63, 1), a64::cmp(16, RDX)]);
                        a.branch(|off| a64::bcond("ne", off), done);
                        a.branch(move |off| a64::cbz(rs, off), done);
                        a.emit(&[a64::addi(16, rs, 1)]);
                        a.branch(|off| a64::cbz(16, off), done);
                        a.emit(&[a64::sdiv(16, RAX, rs), a64::msub(RDX, 16, rs, RAX), a64::mov(RAX, 16)]);
                        a.branch(a64::b, fast);
                    } else if *bits == 32 {
                        // EDX:EAX as a signed 64-bit dividend over the sign-extended divisor; a zero divisor or a
                        // quotient past 32 bits still runs in the model
                        a.emit(&[
                            a64::mov32(16, RDX),
                            a64::eor(16, ZR, 16, LSL, 32),
                            a64::bfi(16, RAX, 0, 32),
                            a64::sbfx(17, rs, 0, 32),
                        ]);
                        a.branch(|off| a64::cbz(17, off), done);
                        a.emit(&[
                            a64::sdiv(18, 16, 17),
                            a64::msub(16, 18, 17, 16),
                            a64::sbfx(17, 18, 0, 32),
                            a64::cmp(17, 18),
                        ]);
                        a.branch(|off| a64::bcond("ne", off), done);
                        a.emit(&[a64::mov32(RAX, 18), a64::mov32(RDX, 16)]);
                        a.branch(a64::b, fast);
                    }
                } else if *bits == 64 {
                    a.branch(|off| a64::cbnz(RDX, off), done);
                    a.branch(move |off| a64::cbz(rs, off), done);
                    a.emit(&[a64::udiv(16, RAX, rs), a64::msub(RDX, 16, rs, RAX), a64::mov(RAX, 16)]);
                    a.branch(a64::b, fast);
                } else if *bits == 32 {
                    a.emit(&[a64::mov32(16, RDX), a64::mov32(17, rs), a64::cmp(16, 17)]);
                    a.branch(|off| a64::bcond("hs", off), done);
                    a.emit(&[
                        a64::eor(16, ZR, 16, LSL, 32),
                        a64::bfi(16, RAX, 0, 32),
                        a64::udiv(RAX, 16, 17),
                        a64::msub(RDX, RAX, 17, 16),
                    ]);
                    a.branch(a64::b, fast);
                }
                a.bind(done);
                a.emit(&[a64::hvc(HVC_SLOW_STEP)]);
                a.bind(fast);
                mark_undefined(a, U_CF | U_ZF | U_SF | U_OF | U_PF | U_AF);
            }
            Op::Sse {
                ic,
                dst,
                src,
                imm: Some(imm),
                ..
            } if matches!(ic.as_str(), "PCMPISTRI" | "PCMPISTRI64") && runtime.pcmpistri.is_some() => {
                // 1790-1812: natively, as CPUID's call is made: the operands to the monitor data page at
                // NATIVE_OPERANDS, the GPRs to the state block, and string_compare.c answers ECX and CF ZF SF OF (AF and
                // PF 0) in x0
                let v = sse_src(a, src, 16, &reg, 31)?;
                let SseLoc::X(d) = dst else {
                    return Err(CompileError::Refused(format!("sse {ic}: an XMM destination expected")));
                };
                a.emit(&[
                    a64::str_v(16, u32::from(*d), 28, NATIVE_OPERANDS),
                    a64::str_v(16, v, 28, NATIVE_OPERANDS + 16),
                ]);
                save_guest_registers(a, state_va);
                a.emit(&[
                    a64::addi(0, 28, NATIVE_OPERANDS),
                    a64::addi(1, 28, NATIVE_OPERANDS + 16),
                ]);
                a.emit(&a64::mov_imm(2, imm & 0xFF));
                call_runtime(a, runtime.pcmpistri.unwrap());
                a.emit(&[a64::mov(17, 0)]);
                load_guest_registers(a, state_va);
                a.emit(&[
                    a64::ubfx(RCX, 17, 0, 5), // ECX, RCX's upper half cleared
                    a64::ubfx(REG_CF, 17, 8, 1),
                    a64::ubfx(REG_ZF, 17, 9, 1),
                    a64::ubfx(REG_SF, 17, 10, 1),
                    a64::ubfx(REG_OF, 17, 11, 1),
                    a64::movz(16, 1, 0), // PF and AF clear (PF_RESULT 1)
                    a64::stp(ZR, ZR, 28, i64::from(AF_OPERANDS)),
                    a64::str(8, 16, 28, PF_RESULT),
                ]);
                mark_defined(a, U_CF | U_ZF | U_SF | U_OF | U_PF | U_AF);
            }
            Op::Sse {
                ic, dst, src, align, ..
            } if matches!(ic.as_str(), "RCPPS" | "RSQRTPS" | "RCPSS" | "RSQRTSS") && runtime.approximate.is_some() => {
                // 1813-1833: the reference CPU's tables in the runtime's C (approximate.c), as PCMPISTRI's call: the
                // destination and the source to NATIVE_OPERANDS, the GPRs to the state block (the C uses no vector
                // register), and the destination back with its new lanes
                let n = if ic.ends_with("PS") { 4 } else { 1 };
                if let (Some(addr), true) = (src.addr(), *align != 0) {
                    check_alignment(a, reg_at(&reg, addr)?);
                }
                let v = sse_src(a, src, if n == 4 { 16 } else { 4 }, &reg, 31)?;
                let SseLoc::X(d) = dst else {
                    return Err(CompileError::Refused(format!("sse {ic}: an XMM destination expected")));
                };
                let d = u32::from(*d);
                a.emit(&[
                    a64::str_v(16, d, 28, NATIVE_OPERANDS),
                    a64::str_v(16, v, 28, NATIVE_OPERANDS + 16),
                ]);
                save_guest_registers(a, state_va);
                a.emit(&[
                    a64::addi(0, 28, NATIVE_OPERANDS),
                    a64::addi(1, 28, NATIVE_OPERANDS + 16),
                    a64::movz(2, u64::from(ic.contains("RSQRT")), 0),
                    a64::movz(3, n, 0),
                ]);
                call_runtime(a, runtime.approximate.unwrap());
                load_guest_registers(a, state_va);
                a.emit(&[a64::ldr_v(16, d, 28, NATIVE_OPERANDS)]);
            }
            Op::Sse {
                ic,
                dst,
                src,
                align,
                imm,
            } => lower_sse(a, ic, dst, src, *align, *imm, &reg, &folded)?,
            // run by the model (hvc #11, 1665 and 1785): the profile's #UD, XGETBV, and the model's whole steps
            Op::Ud | Op::Xgetbv | Op::X86 { .. } => a.emit(&[a64::hvc(HVC_SLOW_STEP)]),
            // ("x87", ...) (1663-1666; Sprint 177): `_x87_fast` (1349-1400) for FLD m64, FSTP m64 and FSQRT when the
            // runtime defines its functions: a call into x87_fast.c, which answers 0 where the result is not exact,
            // and the model step then; every other x87 form is the model step
            Op::X87 {
                ic,
                opcode,
                reg: modrm_reg,
                st,
                mem,
            } => {
                let form = match (ic.as_str(), *opcode, *modrm_reg, st, mem) {
                    ("FLD", 0xDD, 0, None, Some(Loc::M { addr, .. })) => Some(("fld", Some(*addr))),
                    ("FSTP", 0xDD, 3, None, Some(Loc::M { addr, .. })) => Some(("fstp", Some(*addr))),
                    ("FSQRT", 0xD9, _, _, None) => Some(("fsqrt", None)),
                    _ => None,
                };
                match (form, runtime.x87_fast) {
                    (Some((form, addr)), Some(calls)) => {
                        let symbol = |i: usize| {
                            calls[i].ok_or_else(|| {
                                CompileError::Refused(format!("x87_fast.c's function {i} not in the runtime's symbols"))
                            })
                        };
                        let n = NATIVE_OPERANDS;
                        // `call` (1367-1375): x0-x15 to the state block, x0 as `x0` sets it, the call on the
                        // runtime's stack, its answer to x17, x0-x15 back
                        let call = |a: &mut Assembler, va: u64, x0: &[u32]| {
                            save_guest_registers(a, state_va);
                            a.emit(x0);
                            call_runtime(a, va);
                            a.emit(&[a64::mov(17, 0)]);
                            load_guest_registers(a, state_va);
                        };
                        let (slow, done) = (a.label(), a.label());
                        match form {
                            "fld" => {
                                // the load first: its fault before any change
                                let r = at(addr.unwrap_or_default(), &reg)?;
                                a.emit(&[a64::ldr(8, 17, r, 0)]);
                                call(a, symbol(0)?, &[a64::mov(0, 17)]);
                                a.branch(|off| a64::cbz(17, off), slow);
                            }
                            "fstp" => {
                                // the address, kept past the call (x30 is a temp)
                                let r = at(addr.unwrap_or_default(), &reg)?;
                                a.emit(&[a64::str(8, r, 28, n + 24)]);
                                call(a, symbol(1)?, &[a64::addi(0, 28, n)]);
                                a.branch(|off| a64::cbz(17, off), slow);
                                // the store, which may fault, before the commit
                                a.emit(&[
                                    a64::ldr(8, 16, 28, n),
                                    a64::ldr(8, 17, 28, n + 24),
                                    a64::str(8, 16, 17, 0),
                                ]);
                                call(a, symbol(2)?, &[a64::ldr(8, 0, 28, n + 8)]);
                            }
                            _ => {
                                call(a, symbol(3)?, &[a64::addi(0, 28, n)]);
                                a.branch(|off| a64::cbz(17, off), slow);
                                // FPSR kept: MXCSR's flags live there; x - r*r, exact
                                a.emit(&[
                                    a64::mrs(16, a64::FPSR),
                                    a64::str(8, 16, 28, n + 16),
                                    a64::ldr_v(8, 30, 28, n),
                                    a64::fsqrt_scalar(8, 31, 30),
                                    a64::fp_fused("fmsub", 8, 30, 31, 31, 30),
                                    a64::str_v(8, 31, 28, n),
                                    a64::str_v(8, 30, 28, n + 8),
                                    a64::ldr(8, 16, 28, n + 16),
                                    a64::msr(a64::FPSR, 16),
                                ]);
                                call(a, symbol(4)?, &[a64::addi(0, 28, n)]);
                            }
                        }
                        a.branch(a64::b, done);
                        a.bind(slow);
                        a.emit(&[a64::hvc(HVC_SLOW_STEP)]);
                        a.bind(done);
                    }
                    _ => a.emit(&[a64::hvc(HVC_SLOW_STEP)]),
                }
            }
            // ("mmx", ...): `rules/_mmx.py`'s A64 lowering (1658-1660) is not ported; refused by name (Sprint 177)
            // VEX (_vex.py's `lower`, 136-207; Sprint 182)
            Op::Vzeroupper => {
                a.emit(&[a64::addi(16, 28, YMM_HI_OFF)]);
                for i in 0..16 {
                    a.emit(&[a64::stp(a64::ZR, a64::ZR, 16, 16 * i)]);
                }
            }
            // in the model for now
            Op::Vexs { .. } => a.emit(&[a64::hvc(HVC_SLOW_SSE)]),
            Op::Fma { ic, d, s2, s3, .. } => lower_fma(a, ic, d, s2, s3, &reg)?,
            Op::Vex {
                base,
                d,
                s1,
                s2,
                vl,
                align,
                imm,
            } => lower_vex(a, base, d, s1.as_ref(), s2, *vl, *align, *imm, &reg, &folded)?,
            Op::Mmx { ic, .. } => {
                return Err(CompileError::LaterGroup {
                    class: ic.clone(),
                    why: format!(
                        "mmx {ic} at 0x{:x}: the MMX lowering (rules/_mmx.py) is a later group",
                        ins.address
                    ),
                });
            }
            Op::Mxcsr { store: st, loc } => {
                let Loc::M { addr, .. } = loc else {
                    return Err(CompileError::Refused(format!(
                        "MXCSR to a register at 0x{:x}",
                        ins.address
                    )));
                };
                lower_mxcsr(a, *st, at(*addr, &reg)?);
            }
            Op::Fcw { loc } => {
                // FNSTCW (1660-1662): the control word to memory
                a.emit(&[a64::ldr(2, 16, 28, FCW_OFF)]);
                store(a, loc, 16, &folded, &reg)?;
            }
            Op::Fence { kind } => a.emit(&[match *kind {
                "st" => a64::dmb_ishst(),
                "ld" => a64::dmb_ishld(),
                _ => a64::dmb_ish(),
            }]),
            Op::Rep { kind, size } => {
                // REP STOS / MOVS (1669-1686): the guest's RCX, RSI, RDI live in x1, x6, x7 and change once per
                // element, so a fault on an element leaves them at that iteration and the reconstruction table gives
                // the instruction's address (SDM Vol 1 7.3.9: the instruction is interruptible).
                let (lp, end) = (a.label(), a.label());
                a.emit(&[a64::ldr(1, 16, 28, DF_BYTE), a64::movz(17, u64::from(*size), 0)]);
                a.branch(|off| a64::cbz(16, off), lp);
                a.emit(&[a64::sub(17, ZR, 17)]); // DF: step down
                a.bind(lp);
                a.branch(|off| a64::cbz(RCX, off), end);
                if *kind == "movs" {
                    a.emit(&[
                        a64::ldr(*size, 16, RSI, 0),
                        a64::str(*size, 16, RDI, 0),
                        a64::add(RSI, RSI, 17, LSL, 0),
                    ]);
                } else {
                    a.emit(&[a64::str(*size, RAX, RDI, 0)]);
                }
                a.emit(&[a64::add(RDI, RDI, 17, LSL, 0), a64::subi(RCX, RCX, 1)]);
                a.branch(a64::b, lp);
                a.bind(end);
            }
            Op::Cx16 { addr } => {
                // CMPXCHG16B (1732-1760): #GP unless 16-byte aligned, raised as SSE's alignment check raises it (a load
                // from 0 before anything is written); then an exclusive pair loop. Equal: RCX:RBX stored, ZF 1; else
                // the old value written back (x86 always writes), RDX:RAX loaded, ZF 0. The status register is
                // borrowed and kept in SPILL.
                const RBX: u32 = 3;
                let rn = at(*addr, &reg)?;
                let status = if rn != 30 { 30 } else { 29 };
                let (aligned, retry, differ, done) = (a.label(), a.label(), a.label(), a.label());
                a.emit(&[a64::ubfx(16, rn, 0, 4)]);
                a.branch(|off| a64::cbz(16, off), aligned);
                a.emit(&[a64::movz(16, 0, 0), a64::ldr(8, 16, 16, 0)]);
                a.bind(aligned);
                a.emit(&[a64::str(8, status, 28, SPILL)]);
                a.bind(retry);
                a.emit(&[a64::ldaxp(16, 17, rn), a64::cmp(16, RAX)]);
                a.branch(|off| a64::bcond("ne", off), differ);
                a.emit(&[a64::cmp(17, RDX)]);
                a.branch(|off| a64::bcond("ne", off), differ);
                a.emit(&[a64::stlxp(status, RBX, RCX, rn)]); // RBX low, RCX high
                a.branch(move |off| a64::cbnz(status, off), retry);
                a.emit(&[a64::movz(REG_ZF, 1, 0)]);
                a.branch(a64::b, done);
                a.bind(differ);
                a.emit(&[a64::stlxp(status, 16, 17, rn)]);
                a.branch(move |off| a64::cbnz(status, off), retry);
                a.emit(&[a64::mov(RAX, 16), a64::mov(RDX, 17), a64::movz(REG_ZF, 0, 0)]);
                a.bind(done);
                a.emit(&[a64::ldr(8, status, 28, SPILL)]);
                mark_defined(a, U_ZF);
            }
            Op::Cpuid => {
                // 1840-1848: the guest runtime answers from the profile, called on the runtime's stack with the guest's
                // x0-x15 saved in the state block and loaded back after
                a.emit(&a64::mov_imm(16, state_va));
                for r in (0..16).step_by(2) {
                    a.emit(&[a64::stp(r, r + 1, 16, 8 * i64::from(r))]);
                }
                a.emit(&a64::mov_imm(16, RUNTIME_STACK_TOP));
                a.emit(&[a64::addi(31, 16, 0)]);
                a.emit(&a64::mov_imm(16, runtime.cpuid));
                a.emit(&[a64::blr(16)]);
                a.emit(&a64::mov_imm(16, state_va));
                for r in (0..16).step_by(2) {
                    a.emit(&[a64::ldp(r, r + 1, 16, 8 * i64::from(r))]);
                }
            }
            Op::Df { set } => {
                // CLD, STD (1849-1850)
                a.emit(&[a64::movz(16, u64::from(*set), 0), a64::str(1, 16, 28, DF_BYTE)]);
            }
            Op::Pause => {
                // PAUSE counted (1851-1859): the spin census; and a yield when another thread is ready (SLICE_UNTIL 0;
                // Sprint 096, KVM's directed yield): the limit becomes the count (limit - x23) and x23 0, so the next
                // block's entry ends the slice with the count exact
                a.emit(&[
                    a64::ldr(8, 16, 28, PAUSE_COUNT),
                    a64::addi(16, 16, 1),
                    a64::str(8, 16, 28, PAUSE_COUNT),
                ]);
                let stay = a.label();
                a.emit(&[a64::ldr(8, 16, 28, SLICE_UNTIL)]);
                a.branch(|off| a64::cbnz(16, off), stay);
                a.emit(&[
                    a64::ldr(8, 16, 28, 8),
                    a64::sub(16, 16, REG_BUDGET),
                    a64::str(8, 16, 28, 8),
                    a64::movz(REG_BUDGET, 0, 0),
                ]);
                a.bind(stay);
            }
        }
        for t in temps_read(op).into_iter().chain(temps_written(op)) {
            if reg.contains_key(&t) && last.get(&t).is_none_or(|&l| l <= i) {
                let r = reg.remove(&t).unwrap();
                if !pinned.contains_key(&t) {
                    free.push(r);
                }
            }
        }
    }
    Ok(ends)
}

/// compile_insn's `store` (1446-1460).
fn store(
    a: &mut Assembler,
    loc: &Loc,
    rt: u32,
    folded: &BTreeMap<Temp, AddressForm>,
    reg: &BTreeMap<Temp, u32>,
) -> Result<(), CompileError> {
    match loc {
        Loc::R { idx, bits, hi } => {
            let idx = u32::from(*idx);
            a.emit(&[if *hi {
                a64::bfi(idx, rt, 8, 8)
            } else if *bits == 64 {
                a64::mov(idx, rt)
            } else if *bits == 32 {
                a64::mov32(idx, rt)
            } else {
                a64::bfi(idx, rt, 0, *bits)
            }]);
        }
        Loc::M { addr, bytes } => match folded.get(addr) {
            Some(f) => a.emit(&[mem(false, *bytes, rt, *f)]),
            None => {
                let ra = reg
                    .get(addr)
                    .ok_or_else(|| CompileError::Refused(format!("address temporary {addr} has no register")))?;
                a.emit(&[a64::str(*bytes, rt, *ra, 0)]);
            }
        },
    }
    Ok(())
}

/// `_lower_mxcsr` (1150-1182): STMXCSR and LDMXCSR in A64, as `_slow_sse` leaves the state; x16 and x17 only (TEMP_REGS
/// holds x18, and the operand's address may be in it). STMXCSR stores the saved MXCSR with FPSR's cumulative flags
/// folded in. LDMXCSR with a reserved bit (31:16) set leaves to the model, which raises #GP before any write; otherwise
/// it saves the value, clears FPSR and sets FPCR as `fpcr_for` does.
fn lower_mxcsr(a: &mut Assembler, store: bool, addr: u32) {
    if store {
        a.emit(&[
            a64::mrs(17, a64::FPSR),
            a64::lsri(16, 17, 7),   // x16 bit 0: IDC
            a64::bfi(17, 17, 1, 5), // bits 5:1: IOC DZC OFC UFC IXC
            a64::bfi(17, 16, 1, 1), // bit 1: IDC -> DE; bit 0 stays IOC -> IE
            a64::ubfx(17, 17, 0, 6),
            a64::ldr(4, 16, 28, MXCSR_OFF),
            a64::orr(16, 16, 17),
            a64::str(4, 16, addr, 0),
        ]);
        return;
    }
    let ok = a.label();
    a.emit(&[a64::ldr(4, 16, addr, 0), a64::lsri(17, 16, 16)]);
    a.branch(|off| a64::cbz(17, off), ok);
    a.emit(&[a64::hvc(HVC_SLOW_STEP)]);
    a.bind(ok);
    a.emit(&[
        a64::str(4, 16, 28, MXCSR_OFF),
        a64::msr(a64::FPSR, ZR),
        a64::ubfx(17, 16, 6, 1), // FIZ = DAZ
        a64::lsri(16, 16, 13),
        a64::bfi(17, 16, 23, 1), // RMode bit 23 = RC bit 0 (MXCSR 13)
        a64::lsri(16, 16, 1),
        a64::bfi(17, 16, 22, 1), // RMode bit 22 = RC bit 1 (MXCSR 14)
        a64::lsri(16, 16, 1),
        a64::bfi(17, 16, 24, 1), // FZ = FTZ (MXCSR 15)
        a64::movz(16, 1, 0),
        a64::bfi(17, 16, 1, 1), // AH
        a64::msr(a64::FPCR, 17),
    ]);
}

/// The guest's x0-x15 to the state block at `state_va` (x16 holds its address), before a call into the runtime's C.
fn save_guest_registers(a: &mut Assembler, state_va: u64) {
    a.emit(&a64::mov_imm(16, state_va));
    for r in (0..16).step_by(2) {
        a.emit(&[a64::stp(r, r + 1, 16, 8 * i64::from(r))]);
    }
}

/// The guest's x0-x15 back from the state block at `state_va`.
fn load_guest_registers(a: &mut Assembler, state_va: u64) {
    a.emit(&a64::mov_imm(16, state_va));
    for r in (0..16).step_by(2) {
        a.emit(&[a64::ldp(r, r + 1, 16, 8 * i64::from(r))]);
    }
}

/// A call into the runtime's C at `va` on the runtime's own stack (SP set from its top), as CPUID's is made.
fn call_runtime(a: &mut Assembler, va: u64) {
    a.emit(&a64::mov_imm(16, RUNTIME_STACK_TOP));
    a.emit(&[a64::addi(31, 16, 0)]);
    a.emit(&a64::mov_imm(16, va));
    a.emit(&[a64::blr(16)]);
}

/// `FP_CLASSES` (vm_backend 1114-1116): the SSE classes that can raise an SIMD exception, lowered only while MXCSR masks
/// all six.
fn fp_class(ic: &str) -> bool {
    let arithmetic = ["ADD", "SUB", "MUL", "DIV", "SQRT", "MIN", "MAX"].iter().any(|op| {
        ic.strip_prefix(op)
            .is_some_and(|t| matches!(t, "SS" | "SD" | "PS" | "PD"))
    });
    arithmetic
        || matches!(
            ic,
            "CVTTSS2SI"
                | "CVTTSD2SI"
                | "CVTSS2SI"
                | "CVTSD2SI"
                | "CVTSI2SS"
                | "CVTSI2SD"
                | "CVTSS2SD"
                | "CVTSD2SS"
                | "UCOMISS"
                | "UCOMISD"
                | "COMISS"
                | "COMISD"
                // `CMP_LOWERED` (_sse.py 113, 330; Sprint 180): its fast path only while MXCSR masks all six
                | "CMPPS"
                | "CMPPD"
        )
}

/// `_sse_src` (_sse.py 70-75): the V register holding a source operand, an XMM register or `scratch` loaded from
/// memory (`size` bytes).
fn sse_src(
    a: &mut Assembler,
    loc: &SseLoc,
    size: u32,
    reg: &BTreeMap<Temp, u32>,
    scratch: u32,
) -> Result<u32, CompileError> {
    match loc {
        SseLoc::X(n) => Ok(u32::from(*n)),
        SseLoc::Loc(Loc::M { addr, .. }) => {
            a.emit(&[a64::ldr_v(size, scratch, reg_at(reg, *addr)?, 0)]);
            Ok(scratch)
        }
        SseLoc::Loc(Loc::R { .. }) => Err(CompileError::Refused("an SSE source in a general register".into())),
    }
}

fn reg_at(reg: &BTreeMap<Temp, u32>, t: Temp) -> Result<u32, CompileError> {
    reg.get(&t)
        .copied()
        .ok_or_else(|| CompileError::Refused(format!("temporary {t} has no register")))
}

/// `_check_alignment` (_sse.py 78-86): #GP on a misaligned 16-byte operand, which Windows reports as an access
/// violation at the instruction; a load from address 0 (never mapped) raises it through the same exception path,
/// before the instruction changes anything.
fn check_alignment(a: &mut Assembler, rt: u32) {
    let ok = a.label();
    a.emit(&[a64::ubfx(16, rt, 0, 4)]);
    a.branch(|off| a64::cbz(16, off), ok);
    a.emit(&[a64::movz(16, 0, 0), a64::ldr(8, 16, 16, 0)]);
    a.bind(ok);
}

/// `_set_flags_from_fcmp` (_sse.py 89-97): (U)COMIS[SD]: FCMP gives NZCV 0011 unordered, 1000 less, 0110 equal, 0010
/// greater; x86 wants ZF PF CF 111, 001, 100, 000 and OF SF AF cleared. PF is unordered, kept lazily (PF_RESULT: 0
/// for PF set, 1 for clear, so V clear stores 1); x17 is scratch (x23 holds the step budget).
fn set_flags_from_fcmp(a: &mut Assembler) {
    a.emit(&[
        a64::cset(17, "vs"),
        a64::cset(REG_CF, "lt"),
        a64::cset(REG_ZF, "eq"),
        a64::orr(REG_ZF, REG_ZF, 17),
        a64::mov(REG_OF, ZR),
        a64::mov(REG_SF, ZR),
        a64::cset(16, "vc"),
        a64::stp(ZR, ZR, 28, i64::from(AF_OPERANDS)),
        a64::str(8, 16, 28, AF_OPERANDS + 16),
    ]);
    mark_defined(a, U_CF | U_ZF | U_SF | U_OF | U_PF | U_AF);
}

/// `_sse.lower` (_sse.py 324-488) for the SSE float core (group 4, Sprint 160): `_sse.rule`'s classes. XMM i is V i;
/// V30 and V31 are scratch; FPCR follows MXCSR. An instruction that can raise an SIMD exception leaves the fast path
/// when MXCSR unmasks any.
#[allow(clippy::too_many_arguments)]
fn lower_sse(
    a: &mut Assembler,
    ic: &str,
    dst: &SseLoc,
    src: &SseLoc,
    align: u32,
    imm: Option<u64>,
    reg: &BTreeMap<Temp, u32>,
    folded: &BTreeMap<Temp, AddressForm>,
) -> Result<(), CompileError> {
    for loc in [dst, src] {
        if let (Some(addr), true) = (loc.addr(), align != 0) {
            check_alignment(a, reg_at(reg, addr)?);
        }
    }
    if fp_class(ic) {
        // fast path only when MXCSR masks all six (330-335)
        let fast = a.label();
        a.emit(&[
            a64::ldr(4, 16, 28, MXCSR_OFF),
            a64::ubfx(16, 16, 7, 6),
            a64::cmpi(16, 0x3F),
        ]);
        a.branch(|off| a64::bcond("eq", off), fast);
        a.emit(&[a64::hvc(HVC_SLOW_SSE)]);
        a.bind(fast);
    }
    if ic == "PMOVSXDQ" {
        // 336-339: two dwords sign-extended (SSHLL #0; Sprint 180)
        let SseLoc::X(d) = dst else {
            return Err(CompileError::Refused(format!("sse {ic}: an XMM destination expected")));
        };
        let v = sse_src(a, src, if src.addr().is_some() { 8 } else { 16 }, reg, 31)?;
        a.emit(&[a64::sxtl_2d(u32::from(*d), v)]);
        return Ok(());
    }
    if crate::rules::SSE2_INT_BLOCK.contains(&ic) {
        // 340-342
        return lower_int(a, ic, dst, src, imm, reg);
    }
    let xmm = |l: &SseLoc| match l {
        SseLoc::X(n) => Ok(u32::from(*n)),
        _ => Err(CompileError::Refused(format!("sse {ic}: an XMM operand expected"))),
    };
    let addr_reg = |l: &SseLoc| match l.addr() {
        Some(t) => reg_at(reg, t),
        None => Err(CompileError::Refused(format!("sse {ic}: a memory operand expected"))),
    };
    match ic {
        "MOVAPS" | "MOVAPD" | "MOVUPS" | "MOVUPD" | "LDDQU" => match (dst, src) {
            // 368-374; LDDQU, a 16-byte load, memory only (Sprint 065 there, Sprint 235 here)
            (SseLoc::X(d), SseLoc::X(s)) => a.emit(&[a64::mov_v(u32::from(*d), u32::from(*s))]),
            (SseLoc::X(d), _) => a.emit(&[a64::ldr_v(16, u32::from(*d), addr_reg(src)?, 0)]),
            _ => a.emit(&[a64::str_v(16, xmm(src)?, addr_reg(dst)?, 0)]),
        },
        "MOVSS" | "MOVSD_XMM" => {
            // 375-382: the register form merges; a load zeroes the rest
            let n = if ic == "MOVSS" { 4 } else { 8 };
            match (dst, src) {
                (SseLoc::X(d), SseLoc::X(s)) => a.emit(&[a64::ins_lane0(n, u32::from(*d), u32::from(*s))]),
                (SseLoc::X(d), _) => a.emit(&[a64::ldr_v(n, u32::from(*d), addr_reg(src)?, 0)]),
                _ => a.emit(&[a64::str_v(n, xmm(src)?, addr_reg(dst)?, 0)]),
            }
        }
        "MOVD" | "MOVQ" => match (dst, src) {
            // 383-396
            (_, SseLoc::Loc(Loc::R { idx, bits, .. })) => {
                let n = bits / 8;
                let idx = u32::from(*idx);
                a.emit(&[
                    if n == 8 { a64::mov(16, idx) } else { a64::mov32(16, idx) },
                    a64::fmov_to_v(n, xmm(dst)?, 16),
                ]);
            }
            (SseLoc::Loc(r @ Loc::R { bits, .. }), _) => {
                a.emit(&[a64::fmov_from_v(bits / 8, 16, xmm(src)?)]);
                store(a, r, 16, folded, reg)?;
            }
            (SseLoc::X(d), SseLoc::Loc(Loc::M { .. })) => {
                let n = if ic == "MOVD" { 4 } else { 8 };
                a.emit(&[a64::ldr_v(n, u32::from(*d), addr_reg(src)?, 0)]);
            }
            (SseLoc::Loc(Loc::M { .. }), _) => {
                let n = if ic == "MOVD" { 4 } else { 8 };
                a.emit(&[a64::str_v(n, xmm(src)?, addr_reg(dst)?, 0)]);
            }
            _ => {
                // MOVQ xmm, xmm: the low 64 bits, the rest zeroed
                a.emit(&[a64::fmov_from_v(8, 16, xmm(src)?), a64::fmov_to_v(8, xmm(dst)?, 16)]);
            }
        },
        "PEXTRW" => {
            // 361-367 (Sprint 235): word imm & 7 of the XMM source, zero-extended into the general register; a memory
            // source is #UD in `register_only_rule`, before this
            let Some(imm) = imm else {
                return Err(CompileError::Refused("sse PEXTRW: no immediate".into()));
            };
            let SseLoc::Loc(r @ Loc::R { .. }) = dst else {
                return Err(CompileError::Refused(
                    "sse PEXTRW: a general register destination expected".into(),
                ));
            };
            a.emit(&[a64::umov_h(16, xmm(src)?, (imm & 7) as u32)]);
            store(a, r, 16, folded, reg)?;
        }
        "XORPS" | "XORPD" | "ANDPS" | "ANDPD" | "ORPD" => {
            // 397-399 (ORPD from Sprint 182: VORPD's base)
            let v = sse_src(a, src, 16, reg, 31)?;
            let d = xmm(dst)?;
            a.emit(&[if ic.starts_with("XOR") {
                a64::eor_v(d, d, v)
            } else if ic == "ORPD" {
                a64::orr_v(d, d, v)
            } else {
                a64::and_v(d, d, v)
            }]);
        }
        "CVTTSS2SI" | "CVTTSD2SI" | "CVTSS2SI" | "CVTSD2SI" => {
            // 440-467: x86 reports only IE and PE here, and only IE when the result is the integer indefinite; FRINTX,
            // FCVTZS and the range check can set IDC, and FRINTX sets IXC before an invalid conversion, so FPSR is
            // rebuilt from its value before the instruction (in x18: the only temporary of this instruction, a memory
            // source's address, is dead once the source is loaded)
            let n = if ic.contains("SS") { 4 } else { 8 };
            let SseLoc::Loc(r @ Loc::R { bits, .. }) = dst else {
                return Err(CompileError::Refused(format!(
                    "sse {ic}: a general register destination expected"
                )));
            };
            let bits = *bits;
            let mut v = sse_src(a, src, n, reg, 31)?;
            a.emit(&[a64::mrs(18, a64::FPSR)]);
            if !ic.starts_with("CVTT") {
                a.emit(&[a64::frintx(n, 30, v)]); // round in MXCSR's mode first
                v = 30;
            }
            a.emit(&[a64::fcvtzs(bits, n, 16, v)]);
            // the integer indefinite 1 << (bits-1) on NaN or input >= 2^(bits-1); FCVTZS saturates and gives 0 for NaN
            let bound: u64 = match (n, bits) {
                (4, 32) => 0x4F00_0000,
                (4, 64) => 0x5F00_0000,
                (8, 32) => 0x41E0_0000_0000_0000,
                _ => 0x43E0_0000_0000_0000,
            };
            let (ok, done) = (a.label(), a.label());
            let breg = if v != 31 { 31 } else { 30 };
            a.emit(&a64::mov_imm(17, bound));
            a.emit(&[a64::fmov_to_v(n, breg, 17), a64::fcmp(n, v, breg, false)]);
            a.branch(|off| a64::bcond("mi", off), ok); // CVT_INDEFINITE (vm_backend 1135) is True
            a.emit(&a64::mov_imm(16, 1 << (bits - 1)));
            a.emit(&[a64::movz(17, 1, 0), a64::orr(18, 18, 17), a64::msr(a64::FPSR, 18)]);
            a.branch(a64::b, done);
            a.bind(ok); // in range: IDC as it was before
            a.emit(&[
                a64::mrs(17, a64::FPSR),
                a64::ubfx(18, 18, 7, 1),
                a64::bfi(17, 18, 7, 1),
                a64::msr(a64::FPSR, 17),
            ]);
            a.bind(done);
            store(a, r, 16, folded, reg)?;
        }
        "CVTSI2SS" | "CVTSI2SD" => {
            // 468-477
            let n = if ic.ends_with("SS") { 4 } else { 8 };
            let bits = match src {
                SseLoc::Loc(Loc::R { idx, bits, .. }) => {
                    let idx = u32::from(*idx);
                    a.emit(&[if *bits == 64 {
                        a64::mov(16, idx)
                    } else {
                        a64::mov32(16, idx)
                    }]);
                    *bits
                }
                SseLoc::Loc(Loc::M { addr, bytes }) => {
                    a.emit(&[a64::ldr(*bytes, 16, reg_at(reg, *addr)?, 0)]);
                    8 * bytes
                }
                SseLoc::X(_) => return Err(CompileError::Refused(format!("sse {ic}: an XMM source"))),
            };
            if bits == 32 {
                a.emit(&[a64::sxtw(16, 16)]);
            }
            a.emit(&[a64::scvtf(n, 64, 30, 16), a64::ins_lane0(n, xmm(dst)?, 30)]);
        }
        "CVTSS2SD" | "CVTSD2SS" => {
            // 478-481
            let (sn, dn) = if ic == "CVTSS2SD" { (4, 8) } else { (8, 4) };
            let v = sse_src(a, src, sn, reg, 31)?;
            a.emit(&[a64::fcvt(dn, sn, 30, v), a64::ins_lane0(dn, xmm(dst)?, 30)]);
        }
        "UCOMISS" | "UCOMISD" | "COMISS" | "COMISD" => {
            // 482-486
            let n = if ic.ends_with("SS") { 4 } else { 8 };
            let v = sse_src(a, src, n, reg, 31)?;
            a.emit(&[a64::fcmp(n, xmm(dst)?, v, ic.starts_with("COMI"))]);
            set_flags_from_fcmp(a);
        }
        "CMPPS" | "CMPPD" => {
            // `CMP_LOWERED` (343-360; Sprint 180): imm8[2:0]; FCMEQ is quiet and FCMGE / FCMGT signal on any NaN, as
            // the x86 predicates 0 3 4 7 and 1 2 5 6 do
            let imm = imm.ok_or_else(|| CompileError::Refused(format!("sse {ic} without its immediate")))?;
            let (d, dbl, p) = (xmm(dst)?, ic == "CMPPD", imm & 7);
            let s = sse_src(a, src, 16, reg, 31)?;
            match p {
                0 | 4 => a.emit(&[a64::fcm("eq", dbl, d, d, s)]),
                1 | 5 => a.emit(&[a64::fcm("gt", dbl, d, s, d)]), // a < b: b > a
                2 | 6 => a.emit(&[a64::fcm("ge", dbl, d, s, d)]),
                // ordered: neither lane a NaN
                _ => a.emit(&[
                    a64::fcm("eq", dbl, 30, d, d),
                    a64::fcm("eq", dbl, 31, s, s),
                    a64::and_v(d, 30, 31),
                ]),
            }
            if (3..=6).contains(&p) {
                a.emit(&[a64::not_16b(d, d)]);
            }
        }
        "CVTDQ2PS" => {
            // 400-406: SCVTF in FPCR's mode, MXCSR's RC; an unmasked exception to the model
            unmasked_to_model(a);
            let v = sse_src(a, src, 16, reg, 31)?;
            a.emit(&[a64::scvtf_4s(xmm(dst)?, v)]);
        }
        "CVTPS2PD" => {
            // 407-413: FCVTL widens the low two singles exactly; the memory form reads 8 bytes
            unmasked_to_model(a);
            let v = sse_src(a, src, if src.addr().is_some() { 8 } else { 16 }, reg, 31)?;
            a.emit(&[a64::fcvtl_2d(xmm(dst)?, v)]);
        }
        "CVTPD2PS" => {
            // 414-421: FCVTN narrows each double in FPCR's mode and clears the high half
            unmasked_to_model(a);
            let v = sse_src(a, src, 16, reg, 31)?;
            a.emit(&[a64::fcvtn_2s(xmm(dst)?, v)]);
        }
        "CVTPD2DQ" | "CVTPS2DQ" | "CVTTPD2DQ" | "CVTTPS2DQ" => {
            // 422-424
            unmasked_to_model(a);
            let v = sse_src(a, src, 16, reg, 31)?;
            to_dwords(a, xmm(dst)?, v, ic.ends_with("PD2DQ"), ic.starts_with("CVTT"));
        }
        // 425-426: in the model (x86fp), whole
        _ if matches!(ic, "CMPSD_XMM" | "HADDPD" | "MPSADBW") || SSE_050.contains(&ic) => {
            a.emit(&[a64::hvc(HVC_SLOW_SSE)]);
        }
        _ => {
            // 427-439: ADD SUB MUL DIV MIN MAX SQRT in SS SD PS PD
            let (kind, form) = ic.split_at(ic.len() - 2);
            if !["ADD", "SUB", "MUL", "DIV", "MIN", "MAX", "SQRT"].contains(&kind) {
                return Err(CompileError::Refused(format!("sse {ic}")));
            }
            let kind = kind.to_lowercase();
            let n = if form == "SS" || form == "PS" { 4 } else { 8 };
            let d = xmm(dst)?;
            if form == "SS" || form == "SD" {
                let v = sse_src(a, src, n, reg, 31)?;
                a.emit(&[if kind == "sqrt" {
                    a64::fsqrt_scalar(n, 30, v)
                } else {
                    a64::fp_scalar(&kind, n, 30, d, v)
                }]);
                a.emit(&[a64::ins_lane0(n, d, 30)]);
            } else {
                let v = sse_src(a, src, 16, reg, 31)?;
                a.emit(&[if kind == "sqrt" {
                    a64::fsqrt_vector(n, d, v)
                } else {
                    a64::fp_vector(&kind, n, d, d, v)
                }]);
            }
        }
    }
    Ok(())
}

/// `SSE_050` (lift.py 130-133): the set-050 classes; those `lower` does not lower before 425 go to the model.
const SSE_050: [&str; 39] = [
    "CVTDQ2PS",
    "CVTPS2DQ",
    "CVTTPS2DQ",
    "CVTPS2PD",
    "CVTPD2PS",
    "CMPPS",
    "CMPPD",
    "CMPSS",
    "SHUFPD",
    "MOVLPS",
    "MOVHPD",
    "PSHUFB",
    "PALIGNR",
    "PSADBW",
    "PACKSSWB",
    "PACKSSDW",
    "PMADDUBSW",
    "PMADDWD",
    "PEXTRW",
    "PINSRB",
    "PINSRD",
    "PUNPCKHQDQ",
    "PMULHW",
    "PMULLD",
    "PADDUSB",
    "PADDB",
    "PAVGB",
    "PABSW",
    "PCMPGTB",
    "PCMPGTW",
    "PCMPEQQ",
    "PMAXUD",
    "PMAXUW",
    "PMAXSW",
    "PMINSD",
    "PMOVSXDQ",
    "PMOVZXWD",
    "PSLLW",
    "MOVNTDQ",
];

/// MONITOR_DATA + this: YMM0-15 bits 255:128, 16 bytes each (vm_backend 106).
const YMM_HI_OFF: u32 = 0x400;

/// `SLOW` (_vex.py 57-59): VEX bases the model runs (x86fp) for now.
const VEX_SLOW: [&str; 19] = [
    "CMPSD_XMM_VEX",
    "HADDPD",
    "PSHUFB",
    "PALIGNR",
    "PINSRD",
    "PINSRQ",
    "PEXTRQ",
    "CMPPS_VEX",
    "ROUNDSD",
    "PBLENDD",
    "PERMILPS_VAR",
    "PADDB",
    "PMULUDQ",
    "PUNPCKHQDQ",
    "CVTSS2SI",
    "CVTSI2SS",
    "CVTTSD2SI",
    "RCPPS",
    "RSQRTPS",
];

/// `NATIVE_256` (_vex.py 60-63): the bases lowered natively at VEX.256 too.
const VEX_NATIVE_256: [&str; 35] = [
    "PXOR",
    "XORPS",
    "XORPD",
    "PAND",
    "POR",
    "ORPD",
    "ANDPD",
    "ANDPS",
    "PCMPEQB",
    "PCMPEQW",
    "PSUBQ",
    "PSUBD",
    "PADDQ",
    "MOVDQU",
    "MOVDQA",
    "MOVAPD",
    "PMOVMSKB",
    "MOVUPS",
    "MOVAPS",
    "PADDD",
    "PSRLD",
    "PSLLD",
    "PSRAD",
    "PSLLDQ",
    "PANDN",
    "PUNPCKLDQ",
    "PSHUFD",
    "UNPCKHPS",
    "UNPCKLPS",
    "PCMPEQD",
    "PCMPGTD",
    "ANDNPS",
    "ORPS",
    "SHUFPS",
    "UNPCKLPD",
];

/// `SHIFT_BY_COUNT` (runtime.py 34): at VEX.256 one count shifts both halves.
const SHIFT_BY_COUNT: [&str; 7] = ["PSRLQ", "PSLLQ", "PSRLD", "PSLLD", "PSRAD", "PSRLW", "PSRAW"];

/// `_clear_upper` (_vex.py 110-111): a register's bits 255:128 to zero.
fn clear_upper(a: &mut Assembler, reg: u32) {
    a.emit(&[
        a64::addi(16, 28, YMM_HI_OFF),
        a64::stp(a64::ZR, a64::ZR, 16, 16 * i64::from(reg)),
    ]);
}

/// `_vex.py` 146-152: a VEX op goes to the model (`hvc #10`) when its base is `SLOW`, or at 256 bits when its base has no
/// native 256-bit lowering (Sprint 192: named, so a test can hold which stores reach the model's 256-bit branch).
pub fn vex_to_model(base: &str, vl: bool) -> bool {
    VEX_SLOW.contains(&base) || (vl && !VEX_NATIVE_256.contains(&base))
}

/// `lower` (_vex.py 149-207) for a ("vex", base, d, s1, s2, L, align, imm): the base's lowering on the low half, and at
/// VEX.256 on the upper halves staged from the monitor data page.
#[allow(clippy::too_many_arguments)]
fn lower_vex(
    a: &mut Assembler,
    base: &str,
    d: &SseLoc,
    s1: Option<&SseLoc>,
    s2: &SseLoc,
    vl: u8,
    align: u32,
    imm: Option<u64>,
    reg: &BTreeMap<Temp, u32>,
    folded: &BTreeMap<Temp, AddressForm>,
) -> Result<(), CompileError> {
    let vl = vl != 0;
    if vex_to_model(base, vl) {
        a.emit(&[a64::hvc(HVC_SLOW_SSE)]);
        return Ok(());
    }
    if fp_class(base) {
        // `_fp_guard` (118-125): the model runs it while MXCSR unmasks any exception
        unmasked_to_model(a);
    }
    for loc in [d, s2] {
        if let (Some(t), true) = (loc.addr(), align != 0) {
            let ok = a.label();
            a.emit(&[a64::ubfx(16, reg_at(reg, t)?, 0, if align == 32 { 5 } else { 4 })]);
            a.branch(|off| a64::cbz(16, off), ok);
            // #GP: an access violation at the instruction
            a.emit(&[a64::movz(16, 0, 0), a64::ldr(8, 16, 16, 0)]);
            a.bind(ok);
        }
    }
    let x = |l: &SseLoc| match l {
        SseLoc::X(n) => Some(u32::from(*n)),
        _ => None,
    };
    let addr = |l: &SseLoc| -> Result<u32, CompileError> {
        match l.addr() {
            Some(t) => reg_at(reg, t),
            None => Err(CompileError::Refused(format!("vex {base}: a memory operand expected"))),
        }
    };
    let base_lower = |a: &mut Assembler, dd: &SseLoc, ss: &SseLoc| lower_sse(a, base, dd, ss, 0, imm, reg, folded);
    if vl && base == "PMOVMSKB" {
        // 32 sign bits: the high half first, then the low
        let SseLoc::Loc(Loc::R { idx, .. }) = d else {
            return Err(CompileError::Refused(
                "vex PMOVMSKB: a general register destination expected".into(),
            ));
        };
        let rd = u32::from(*idx);
        let s = x(s2).ok_or_else(|| CompileError::Refused("vex PMOVMSKB: an XMM source expected".into()))?;
        a.emit(&[a64::addi(16, 28, YMM_HI_OFF), a64::ldr_v(16, 29, 16, 16 * s)]);
        base_lower(a, d, &SseLoc::X(29))?;
        a.emit(&[a64::eor(rd, a64::ZR, rd, a64::LSL, 16), a64::str(8, rd, 28, SPILL)]);
        base_lower(a, d, s2)?;
        a.emit(&[a64::ldr(8, 16, 28, SPILL), a64::orr(rd, rd, 16)]);
        return Ok(());
    }
    // a shift by count shifts both halves by s2's low quadword (SDM PSLLD, VEX.256 form)
    let count = vl && SHIFT_BY_COUNT.contains(&base) && imm.is_none();
    let mut low_src = *s2;
    let mut hi_src = match (count, x(s2)) {
        (true, Some(n)) => n,
        _ => 29,
    };
    if let (Some(_), Some(SseLoc::Loc(Loc::M { bytes, .. })), Some(_)) = (s1, Some(s2), x(d)) {
        // memory first: a fault leaves every register as it was (set 049, Xeon 8573C)
        let n = (*bytes).min(16);
        if vl && !count {
            a.emit(&[a64::ldr_v(16, 27, addr(s2)?, 16)]);
        }
        a.emit(&[a64::ldr_v(n, 29, addr(s2)?, 0)]);
        low_src = SseLoc::X(29);
        hi_src = if count { 29 } else { 27 };
    }
    if let (Some(s1n), Some(dn)) = (s1.and_then(x), x(d))
        && s1n != dn
    {
        if x(s2) == Some(dn) {
            // s2 is d: keep it before d takes s1
            a.emit(&[a64::mov_v(29, dn)]);
            low_src = SseLoc::X(29);
            hi_src = if count { 29 } else { 27 };
        }
        a.emit(&[a64::mov_v(dn, s1n)]);
    }
    if vl && x(d).is_some() {
        // stage both upper halves before d changes
        a.emit(&[a64::addi(16, 28, YMM_HI_OFF)]);
        if let Some(s1n) = s1.and_then(x) {
            a.emit(&[a64::ldr_v(16, 28, 16, 16 * s1n)]);
        }
        if let (Some(s2n), false) = (x(s2), count) {
            // (a shift's count stays in s2's low half)
            a.emit(&[a64::ldr_v(16, hi_src, 16, 16 * s2n)]);
        } else if s2.addr().is_some() && s1.is_none() {
            a.emit(&[a64::ldr_v(16, hi_src, addr(s2)?, 16)]);
        }
    }
    if d.addr().is_some() && vl {
        // a 256-bit store: the upper half first, so a fault on the next page leaves memory as it was
        let s = x(s2).ok_or_else(|| CompileError::Refused(format!("vex {base}: an XMM source expected")))?;
        a.emit(&[
            a64::addi(16, 28, YMM_HI_OFF),
            a64::ldr_v(16, 28, 16, 16 * s),
            a64::str_v(16, 28, addr(d)?, 16),
        ]);
    }
    base_lower(a, d, &low_src)?;
    if let Some(dn) = x(d)
        && !matches!(base, "COMISD" | "UCOMISD" | "COMISS" | "UCOMISS")
    {
        if !vl {
            clear_upper(a, dn);
        } else {
            if s1.is_none() && imm.is_none() {
                // a 256-bit load or move: the source's upper half
                a.emit(&[a64::mov_v(28, hi_src)]);
            } else {
                // (PSHUFD's imm8 permutes each half alike)
                base_lower(a, &SseLoc::X(28), &SseLoc::X(hi_src as u8))?;
            }
            a.emit(&[a64::addi(16, 28, YMM_HI_OFF), a64::str_v(16, 28, 16, 16 * dn)]);
        }
    }
    Ok(())
}

/// `_lower_fma` (_vex.py 210-228): scalar FMA natively while MXCSR masks every exception; packed forms in the model.
fn lower_fma(
    a: &mut Assembler,
    ic: &str,
    d: &SseLoc,
    s2: &SseLoc,
    s3: &SseLoc,
    reg: &BTreeMap<Temp, u32>,
) -> Result<(), CompileError> {
    let (kind, order, form) = (
        &ic[1..ic.len() - 5],
        &ic[ic.len() - 5..ic.len() - 2],
        &ic[ic.len() - 2..],
    );
    if form.starts_with('P') {
        a.emit(&[a64::hvc(HVC_SLOW_SSE)]);
        return Ok(());
    }
    let n = if form == "SS" { 4 } else { 8 };
    unmasked_to_model(a);
    let xreg = |l: &SseLoc| match l {
        SseLoc::X(r) => Ok(u32::from(*r)),
        _ => Err(CompileError::Refused(format!("fma {ic}: an XMM operand expected"))),
    };
    let (o1, o2) = (xreg(d)?, xreg(s2)?);
    let o3 = match s3 {
        SseLoc::X(r) => u32::from(*r),
        _ => {
            let t = s3
                .addr()
                .ok_or_else(|| CompileError::Refused(format!("fma {ic}: a memory operand expected")))?;
            a.emit(&[a64::ldr_v(n, 31, reg_at(reg, t)?, 0)]);
            31
        }
    };
    let regs = [o1, o2, o3];
    let ((m0, m1), add) = match order {
        "132" => ((0, 2), 1),
        "213" => ((1, 0), 2),
        _ => ((1, 2), 0),
    };
    // FUSED (_vex.py 64)
    let fused = match kind {
        "FMADD" => "fmadd",
        "FMSUB" => "fnmsub",
        "FNMADD" => "fmsub",
        _ => "fnmadd",
    };
    a.emit(&[
        a64::fp_fused(fused, n, 30, regs[m0], regs[m1], regs[add]),
        a64::ins_lane0(n, o1, 30),
    ]);
    clear_upper(a, o1);
    Ok(())
}

/// `_unmasked_to_model` (_sse.py 23-29): the model runs the instruction when MXCSR unmasks any exception, since only
/// the model raises the SIMD fault.
fn unmasked_to_model(a: &mut Assembler) {
    let fast = a.label();
    a.emit(&[
        a64::ldr(4, 16, 28, MXCSR_OFF),
        a64::ubfx(16, 16, 7, 6),
        a64::cmpi(16, 0x3F),
    ]);
    a.branch(|off| a64::bcond("eq", off), fast);
    a.emit(&[a64::hvc(HVC_SLOW_SSE)]);
    a.bind(fast);
}

/// `_to_dwords` (_sse.py 491-528): CVTPD2DQ, CVTPS2DQ, CVTTPD2DQ, CVTTPS2DQ, each lane as CVT(T)S*2SI converts it into
/// dwords, a double form's upper half zeroed; NaN or a value outside int32 gives 0x80000000 and IE with the lane's other
/// flags dropped; in range, the lane keeps PE and FPSR's IDC as it was. x18 holds FPSR before the lane; the source goes
/// to V30, V29 holds a bound, the results gather in V28.
fn to_dwords(a: &mut Assembler, d: u32, v: u32, double: bool, truncate: bool) {
    if v != 30 {
        a.emit(&[a64::mov_v(30, v)]);
    }
    let (n, lanes) = if double { (8, 2) } else { (4, 4) };
    let upper: u64 = if double { 0x41E0_0000_0000_0000 } else { 0x4F00_0000 };
    let (lower, holds) = if double {
        if truncate {
            (0xC1E0_0000_0020_0000u64, "gt")
        } else {
            (0xC1E0_0000_0000_0000, "ge")
        }
    } else {
        (0xCF00_0000, "ge")
    };
    if double {
        a.emit(&[a64::movi_zero(28)]);
    }
    for lane in 0..lanes {
        let (ok, low, done, indefinite) = (a.label(), a.label(), a.label(), a.label());
        a.emit(&[
            a64::mrs(18, a64::FPSR),
            if double {
                a64::umov_d(17, 30, lane)
            } else {
                a64::umov_s(17, 30, lane)
            },
            a64::fmov_to_v(n, 31, 17),
        ]);
        if !truncate {
            a.emit(&[a64::frintx(n, 31, 31)]);
        }
        a.emit(&[a64::fcvtzs(32, n, 16, 31)]);
        a.emit(&a64::mov_imm(17, upper));
        a.emit(&[a64::fmov_to_v(n, 29, 17), a64::fcmp(n, 31, 29, false)]);
        // NaN is unordered, so not below the upper bound; below the lower one FCVTZS gives 0x80000000 too, but the
        // flags differ (set 050 case 227: the Xeon reports IE alone)
        a.branch(|off| a64::bcond("mi", off), low); // CVT_INDEFINITE (vm_backend 1135) is True
        a.branch(a64::b, indefinite);
        a.bind(low);
        a.emit(&a64::mov_imm(17, lower));
        a.emit(&[a64::fmov_to_v(n, 29, 17), a64::fcmp(n, 31, 29, false)]);
        a.branch(move |off| a64::bcond(holds, off), ok); // CVT_INDEFINITE is True
        a.bind(indefinite);
        a.emit(&a64::mov_imm(16, 0x8000_0000));
        a.emit(&[a64::movz(17, 1, 0), a64::orr(18, 18, 17), a64::msr(a64::FPSR, 18)]);
        a.branch(a64::b, done);
        a.bind(ok);
        // in range: IDC as it was before the lane
        a.emit(&[
            a64::mrs(17, a64::FPSR),
            a64::ubfx(18, 18, 7, 1),
            a64::bfi(17, 18, 7, 1),
            a64::msr(a64::FPSR, 17),
        ]);
        a.bind(done);
        a.emit(&[a64::ins_s(28, lane, 16)]);
    }
    a.emit(&[a64::mov_v(d, 28)]);
}

/// `LANE_LOWERED` (_sse.py 103-111): one NEON three-same instruction each, d = op(d, source): (name, lane size).
fn lane_lowered(ic: &str) -> Option<(&'static str, u32)> {
    Some(match ic {
        "PUNPCKLQDQ" | "UNPCKLPD" => ("zip1", 3),
        "UNPCKHPD" | "PUNPCKHQDQ" => ("zip2", 3),
        "PADDQ" => ("add", 3),
        "PADDW" => ("add", 1),
        "PSUBW" => ("sub", 1),
        "PCMPEQD" => ("cmeq", 2),
        "PCMPGTD" => ("cmgt", 2),
        "PSUBUSB" => ("uqsub", 0),
        "PSUBSB" => ("sqsub", 0),
        "PADDSB" => ("sqadd", 0),
        "PADDSW" => ("sqadd", 1),
        "PMAXUB" => ("umax", 0),
        "PMULLW" => ("mul", 1),
        "PUNPCKHBW" => ("zip2", 0),
        "PUNPCKHDQ" => ("zip2", 2),
        "PUNPCKHWD" => ("zip2", 1),
        "PUNPCKLWD" => ("zip1", 1),
        "PCMPEQW" => ("cmeq", 1),
        "PADDUSB" => ("uqadd", 0),
        "PCMPGTB" => ("cmgt", 0),
        "PADDB" => ("add", 0),
        "PCMPEQQ" => ("cmeq", 3),
        "PMULLD" => ("mul", 2),
        _ => return None,
    })
}

/// `SHIFTS` (_sse.py 114-115): (lane size, kind) of the shifts by imm8 or by the source's low 64 bits.
fn shifts(ic: &str) -> Option<(u32, &'static str)> {
    Some(match ic {
        "PSRLD" => (2, "right"),
        "PSLLD" => (2, "left"),
        "PSRAD" => (2, "arith"),
        "PSRLW" => (1, "right"),
        "PSRAW" => (1, "arith"),
        "PSLLQ" => (3, "left"),
        _ => return None,
    })
}

/// `_lower_int` (_sse.py 129-321): the SSE2 integer block in NEON (Sprint 044 on). V31 and V30 are scratch; x16 and x17
/// too.
fn lower_int(
    a: &mut Assembler,
    ic: &str,
    dst: &SseLoc,
    src: &SseLoc,
    imm: Option<u64>,
    reg: &BTreeMap<Temp, u32>,
) -> Result<(), CompileError> {
    let bad = || CompileError::Refused(format!("sse {ic}"));
    let xmm = |l: &SseLoc| match l {
        SseLoc::X(n) => Ok(u32::from(*n)),
        _ => Err(CompileError::Refused(format!("sse {ic}: an XMM operand expected"))),
    };
    let gpr = |l: &SseLoc| match l {
        SseLoc::Loc(Loc::R { idx, .. }) => Ok(u32::from(*idx)),
        _ => Err(CompileError::Refused(format!("sse {ic}: a general register expected"))),
    };
    let addr_reg = |l: &SseLoc| match l.addr() {
        Some(t) => reg_at(reg, t),
        None => Err(CompileError::Refused(format!("sse {ic}: a memory operand expected"))),
    };
    let need_imm = |what: &str| imm.ok_or_else(|| CompileError::Refused(format!("sse {ic}: no immediate for {what}")));
    if matches!(ic, "MOVDQA" | "MOVDQU" | "MOVNTPS" | "MOVNTDQ") {
        // 131-138
        match (dst, src) {
            (SseLoc::X(d), SseLoc::X(s)) => a.emit(&[a64::mov_v(u32::from(*d), u32::from(*s))]),
            (SseLoc::X(d), _) => a.emit(&[a64::ldr_v(16, u32::from(*d), addr_reg(src)?, 0)]),
            _ => a.emit(&[a64::str_v(16, xmm(src)?, addr_reg(dst)?, 0)]),
        }
        return Ok(());
    }
    if ic == "PMOVMSKB" {
        // 139-149: the 16 sign bits into a general register
        let (v, rd) = (xmm(src)?, gpr(dst)?);
        for (lane, shift) in [(1, 8), (0, 0)] {
            a.emit(&[a64::umov_d(16, v, lane)]);
            a.emit(&a64::mov_imm(17, 0x8080_8080_8080_8080));
            a.emit(&[a64::and(16, 16, 17), a64::lsri(16, 16, 7)]);
            a.emit(&a64::mov_imm(17, 0x0102_0408_1020_4080));
            a.emit(&[a64::mul(16, 16, 17), a64::lsri(16, 16, 56)]);
            if shift != 0 {
                a.emit(&[a64::eor(rd, ZR, 16, LSL, 8)]);
            } else {
                a.emit(&[a64::orr(rd, rd, 16)]);
            }
        }
        return Ok(());
    }
    if ic == "MOVMSKPS" {
        // 187-193: the four sign bits into a general register; the bits do not overlap, so EOR is OR
        let (v, rd) = (xmm(src)?, gpr(dst)?);
        a.emit(&[a64::mov(rd, ZR)]);
        for lane in 0..4 {
            a.emit(&[
                a64::umov_s(16, v, lane),
                a64::lsri(16, 16, 31),
                a64::eor(rd, rd, 16, LSL, lane),
            ]);
        }
        return Ok(());
    }
    if ic == "PINSRW" {
        // 220-227: the source's low word into word imm & 7
        let (d, lane) = (xmm(dst)?, (need_imm("PINSRW")? & 7) as u32);
        match src {
            SseLoc::Loc(Loc::M { addr, .. }) => {
                a.emit(&[a64::ldr(2, 16, reg_at(reg, *addr)?, 0), a64::ins_h(d, lane, 16)]);
            }
            _ => a.emit(&[a64::ins_h(d, lane, gpr(src)?)]),
        }
        return Ok(());
    }
    if matches!(ic, "MOVHPS" | "MOVLPS") {
        // 228-233, 251-256: the high (low) quadword from or to memory
        let lane = u32::from(ic == "MOVHPS");
        match dst {
            SseLoc::X(d) => a.emit(&[a64::ldr(8, 16, addr_reg(src)?, 0), a64::ins_d(u32::from(*d), lane, 16)]),
            _ => a.emit(&[a64::umov_d(16, xmm(src)?, lane), a64::str(8, 16, addr_reg(dst)?, 0)]),
        }
        return Ok(());
    }
    let d = xmm(dst)?;
    match ic {
        "CVTDQ2PD" => {
            // 151-154: two signed words to doubles, exactly
            let v = sse_src(a, src, 8, reg, 31)?;
            a.emit(&[a64::sxtl_2d(30, v), a64::scvtf_2d(d, 30)]);
        }
        "PSRLDQ" | "PSLLDQ" => {
            // 155-162: whole-register byte shifts by imm8
            let n = need_imm(ic)?;
            if n > 15 {
                a.emit(&[a64::movi_zero(d)]);
            } else if n != 0 {
                let n = n as u32;
                a.emit(&[a64::movi_zero(31)]);
                a.emit(&[if ic == "PSRLDQ" {
                    a64::ext_v(d, d, 31, n)
                } else {
                    a64::ext_v(d, 31, d, 16 - n)
                }]);
            }
        }
        "PSRLQ" if imm.is_some() => {
            // 163-168
            let n = need_imm(ic)?;
            if n > 63 {
                a.emit(&[a64::movi_zero(d)]);
            } else if n != 0 {
                a.emit(&[a64::ushr_2d(d, d, n as u32)]);
            }
        }
        "PSRLQ" => {
            // 169-180: by a count in the source's low 64 bits
            let v = sse_src(a, src, 16, reg, 31)?;
            let (big, done) = (a.label(), a.label());
            a.emit(&[a64::umov_d(16, v, 0), a64::cmpi(16, 63)]);
            a.branch(|off| a64::bcond("hi", off), big);
            for lane in 0..2 {
                a.emit(&[a64::umov_d(17, d, lane), a64::lsrv(17, 17, 16), a64::ins_d(d, lane, 17)]);
            }
            a.branch(a64::b, done);
            a.bind(big);
            a.emit(&[a64::movi_zero(d)]);
            a.bind(done);
        }
        // 181-186: run in the model (string_compare.py, approximate.py)
        "PCMPISTRI" | "PCMPISTRI64" | "RCPPS" | "RSQRTPS" | "RCPSS" | "RSQRTSS" => a.emit(&[a64::hvc(HVC_SLOW_SSE)]),
        _ if shifts(ic).is_some() && imm.is_some() => {
            // 194-205: by imm8; past the lane, zero (logical) or the sign
            let (size, kind) = shifts(ic).unwrap();
            let bits = 8u64 << size;
            let n = need_imm(ic)?.min(bits) as u32;
            if kind == "arith" {
                if n != 0 {
                    a.emit(&[a64::shift_right(d, d, n, size, true)]);
                }
            } else if u64::from(n) >= bits {
                a.emit(&[a64::movi_zero(d)]);
            } else if n != 0 {
                a.emit(&[if kind == "right" {
                    a64::shift_right(d, d, n, size, false)
                } else {
                    a64::shift_left(d, d, n, size)
                }]);
            }
        }
        _ if shifts(ic).is_some() => {
            // 206-219: by the source's low 64 bits, clamped to the lane's bits
            let (size, kind) = shifts(ic).unwrap();
            let bits = 8u32 << size;
            let v = sse_src(a, src, 16, reg, 31)?;
            let small = a.label();
            a.emit(&[a64::umov_d(16, v, 0), a64::cmpi(16, bits)]);
            a.branch(|off| a64::bcond("ls", off), small);
            a.emit(&[a64::movz(16, u64::from(bits), 0)]);
            a.bind(small);
            if kind != "left" {
                a.emit(&[a64::sub(16, ZR, 16)]); // a negative count shifts right
            }
            a.emit(&[a64::dup_gpr(30, 16, size)]); // the shift count is the lane's low byte
            a.emit(&[a64::three_same(
                if kind == "arith" { "sshl" } else { "ushl" },
                d,
                d,
                30,
                size,
            )]);
        }
        "PSHUFLW" | "PSHUFHW" => {
            // 234-241: four words permuted by imm; the other four kept
            let n = need_imm(ic)?;
            let v = sse_src(a, src, 16, reg, 31)?;
            a.emit(&[a64::mov_v(30, v), a64::mov_v(d, 30)]);
            let base = if ic == "PSHUFLW" { 0 } else { 4 };
            for lane in 0..4u32 {
                a.emit(&[a64::ins_h_element(
                    d,
                    base + lane,
                    30,
                    base + (n >> (2 * lane) & 3) as u32,
                )]);
            }
        }
        "SHUFPS" => {
            // 242-250: words 0, 1 from the destination; 2, 3 from the source
            let n = need_imm(ic)?;
            let v = sse_src(a, src, 16, reg, 31)?;
            a.emit(&[a64::mov_v(30, d)]);
            if v != 31 {
                a.emit(&[a64::mov_v(31, v)]);
            }
            for lane in 0..4u32 {
                a.emit(&[a64::ins_s_element(
                    d,
                    lane,
                    if lane < 2 { 30 } else { 31 },
                    (n >> (2 * lane) & 3) as u32,
                )]);
            }
        }
        "MOVHLPS" | "MOVLHPS" => {
            // 257-260: one quadword between halves; the other kept
            let (v, hl) = (xmm(src)?, ic == "MOVHLPS");
            a.emit(&[a64::umov_d(16, v, u32::from(hl)), a64::ins_d(d, u32::from(!hl), 16)]);
        }
        "PSHUFD" => {
            // 261-267: word i of the result = source word (imm >> 2i) & 3
            let n = need_imm(ic)?;
            let v = sse_src(a, src, 16, reg, 31)?;
            a.emit(&[a64::mov_v(30, v)]);
            for lane in 0..4u32 {
                a.emit(&[a64::ins_s_element(d, lane, 30, (n >> (2 * lane) & 3) as u32)]);
            }
        }
        _ => {
            // 268-321: d = op(d, source)
            let v = sse_src(a, src, 16, reg, 31)?;
            match ic {
                "PAND" => a.emit(&[a64::and_v(d, d, v)]),
                "POR" | "ORPS" => a.emit(&[a64::orr_v(d, d, v)]),
                "PSUBD" => a.emit(&[a64::sub_v(d, d, v, 2)]),
                "PSUBQ" => a.emit(&[a64::sub_v(d, d, v, 3)]),
                "PUNPCKLBW" => a.emit(&[a64::zip1_v(d, d, v)]),
                "PCMPEQB" => a.emit(&[a64::cmeq_v(d, d, v)]),
                "PXOR" => a.emit(&[a64::eor_v(d, d, v)]),
                "ANDNPS" | "PANDN" => a.emit(&[a64::bic_v(d, v, d)]),
                "UNPCKLPS" | "PUNPCKLDQ" => a.emit(&[a64::zip1_4s(d, d, v)]),
                "UNPCKHPS" => a.emit(&[a64::zip2_4s(d, d, v)]),
                "PADDD" => a.emit(&[a64::add_v(d, d, v, 2)]),
                _ if lane_lowered(ic).is_some() => {
                    let (name, size) = lane_lowered(ic).unwrap();
                    a.emit(&[a64::three_same(name, d, d, v, size)]);
                }
                "PACKUSWB" => a.emit(&[a64::sqxtun(30, d, false), a64::sqxtun(30, v, true), a64::mov_v(d, 30)]),
                "PACKSSWB" => a.emit(&[a64::sqxtn(30, d, false), a64::sqxtn(30, v, true), a64::mov_v(d, 30)]),
                // dwords to signed words, saturated: the destination's low half, the source's high
                "PACKSSDW" => a.emit(&[a64::sqxtn_h(30, d, false), a64::sqxtn_h(30, v, true), a64::mov_v(d, 30)]),
                // each word's signed product, its high half
                "PMULHW" => a.emit(&[
                    a64::smull(30, d, v, false),
                    a64::smull(31, d, v, true),
                    a64::three_same("uzp2", d, 30, 31, 1),
                ]),
                // a byte's index with bit 7 set gives 0; TBL gives 0 for any index past 15
                "PSHUFB" => a.emit(&[a64::movi_bytes(30, 0x8F), a64::and_v(30, v, 30), a64::tbl(d, d, 30)]),
                // each quadword's eight absolute byte differences summed into its low word (SDM PSADBW)
                "PSADBW" => a.emit(&[
                    a64::three_same("uabd", 30, d, v, 0),
                    a64::add_pairwise_long(30, 30, 0, false),
                    a64::add_pairwise_long(30, 30, 1, false),
                    a64::add_pairwise_long(30, 30, 2, false),
                    a64::mov_v(d, 30),
                ]),
                "PMADDUBSW" => {
                    // the destination's unsigned bytes times the source's signed bytes, adjacent products added with
                    // signed saturation to a word; the source read whole before any scratch it may be is written
                    let free: Vec<u32> = [30, 31, 29].into_iter().filter(|t| *t != v).collect();
                    let (t1, t2) = (free[0], free[1]);
                    let t3 = if [29, 30, 31].contains(&v) { v } else { 29 };
                    a.emit(&[
                        a64::extend_bytes(t1, v, true, true),
                        a64::extend_bytes(t2, d, true, false),
                        a64::three_same("mul", t2, t2, t1, 1),
                        a64::add_pairwise_long(t2, t2, 1, true),
                        a64::extend_bytes(t1, v, false, true),
                        a64::extend_bytes(t3, d, false, false),
                        a64::three_same("mul", t3, t3, t1, 1),
                        a64::add_pairwise_long(t3, t3, 1, true),
                        a64::sqxtn_h(t1, t3, false),
                        a64::sqxtn_h(t1, t2, true),
                        a64::mov_v(d, t1),
                    ]);
                }
                _ => return Err(bad()),
            }
        }
    }
    Ok(())
}

/// A block's words and its marks (host VA, x86 address).
pub type Block = (Vec<u32>, Vec<(u64, u64)>);

/// `assemble_block` (922-970), without the block census: a block's code at `base`, the budget's SUBS and its branch,
/// each instruction with its flag arguments (the flags live after it, walked backward from `live_out`), the
/// fall-through exit to `exit_va` with x17 the next address, and the budget's stop (`bl slice_va`, `hvc #4`, back into
/// the body). Returns the words and the marks: the entry, each instruction, the budget's stop.
pub fn assemble_block(
    insns: &[Lifted],
    base: u64,
    exit_va: u64,
    slice_va: u64,
    state_va: u64,
    runtime: &RuntimeCalls,
    live_out: u32,
) -> Result<Block, CompileError> {
    let mut a = Assembler::new(base);
    let start = insns[0].address;
    let last = &insns[insns.len() - 1];
    let addr = last.address + last.length;
    let mut marks = vec![(a.here(), start)]; // the block entry: a guest boundary
    let (budget_stop, body) = (a.label(), a.label());
    a.emit(&[0xF100_0000 | (insns.len() as u32) << 10 | REG_BUDGET << 5 | REG_BUDGET]); // SUBS x23, x23, #n
    a.branch(|off| a64::bcond("mi", off), budget_stop);
    a.bind(body);
    let mut ended = false;
    let mut live = live_out;
    let mut lives = vec![ALL_FLAGS; insns.len()]; // the flags read after each instruction
    for k in (0..insns.len()).rev() {
        lives[k] = live;
        let setter = if k > 0 { nzcv_setter(&insns[k - 1]) } else { None };
        let (read, written) = insn_flags(&insns[k], setter)?;
        live = live & !written | read;
    }
    for (k, ins) in insns.iter().enumerate() {
        marks.push((a.here(), ins.address));
        let from = if k > 0 { nzcv_setter(&insns[k - 1]) } else { None };
        let out = k + 1 < insns.len() && nzcv_branch(ins, &insns[k + 1]);
        ended = compile_insn(&mut a, ins, exit_va, state_va, runtime, from, lives[k], out)?;
    }
    if !ended {
        a.emit(&a64::mov_imm(17, addr));
        a.branch(a64::b, Target::Va(exit_va));
    }
    a.bind(budget_stop); // an exit whose address maps to the block's first instruction, then on in
    marks.push((a.here(), start));
    a.branch(a64::bl, Target::Va(slice_va)); // the slice stub returns past the HVC when no exit is due
    a.emit(&[a64::hvc(4)]);
    a.branch(a64::b, body);
    Ok((a.finish(), marks))
}

/// `tests/test_operand_forms.py`'s four cases (the guards no class run can reach), carried by hand.
#[cfg(test)]
mod tests {
    use super::*;

    /// Sprint 166: a condition the tables do not name is refused by name, as the Python's dicts raise KeyError; `le`
    /// and its negation keep their flags, and `le` and `gt` invert each other.
    #[test]
    fn an_unknown_condition_is_refused_by_name() {
        assert_eq!(cond_flags("le").unwrap(), U_ZF | U_SF | U_OF);
        assert_eq!(cond_flags("nle").unwrap(), U_ZF | U_SF | U_OF);
        assert!(cond_flags("xx").unwrap_err().to_string().contains("\"xx\""));
        assert_eq!(inverse("le").unwrap(), "gt");
        assert_eq!(inverse("gt").unwrap(), "le");
        assert!(inverse("xx").unwrap_err().to_string().contains("\"xx\""));
    }

    /// Every condition the lifter gives (`lift::cc`'s 16) has its flags. No end-to-end judge reaches `cond_flags` for
    /// every name: the C words judge and the case runner compile one instruction at a time, where a Jcc, SETcc or
    /// CMOVcc holds no flags op, and the two guests' host-compiled blocks branch on no `le` (Sprint 166's plant).
    /// An instruction branching on a condition the table lacks is refused by name through `insn_flags`, the path the
    /// run loop's look-ahead takes (`w2m_vm::compile`), where the frozen Python's KeyError ends the run (Sprint 171).
    /// No lifted instruction carries such a condition (`lift::cc` gives only the 16 the table names), so it is built.
    #[test]
    fn insn_flags_refuses_an_unknown_condition_by_name() {
        let ins = Lifted {
            address: 0x1000,
            length: 2,
            iclass: "JXX".into(),
            ops: vec![Op::Br { cond: "xx", t: 0 }],
        };
        assert!(insn_flags(&ins, None).unwrap_err().to_string().contains("\"xx\""));
    }

    /// Each one's mask is `COND_FLAGS`'s (`src/w2m/vm_backend.py` 852-853), by value, a negation as its condition
    /// (Sprint 171; reviewer-2's High 1 of 2026-10-28: the test held only membership, so a wrong mask passed).
    #[test]
    fn every_condition_the_lifter_gives_has_its_flags() {
        // vm_backend 852-853: {"o": U_OF, "b": U_CF, "z": U_ZF, "s": U_SF, "p": U_PF, "be": U_CF | U_ZF,
        // "l": U_SF | U_OF, "le": U_ZF | U_SF | U_OF}
        let python = [
            ("o", U_OF),
            ("b", U_CF),
            ("z", U_ZF),
            ("s", U_SF),
            ("p", U_PF),
            ("be", U_CF | U_ZF),
            ("l", U_SF | U_OF),
            ("le", U_ZF | U_SF | U_OF),
        ];
        for suffix in [
            "O", "NO", "B", "NB", "Z", "NZ", "BE", "NBE", "S", "NS", "P", "NP", "L", "NL", "LE", "NLE",
        ] {
            let cc = crate::lift::cc(suffix).unwrap();
            let base = cc.strip_prefix('n').unwrap_or(cc);
            let want = python.iter().find(|(k, _)| *k == base).map(|(_, m)| *m);
            assert_eq!(cond_flags(cc).ok(), want, "{cc}");
        }
    }

    const RAX: u8 = 0;
    const RBX: u8 = 3;

    fn r(idx: u8, bits: u32) -> Loc {
        Loc::R { idx, bits, hi: false }
    }

    fn last_reads(ops: &[Op]) -> BTreeMap<Temp, usize> {
        let mut last = BTreeMap::new();
        for (i, op) in ops.iter().enumerate() {
            for t in temps_read(op) {
                last.insert(t, i);
            }
        }
        last
    }

    fn add_then_store(dst_bits: u32, a_bits: u32, b_bits: u32, alu_bits: u32, dst: u8, flags: bool) -> Vec<Op> {
        let mut ops = vec![
            Op::Ld {
                t: 1,
                loc: r(RAX, a_bits),
            },
            Op::Ld {
                t: 2,
                loc: r(RBX, b_bits),
            },
            Op::Alu {
                t: 3,
                op: "add",
                a: 1,
                b: 2,
                bits: alu_bits,
            },
        ];
        if flags {
            ops.push(Op::Flags {
                kind: "add",
                a: 1,
                b: 2,
                r: 3,
                bits: alu_bits,
            });
        }
        ops.push(Op::St {
            loc: r(dst, dst_bits),
            t: 3,
        });
        ops
    }

    #[test]
    fn a_result_is_not_forwarded_past_a_reader_of_the_old_value() {
        let ops = add_then_store(64, 64, 64, 64, RAX, true);
        let (alias, _, forward) = operand_forms(&ops, &last_reads(&ops), &BTreeSet::new());
        assert_eq!(alias, BTreeMap::from([(1, 0), (2, 3)]));
        assert!(forward.is_empty()); // the flags op reads RAX's old value after the add
    }

    #[test]
    fn a_result_with_dead_flags_is_computed_in_place() {
        let ops = add_then_store(64, 64, 64, 64, RAX, true);
        let last = BTreeMap::from([(1, 2), (2, 2), (3, 4)]); // the dead flags op reads nothing
        let (alias, _, forward) = operand_forms(&ops, &last, &BTreeSet::from([3]));
        assert_eq!(alias, BTreeMap::from([(1, 0), (2, 3)]));
        assert_eq!(forward, BTreeMap::from([(3, 0)]));
    }

    #[test]
    fn a_32_bit_register_with_a_64_bit_reader_is_copied() {
        let ops = add_then_store(64, 32, 64, 64, RBX, false);
        let (alias, _, _) = operand_forms(&ops, &last_reads(&ops), &BTreeSet::new());
        assert!(!alias.contains_key(&1)); // the add would read RAX's upper half
    }

    /// The offsets the group 3 lowerings use (Sprint 159) equal the guest layout the runtime is compiled against;
    /// DF_BYTE and SPILL have no twin there and stand on vm_backend 94 and 110.
    #[test]
    fn the_lowerings_offsets_are_the_guest_layout_s() {
        use guest_runtime::layout as l;
        assert_eq!(u64::from(MXCSR_OFF), l::MXCSR_OFF);
        assert_eq!(u64::from(FCW_OFF), l::FCW_OFF);
        assert_eq!(u64::from(PAUSE_COUNT), l::PAUSE_COUNT);
        assert_eq!(u64::from(SLICE_UNTIL), l::SLICE_UNTIL);
        assert_eq!(
            RUNTIME_STACK_TOP,
            l::MONITOR_VA + l::RUNTIME_STACK + l::RUNTIME_STACK_SIZE
        );
    }

    #[test]
    fn a_32_bit_register_read_by_32_bit_ops_is_read_in_place() {
        let ops = add_then_store(32, 32, 32, 32, RAX, false);
        let (alias, _, forward) = operand_forms(&ops, &last_reads(&ops), &BTreeSet::new());
        assert_eq!(alias, BTreeMap::from([(1, 0), (2, 3)]));
        assert_eq!(forward, BTreeMap::from([(3, 0)]));
    }
}
