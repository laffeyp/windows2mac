// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The AArch64 instructions w2m emits (Sprint 135), carried function for function from `src/w2m/a64_encoder.py`
//! (Sprint 030 on). Each function returns one 32-bit word; encodings follow the Arm ARM (DDI 0487) section C6, and
//! `tests/clang.rs` assembles the same text with clang and requires equal words, the 247 cases of
//! `tests/test_a64_encoder.py` and the fixed words the monitor code uses. Registers are 0 to 31; 31 is XZR or SP as
//! the instruction defines it. A register or immediate out of range panics, as the Python's asserts raised.

/// The condition codes (`COND`).
pub fn cond(name: &str) -> u32 {
    match name {
        "eq" => 0,
        "ne" => 1,
        "cs" | "hs" => 2,
        "cc" | "lo" => 3,
        "mi" => 4,
        "pl" => 5,
        "vs" => 6,
        "vc" => 7,
        "hi" => 8,
        "ls" => 9,
        "ge" => 10,
        "lt" => 11,
        "gt" => 12,
        "le" => 13,
        "al" => 14,
        _ => panic!("condition {name}"),
    }
}

pub const ZR: u32 = 31;
pub const LSL: u32 = 0;
pub const LSR: u32 = 1;

fn r(regs: &[u32]) {
    for &x in regs {
        assert!(x <= 31, "register {x}");
    }
}

/// `offset >> 2` masked to `bits`, as Python's arithmetic shift of a negative offset then mask gives it.
fn field(offset: i64, shift: u32, mask: u32) -> u32 {
    ((offset >> shift) as u32) & mask
}

pub fn movz(rd: u32, imm16: u64, hw: u32) -> u32 {
    r(&[rd]);
    0xD280_0000 | hw << 21 | ((imm16 & 0xFFFF) as u32) << 5 | rd
}

pub fn movk(rd: u32, imm16: u64, hw: u32) -> u32 {
    r(&[rd]);
    0xF280_0000 | hw << 21 | ((imm16 & 0xFFFF) as u32) << 5 | rd
}

/// MOVZ, then MOVK for each further nonzero halfword of a 64-bit value.
pub fn mov_imm(rd: u32, value: u64) -> Vec<u32> {
    let mut out = vec![movz(rd, value & 0xFFFF, 0)];
    for hw in 1..4 {
        let h = (value >> (16 * hw)) & 0xFFFF;
        if h != 0 {
            out.push(movk(rd, h, hw));
        }
    }
    out
}

fn sreg(base: u32, rd: u32, rn: u32, rm: u32, shift: u32, amount: u32) -> u32 {
    r(&[rd, rn, rm]);
    base | shift << 22 | rm << 16 | (amount & 63) << 10 | rn << 5 | rd
}

pub fn add(rd: u32, rn: u32, rm: u32, shift: u32, amount: u32) -> u32 {
    sreg(0x8B00_0000, rd, rn, rm, shift, amount)
}

pub fn adds(rd: u32, rn: u32, rm: u32) -> u32 {
    sreg(0xAB00_0000, rd, rn, rm, 0, 0)
}

pub fn sub(rd: u32, rn: u32, rm: u32) -> u32 {
    sreg(0xCB00_0000, rd, rn, rm, 0, 0)
}

pub fn subs(rd: u32, rn: u32, rm: u32) -> u32 {
    sreg(0xEB00_0000, rd, rn, rm, 0, 0)
}

pub fn and(rd: u32, rn: u32, rm: u32) -> u32 {
    sreg(0x8A00_0000, rd, rn, rm, 0, 0)
}

pub fn bic(rd: u32, rn: u32, rm: u32) -> u32 {
    sreg(0x8A20_0000, rd, rn, rm, 0, 0)
}

pub fn orr(rd: u32, rn: u32, rm: u32) -> u32 {
    sreg(0xAA00_0000, rd, rn, rm, 0, 0)
}

pub fn eor(rd: u32, rn: u32, rm: u32, shift: u32, amount: u32) -> u32 {
    sreg(0xCA00_0000, rd, rn, rm, shift, amount)
}

pub fn mov(rd: u32, rm: u32) -> u32 {
    orr(rd, ZR, rm)
}

/// MOV Wd, Wm: writes the low 32 bits and zeroes the upper 32.
pub fn mov32(rd: u32, rm: u32) -> u32 {
    r(&[rd, rm]);
    0x2A00_03E0 | rm << 16 | rd
}

pub fn addi(rd: u32, rn: u32, imm12: u32) -> u32 {
    r(&[rd, rn]);
    assert!(imm12 < 4096);
    0x9100_0000 | imm12 << 10 | rn << 5 | rd
}

pub fn subi(rd: u32, rn: u32, imm12: u32) -> u32 {
    r(&[rd, rn]);
    assert!(imm12 < 4096);
    0xD100_0000 | imm12 << 10 | rn << 5 | rd
}

/// ADD Wd, Wn, #imm12: the 32-bit sum, the upper 32 bits zeroed.
pub fn addi32(rd: u32, rn: u32, imm12: u32) -> u32 {
    r(&[rd, rn]);
    assert!(imm12 < 4096);
    0x1100_0000 | imm12 << 10 | rn << 5 | rd
}

pub fn subi32(rd: u32, rn: u32, imm12: u32) -> u32 {
    r(&[rd, rn]);
    assert!(imm12 < 4096);
    0x5100_0000 | imm12 << 10 | rn << 5 | rd
}

/// ADDS Wd, Wn, Wm, LSL #amount.
pub fn adds32(rd: u32, rn: u32, rm: u32, amount: u32) -> u32 {
    r(&[rd, rn, rm]);
    assert!(amount < 32);
    0x2B00_0000 | rm << 16 | amount << 10 | rn << 5 | rd
}

/// SUBS Wd, Wn, Wm, LSL #amount.
pub fn subs32(rd: u32, rn: u32, rm: u32, amount: u32) -> u32 {
    r(&[rd, rn, rm]);
    assert!(amount < 32);
    0x6B00_0000 | rm << 16 | amount << 10 | rn << 5 | rd
}

/// LSL Wd, Wn, #amount (UBFM Wd, Wn, #(32 - amount) % 32, #31 - amount).
pub fn lsl32(rd: u32, rn: u32, amount: u32) -> u32 {
    r(&[rd, rn]);
    assert!(amount < 32);
    0x5300_0000 | ((32 - amount) % 32) << 16 | (31 - amount) << 10 | rn << 5 | rd
}

/// AND Xd, Xn, #(1 << bits) - 1: the logical immediate of `bits` low ones.
pub fn and_low(rd: u32, rn: u32, bits: u32) -> u32 {
    r(&[rd, rn]);
    assert!((1..64).contains(&bits));
    0x9240_0000 | (bits - 1) << 10 | rn << 5 | rd
}

pub fn cmpi(rn: u32, imm12: u32) -> u32 {
    r(&[rn]);
    assert!(imm12 < 4096);
    0xF100_0000 | imm12 << 10 | rn << 5 | ZR
}

pub fn cmp(rn: u32, rm: u32) -> u32 {
    subs(ZR, rn, rm)
}

pub fn ubfm(rd: u32, rn: u32, immr: u32, imms: u32) -> u32 {
    r(&[rd, rn]);
    0xD340_0000 | immr << 16 | imms << 10 | rn << 5 | rd
}

pub fn ubfx(rd: u32, rn: u32, lsb: u32, width: u32) -> u32 {
    assert!(width > 0 && lsb + width <= 64);
    ubfm(rd, rn, lsb, lsb + width - 1)
}

pub fn lsri(rd: u32, rn: u32, shift: u32) -> u32 {
    ubfm(rd, rn, shift, 63)
}

pub fn bfi(rd: u32, rn: u32, lsb: u32, width: u32) -> u32 {
    r(&[rd, rn]);
    assert!(width > 0 && lsb + width <= 64);
    0xB340_0000 | ((64 - lsb) % 64) << 16 | (width - 1) << 10 | rn << 5 | rd
}

pub fn sxtw(rd: u32, rn: u32) -> u32 {
    r(&[rd, rn]);
    0x9340_7C00 | rn << 5 | rd
}

/// UDIV Xd, Xn, Xm: a zero divisor gives 0, no trap.
pub fn udiv(rd: u32, rn: u32, rm: u32) -> u32 {
    r(&[rd, rn, rm]);
    0x9AC0_0800 | rm << 16 | rn << 5 | rd
}

/// LDAXP Xt, Xt2, [Xn]: a 16-byte exclusive load-acquire.
pub fn ldaxp(rt: u32, rt2: u32, rn: u32) -> u32 {
    r(&[rt, rt2, rn]);
    0xC87F_8000 | rt2 << 10 | rn << 5 | rt
}

/// STLXP Ws, Xt, Xt2, [Xn]: Ws = 0 when the store-release succeeded.
pub fn stlxp(rs: u32, rt: u32, rt2: u32, rn: u32) -> u32 {
    r(&[rs, rt, rt2, rn]);
    0xC820_8000 | rs << 16 | rt2 << 10 | rn << 5 | rt
}

pub fn sdiv(rd: u32, rn: u32, rm: u32) -> u32 {
    r(&[rd, rn, rm]);
    0x9AC0_0C00 | rm << 16 | rn << 5 | rd
}

/// MSUB Xd, Xn, Xm, Xa: Xa - Xn * Xm.
pub fn msub(rd: u32, rn: u32, rm: u32, ra: u32) -> u32 {
    r(&[rd, rn, rm, ra]);
    0x9B00_8000 | rm << 16 | ra << 10 | rn << 5 | rd
}

/// REV Xd, Xn (bits 64) or REV Wd, Wn (bits 32).
pub fn rev(rd: u32, rn: u32, bits: u32) -> u32 {
    r(&[rd, rn]);
    (if bits == 64 { 0xDAC0_0C00 } else { 0x5AC0_0800 }) | rn << 5 | rd
}

pub fn clz(rd: u32, rn: u32) -> u32 {
    r(&[rd, rn]);
    0xDAC0_1000 | rn << 5 | rd
}

pub fn rbit(rd: u32, rn: u32) -> u32 {
    r(&[rd, rn]);
    0xDAC0_0000 | rn << 5 | rd
}

pub fn lsrv(rd: u32, rn: u32, rm: u32) -> u32 {
    r(&[rd, rn, rm]);
    0x9AC0_2400 | rm << 16 | rn << 5 | rd
}

pub fn lslv(rd: u32, rn: u32, rm: u32) -> u32 {
    r(&[rd, rn, rm]);
    0x9AC0_2000 | rm << 16 | rn << 5 | rd
}

pub fn asrv(rd: u32, rn: u32, rm: u32) -> u32 {
    r(&[rd, rn, rm]);
    0x9AC0_2800 | rm << 16 | rn << 5 | rd
}

/// ROR Xd, Xn, Xm, or Wd, Wn, Wm for bits 32.
pub fn rorv(rd: u32, rn: u32, rm: u32, bits: u32) -> u32 {
    r(&[rd, rn, rm]);
    (if bits == 64 { 0x9AC0_2C00 } else { 0x1AC0_2C00 }) | rm << 16 | rn << 5 | rd
}

/// SBFX Xd, Xn, #lsb, #width.
pub fn sbfx(rd: u32, rn: u32, lsb: u32, width: u32) -> u32 {
    r(&[rd, rn]);
    assert!(width > 0 && lsb + width <= 64);
    0x9340_0000 | lsb << 16 | (lsb + width - 1) << 10 | rn << 5 | rd
}

pub fn smulh(rd: u32, rn: u32, rm: u32) -> u32 {
    r(&[rd, rn, rm]);
    0x9B40_7C00 | rm << 16 | rn << 5 | rd
}

/// The size field for 1, 2, 4 or 8 bytes (`_SZ`).
fn sz(size: u32) -> u32 {
    match size {
        1 => 0,
        2 => 1,
        4 => 2,
        8 => 3,
        _ => panic!("size {size}"),
    }
}

/// LDAXRB/LDAXRH/LDAXR Wt/LDAXR Xt, [Xn].
pub fn ldaxr(size: u32, rt: u32, rn: u32) -> u32 {
    r(&[rt, rn]);
    0x085F_FC00 | sz(size) << 30 | rn << 5 | rt
}

/// STLXRB/STLXRH/STLXR Ws, Wt|Xt, [Xn].
pub fn stlxr(size: u32, rs: u32, rt: u32, rn: u32) -> u32 {
    r(&[rs, rt, rn]);
    0x0800_FC00 | sz(size) << 30 | rs << 16 | rn << 5 | rt
}

pub fn mul(rd: u32, rn: u32, rm: u32) -> u32 {
    r(&[rd, rn, rm]);
    0x9B00_7C00 | rm << 16 | rn << 5 | rd
}

pub fn umulh(rd: u32, rn: u32, rm: u32) -> u32 {
    r(&[rd, rn, rm]);
    0x9BC0_7C00 | rm << 16 | rn << 5 | rd
}

pub fn cset(rd: u32, c: &str) -> u32 {
    r(&[rd]);
    0x9A9F_07E0 | (cond(c) ^ 1) << 12 | rd
}

fn by_size(size: u32, table: [u32; 4]) -> u32 {
    table[sz(size) as usize]
}

/// LDRB/LDRH/LDR Wt/LDR Xt [Xn, Xm{, LSL #log2(size)}].
pub fn ldr_reg(size: u32, rt: u32, rn: u32, rm: u32, scaled: bool) -> u32 {
    r(&[rt, rn, rm]);
    by_size(size, [0x3860_6800, 0x7860_6800, 0xB860_6800, 0xF860_6800])
        | rm << 16
        | if scaled && size > 1 { 1 << 12 } else { 0 }
        | rn << 5
        | rt
}

/// STRB/STRH/STR Wt/STR Xt [Xn, Xm{, LSL #log2(size)}].
pub fn str_reg(size: u32, rt: u32, rn: u32, rm: u32, scaled: bool) -> u32 {
    r(&[rt, rn, rm]);
    by_size(size, [0x3820_6800, 0x7820_6800, 0xB820_6800, 0xF820_6800])
        | rm << 16
        | if scaled && size > 1 { 1 << 12 } else { 0 }
        | rn << 5
        | rt
}

/// LDURB/LDURH/LDUR [Xn, #offset], offset -256..255 unscaled.
pub fn ldur(size: u32, rt: u32, rn: u32, offset: i64) -> u32 {
    r(&[rt, rn]);
    assert!((-256..=255).contains(&offset));
    by_size(size, [0x3840_0000, 0x7840_0000, 0xB840_0000, 0xF840_0000]) | field(offset, 0, 0x1FF) << 12 | rn << 5 | rt
}

/// STURB/STURH/STUR [Xn, #offset], offset -256..255 unscaled.
pub fn stur(size: u32, rt: u32, rn: u32, offset: i64) -> u32 {
    r(&[rt, rn]);
    assert!((-256..=255).contains(&offset));
    by_size(size, [0x3800_0000, 0x7800_0000, 0xB800_0000, 0xF800_0000]) | field(offset, 0, 0x1FF) << 12 | rn << 5 | rt
}

/// LDRB/LDRH/LDR Wt/LDR Xt [Xn, #offset]; zero-extends; offset scaled by size.
pub fn ldr(size: u32, rt: u32, rn: u32, offset: u32) -> u32 {
    r(&[rt, rn]);
    assert!(offset.is_multiple_of(size) && offset / size < 4096);
    by_size(size, [0x3940_0000, 0x7940_0000, 0xB940_0000, 0xF940_0000]) | (offset / size) << 10 | rn << 5 | rt
}

pub fn str(size: u32, rt: u32, rn: u32, offset: u32) -> u32 {
    r(&[rt, rn]);
    assert!(offset.is_multiple_of(size) && offset / size < 4096);
    by_size(size, [0x3900_0000, 0x7900_0000, 0xB900_0000, 0xF900_0000]) | (offset / size) << 10 | rn << 5 | rt
}

pub fn ldp(rt: u32, rt2: u32, rn: u32, offset: i64) -> u32 {
    r(&[rt, rt2, rn]);
    assert!(offset % 8 == 0 && (-64..64).contains(&(offset / 8)));
    0xA940_0000 | field(offset / 8, 0, 0x7F) << 15 | rt2 << 10 | rn << 5 | rt
}

/// STP Xt, Xt2, [Xn|SP, #offset] (signed offset form).
pub fn stp(rt: u32, rt2: u32, rn: u32, offset: i64) -> u32 {
    r(&[rt, rt2, rn]);
    assert!(offset % 8 == 0 && (-64..64).contains(&(offset / 8)));
    0xA900_0000 | field(offset / 8, 0, 0x7F) << 15 | rt2 << 10 | rn << 5 | rt
}

pub fn b(offset: i64) -> u32 {
    assert!(offset % 4 == 0 && (-(1 << 27)..1 << 27).contains(&offset));
    0x1400_0000 | field(offset, 2, 0x3FF_FFFF)
}

pub fn bl(offset: i64) -> u32 {
    assert!(offset % 4 == 0 && (-(1 << 27)..1 << 27).contains(&offset));
    0x9400_0000 | field(offset, 2, 0x3FF_FFFF)
}

/// RET (X30).
pub fn ret() -> u32 {
    0xD65F_03C0
}

pub fn bcond(c: &str, offset: i64) -> u32 {
    assert!(offset % 4 == 0 && (-(1 << 20)..1 << 20).contains(&offset));
    0x5400_0000 | field(offset, 2, 0x7FFFF) << 5 | cond(c)
}

pub fn cbz(rt: u32, offset: i64) -> u32 {
    r(&[rt]);
    assert!(offset % 4 == 0 && (-(1 << 20)..1 << 20).contains(&offset));
    0xB400_0000 | field(offset, 2, 0x7FFFF) << 5 | rt
}

pub fn cbnz(rt: u32, offset: i64) -> u32 {
    cbz(rt, offset) | 1 << 24
}

pub fn tbz(rt: u32, bit: u32, offset: i64) -> u32 {
    r(&[rt]);
    assert!(offset % 4 == 0 && (-(1 << 15)..1 << 15).contains(&offset));
    0x3600_0000 | (bit >> 5) << 31 | (bit & 31) << 19 | field(offset, 2, 0x3FFF) << 5 | rt
}

pub fn br(rn: u32) -> u32 {
    r(&[rn]);
    0xD61F_0000 | rn << 5
}

pub fn blr(rn: u32) -> u32 {
    r(&[rn]);
    0xD63F_0000 | rn << 5
}

pub fn hvc(imm16: u32) -> u32 {
    0xD400_0002 | (imm16 & 0xFFFF) << 5
}

// ---- SIMD and floating point (Sprint 041) -------------------------------------------------------------------------
// Registers are V0-V31; `size` is 4 (single) or 8 (double); scalar forms act on the low element.

/// `_FTYPE`: 0 single, 1 double.
fn ftype(size: u32) -> u32 {
    match size {
        4 => 0,
        8 => 1,
        _ => panic!("FP size {size}"),
    }
}

/// LDR St/Dt/Qt, [Xn, #offset] (unsigned offset, scaled); the rest of Vt is zeroed.
pub fn ldr_v(size: u32, vt: u32, rn: u32, offset: u32) -> u32 {
    r(&[vt, rn]);
    assert!(offset.is_multiple_of(size) && offset / size < 4096);
    let base = match size {
        4 => 0xBD40_0000,
        8 => 0xFD40_0000,
        16 => 0x3DC0_0000,
        _ => panic!("size {size}"),
    };
    base | (offset / size) << 10 | rn << 5 | vt
}

/// STR St/Dt/Qt, [Xn, #offset].
pub fn str_v(size: u32, vt: u32, rn: u32, offset: u32) -> u32 {
    r(&[vt, rn]);
    assert!(offset.is_multiple_of(size) && offset / size < 4096);
    let base = match size {
        4 => 0xBD00_0000,
        8 => 0xFD00_0000,
        16 => 0x3D80_0000,
        _ => panic!("size {size}"),
    };
    base | (offset / size) << 10 | rn << 5 | vt
}

/// STP Qt, Qt2, [Xn, #offset] (signed offset, scaled by 16).
pub fn stp_q(vt: u32, vt2: u32, rn: u32, offset: i64) -> u32 {
    r(&[vt, vt2, rn]);
    assert!(offset % 16 == 0 && (-64..64).contains(&(offset / 16)));
    0xAD00_0000 | field(offset / 16, 0, 0x7F) << 15 | vt2 << 10 | rn << 5 | vt
}

pub fn ldp_q(vt: u32, vt2: u32, rn: u32, offset: i64) -> u32 {
    r(&[vt, vt2, rn]);
    assert!(offset % 16 == 0 && (-64..64).contains(&(offset / 16)));
    0xAD40_0000 | field(offset / 16, 0, 0x7F) << 15 | vt2 << 10 | rn << 5 | vt
}

/// MOV Vd.16B, Vn.16B (ORR).
pub fn mov_v(vd: u32, vn: u32) -> u32 {
    r(&[vd, vn]);
    0x4EA0_1C00 | vn << 16 | vn << 5 | vd
}

/// INS Vd.S[0], Vn.S[0] or Vd.D[0], Vn.D[0].
pub fn ins_lane0(size: u32, vd: u32, vn: u32) -> u32 {
    r(&[vd, vn]);
    (match size {
        4 => 0x6E04_0400,
        8 => 0x6E08_0400,
        _ => panic!("size {size}"),
    }) | vn << 5
        | vd
}

pub fn eor_v(vd: u32, vn: u32, vm: u32) -> u32 {
    r(&[vd, vn, vm]);
    0x6E20_1C00 | vm << 16 | vn << 5 | vd
}

pub fn and_v(vd: u32, vn: u32, vm: u32) -> u32 {
    r(&[vd, vn, vm]);
    0x4E20_1C00 | vm << 16 | vn << 5 | vd
}

pub fn orr_v(vd: u32, vn: u32, vm: u32) -> u32 {
    r(&[vd, vn, vm]);
    0x4EA0_1C00 | vm << 16 | vn << 5 | vd
}

/// SUB Vd.<T>, size 2 for .4S, 3 for .2D.
pub fn sub_v(vd: u32, vn: u32, vm: u32, size: u32) -> u32 {
    r(&[vd, vn, vm]);
    0x6E20_8400 | size << 22 | vm << 16 | vn << 5 | vd
}

/// EXT Vd.16B, Vn.16B, Vm.16B, #index.
pub fn ext_v(vd: u32, vn: u32, vm: u32, index: u32) -> u32 {
    r(&[vd, vn, vm]);
    assert!(index < 16);
    0x6E00_0000 | vm << 16 | index << 11 | vn << 5 | vd
}

pub fn zip1_v(vd: u32, vn: u32, vm: u32) -> u32 {
    r(&[vd, vn, vm]);
    0x4E00_3800 | vm << 16 | vn << 5 | vd
}

pub fn cmeq_v(vd: u32, vn: u32, vm: u32) -> u32 {
    r(&[vd, vn, vm]);
    0x6E20_8C00 | vm << 16 | vn << 5 | vd
}

pub fn sxtl_2d(vd: u32, vn: u32) -> u32 {
    r(&[vd, vn]);
    0x0F20_A400 | vn << 5 | vd
}

pub fn scvtf_2d(vd: u32, vn: u32) -> u32 {
    r(&[vd, vn]);
    0x4E61_D800 | vn << 5 | vd
}

pub fn scvtf_4s(vd: u32, vn: u32) -> u32 {
    r(&[vd, vn]);
    0x4E21_D800 | vn << 5 | vd
}

pub fn fcvtl_2d(vd: u32, vn: u32) -> u32 {
    r(&[vd, vn]);
    0x0E61_7800 | vn << 5 | vd
}

pub fn fcvtn_2s(vd: u32, vn: u32) -> u32 {
    r(&[vd, vn]);
    0x0E61_6800 | vn << 5 | vd
}

/// UMOV Xd, Vn.D[lane].
pub fn umov_d(rd: u32, vn: u32, lane: u32) -> u32 {
    r(&[rd, vn]);
    0x4E00_3C00 | ((lane << 4) | 0b1000) << 16 | vn << 5 | rd
}

/// INS Vd.D[lane], Xn.
pub fn ins_d(vd: u32, lane: u32, rn: u32) -> u32 {
    r(&[vd, rn]);
    0x4E00_1C00 | ((lane << 4) | 0b1000) << 16 | rn << 5 | vd
}

/// INS Vd.S[lane], Vn.S[src_lane].
pub fn ins_s_element(vd: u32, lane: u32, vn: u32, src_lane: u32) -> u32 {
    r(&[vd, vn]);
    0x6E00_0400 | ((lane << 3) | 0b100) << 16 | (src_lane << 2) << 11 | vn << 5 | vd
}

/// MOVI Vd.2D, #0.
pub fn movi_zero(vd: u32) -> u32 {
    r(&[vd]);
    0x6F00_E400 | vd
}

/// USHR Vd.2D, Vn.2D, #shift (1-64).
pub fn ushr_2d(vd: u32, vn: u32, shift: u32) -> u32 {
    r(&[vd, vn]);
    assert!((1..=64).contains(&shift));
    0x6F40_0400 | (128 - shift) << 16 | vn << 5 | vd
}

pub fn add_v(vd: u32, vn: u32, vm: u32, size: u32) -> u32 {
    r(&[vd, vn, vm]);
    0x4E20_8400 | size << 22 | vm << 16 | vn << 5 | vd
}

pub fn bic_v(vd: u32, vn: u32, vm: u32) -> u32 {
    r(&[vd, vn, vm]);
    0x4E60_1C00 | vm << 16 | vn << 5 | vd
}

pub fn zip1_4s(vd: u32, vn: u32, vm: u32) -> u32 {
    r(&[vd, vn, vm]);
    0x4E80_3800 | vm << 16 | vn << 5 | vd
}

pub fn zip2_4s(vd: u32, vn: u32, vm: u32) -> u32 {
    r(&[vd, vn, vm]);
    0x4E80_7800 | vm << 16 | vn << 5 | vd
}

pub fn ushr_4s(vd: u32, vn: u32, shift: u32) -> u32 {
    r(&[vd, vn]);
    assert!((1..=32).contains(&shift));
    0x6F00_0400 | (64 - shift) << 16 | vn << 5 | vd
}

pub fn sshr_4s(vd: u32, vn: u32, shift: u32) -> u32 {
    r(&[vd, vn]);
    assert!((1..=32).contains(&shift));
    0x4F00_0400 | (64 - shift) << 16 | vn << 5 | vd
}

pub fn shl_4s(vd: u32, vn: u32, shift: u32) -> u32 {
    r(&[vd, vn]);
    assert!(shift <= 31);
    0x4F00_5400 | (32 + shift) << 16 | vn << 5 | vd
}

pub fn dup_4s(vd: u32, rn: u32) -> u32 {
    r(&[vd, rn]);
    0x4E04_0C00 | rn << 5 | vd
}

pub fn ushl_4s(vd: u32, vn: u32, vm: u32) -> u32 {
    r(&[vd, vn, vm]);
    0x6EA0_4400 | vm << 16 | vn << 5 | vd
}

pub fn sshl_4s(vd: u32, vn: u32, vm: u32) -> u32 {
    r(&[vd, vn, vm]);
    0x4EA0_4400 | vm << 16 | vn << 5 | vd
}

/// UMOV Wd, Vn.S[lane].
pub fn umov_s(rd: u32, vn: u32, lane: u32) -> u32 {
    r(&[rd, vn]);
    0x0E00_3C00 | ((lane << 3) | 0b100) << 16 | vn << 5 | rd
}

/// NEON three-same on a whole register (Q=1), by name (`THREE_SAME`).
fn three_same_base(name: &str) -> u32 {
    match name {
        "add" => 0x4E20_8400,
        "sub" => 0x6E20_8400,
        "cmeq" => 0x6E20_8C00,
        "cmgt" => 0x4E20_3400,
        "uqsub" => 0x6E20_2C00,
        "sqsub" => 0x4E20_2C00,
        "sqadd" => 0x4E20_0C00,
        "umax" => 0x6E20_6400,
        "mul" => 0x4E20_9C00,
        "zip1" => 0x4E00_3800,
        "zip2" => 0x4E00_7800,
        "ushl" => 0x6E20_4400,
        "sshl" => 0x4E20_4400,
        "uqadd" => 0x6E20_0C00,
        "uzp2" => 0x4E00_5800,
        "uabd" => 0x6E20_7400,
        _ => panic!("three-same {name}"),
    }
}

/// <name> Vd.<T>, Vn.<T>, Vm.<T> for T = 16B, 8H, 4S, 2D by size 0-3.
pub fn three_same(name: &str, vd: u32, vn: u32, vm: u32, size: u32) -> u32 {
    r(&[vd, vn, vm]);
    three_same_base(name) | size << 22 | vm << 16 | vn << 5 | vd
}

/// <name> on the low 64 bits (Q=0): T = 8B, 4H, 2S by size 0-2.
pub fn three_same_d(name: &str, vd: u32, vn: u32, vm: u32, size: u32) -> u32 {
    assert!(size < 3);
    three_same(name, vd, vn, vm, size) & !(1 << 30)
}

/// USHR or SSHR Vd.<T>, Vn.<T>, #shift (1 to the lane's bits) for size 1-3.
pub fn shift_right(vd: u32, vn: u32, shift: u32, size: u32, signed: bool) -> u32 {
    r(&[vd, vn]);
    let bits = 8 << size;
    assert!((1..=bits).contains(&shift) && size >= 1);
    (if signed { 0x4F00_0400 } else { 0x6F00_0400 }) | (2 * bits - shift) << 16 | vn << 5 | vd
}

/// SHL Vd.<T>, Vn.<T>, #shift (0 to the lane's bits - 1) for size 1-3.
pub fn shift_left(vd: u32, vn: u32, shift: u32, size: u32) -> u32 {
    r(&[vd, vn]);
    let bits = 8 << size;
    assert!(shift < bits && size >= 1);
    0x4F00_5400 | (bits + shift) << 16 | vn << 5 | vd
}

/// DUP Vd.<T>, Wn (sizes 0-2) or Xn (size 3).
pub fn dup_gpr(vd: u32, rn: u32, size: u32) -> u32 {
    r(&[vd, rn]);
    0x4E00_0C00 | (1 << size) << 16 | rn << 5 | vd
}

pub fn sqxtun(vd: u32, vn: u32, upper: bool) -> u32 {
    r(&[vd, vn]);
    (if upper { 0x6E21_2800 } else { 0x2E21_2800 }) | vn << 5 | vd
}

pub fn sqxtn(vd: u32, vn: u32, upper: bool) -> u32 {
    r(&[vd, vn]);
    (if upper { 0x4E21_4800 } else { 0x0E21_4800 }) | vn << 5 | vd
}

/// ADR Xd, pc + offset (within 1 MB).
pub fn adr(rd: u32, offset: i64) -> u32 {
    r(&[rd]);
    assert!((-(1 << 20)..1 << 20).contains(&offset));
    0x1000_0000 | field(offset, 0, 3) << 29 | field(offset, 2, 0x7FFFF) << 5 | rd
}

pub fn sqxtn_h(vd: u32, vn: u32, upper: bool) -> u32 {
    r(&[vd, vn]);
    (if upper { 0x4E61_4800 } else { 0x0E61_4800 }) | vn << 5 | vd
}

/// SADDLP or UADDLP Vd, Vn.<T> for size 0-2.
pub fn add_pairwise_long(vd: u32, vn: u32, size: u32, signed: bool) -> u32 {
    r(&[vd, vn]);
    assert!(size < 3);
    (if signed { 0x4E20_2800 } else { 0x6E20_2800 }) | size << 22 | vn << 5 | vd
}

/// SXTL or UXTL Vd.8H, Vn.8B (upper: the high eight bytes).
pub fn extend_bytes(vd: u32, vn: u32, upper: bool, signed: bool) -> u32 {
    r(&[vd, vn]);
    (if signed { 0x0F08_A400 } else { 0x2F08_A400 }) | if upper { 1 << 30 } else { 0 } | vn << 5 | vd
}

/// UMOV Wd, Vn.H[lane].
pub fn umov_h(rd: u32, vn: u32, lane: u32) -> u32 {
    r(&[rd, vn]);
    assert!(lane < 8);
    0x0E00_3C00 | ((lane << 2) | 0b10) << 16 | vn << 5 | rd
}

pub fn smull(vd: u32, vn: u32, vm: u32, upper: bool) -> u32 {
    r(&[vd, vn, vm]);
    (if upper { 0x4E60_C000 } else { 0x0E60_C000 }) | vm << 16 | vn << 5 | vd
}

/// TBL Vd.16B, {Vn.16B}, Vm.16B.
pub fn tbl(vd: u32, vn: u32, vm: u32) -> u32 {
    r(&[vd, vn, vm]);
    0x4E00_0000 | vm << 16 | vn << 5 | vd
}

/// MOVI Vd.16B, #imm8.
pub fn movi_bytes(vd: u32, imm8: u32) -> u32 {
    r(&[vd]);
    0x4F00_E400 | (imm8 >> 5) << 16 | (imm8 & 31) << 5 | vd
}

/// INS Vd.H[lane], Vn.H[src_lane].
pub fn ins_h_element(vd: u32, lane: u32, vn: u32, src_lane: u32) -> u32 {
    r(&[vd, vn]);
    0x6E00_0400 | ((lane << 2) | 0b10) << 16 | (src_lane << 1) << 11 | vn << 5 | vd
}

/// INS Vd.H[lane], Wn.
pub fn ins_h(vd: u32, lane: u32, rn: u32) -> u32 {
    r(&[vd, rn]);
    0x4E00_1C00 | ((lane << 2) | 0b10) << 16 | rn << 5 | vd
}

/// DMB ISHST: earlier stores before later stores (SFENCE).
pub fn dmb_ishst() -> u32 {
    0xD503_3ABF
}

/// DMB ISHLD: earlier loads before later loads and stores (LFENCE).
pub fn dmb_ishld() -> u32 {
    0xD503_39BF
}

/// DMB ISH: every earlier access before every later one (MFENCE).
pub fn dmb_ish() -> u32 {
    0xD503_3BBF
}

/// FMOV Sd, Wn / Dd, Xn.
pub fn fmov_to_v(size: u32, vd: u32, rn: u32) -> u32 {
    r(&[vd, rn]);
    (if size == 4 { 0x1E27_0000 } else { 0x9E67_0000 }) | rn << 5 | vd
}

/// FCMEQ / FCMGE / FCMGT Vd, Vn, Vm on four singles or two doubles.
pub fn fcm(c: &str, double: bool, vd: u32, vn: u32, vm: u32) -> u32 {
    r(&[vd, vn, vm]);
    let base = match c {
        "eq" => 0x4E20_E400,
        "ge" => 0x6E20_E400,
        "gt" => 0x6EA0_E400,
        _ => panic!("fcm {c}"),
    };
    base | u32::from(double) << 22 | vm << 16 | vn << 5 | vd
}

pub fn not_16b(vd: u32, vn: u32) -> u32 {
    r(&[vd, vn]);
    0x6E20_5800 | vn << 5 | vd
}

/// INS Vd.S[lane], Wn.
pub fn ins_s(vd: u32, lane: u32, rn: u32) -> u32 {
    r(&[vd, rn]);
    assert!(lane < 4);
    0x4E00_1C00 | (lane << 3 | 4) << 16 | rn << 5 | vd
}

pub fn uaddlv_8b(vd: u32, vn: u32) -> u32 {
    r(&[vd, vn]);
    0x2E30_3800 | vn << 5 | vd
}

pub fn cnt_8b(vd: u32, vn: u32) -> u32 {
    r(&[vd, vn]);
    0x0E20_5800 | vn << 5 | vd
}

pub fn addv_8b(vd: u32, vn: u32) -> u32 {
    r(&[vd, vn]);
    0x0E31_B800 | vn << 5 | vd
}

/// FMOV Wd, Sn / Xd, Dn.
pub fn fmov_from_v(size: u32, rd: u32, vn: u32) -> u32 {
    r(&[rd, vn]);
    (if size == 4 { 0x1E26_0000 } else { 0x9E66_0000 }) | vn << 5 | rd
}

/// FADD/FSUB/FMUL/FDIV/FMIN/FMAX Sd|Dd, Sn|Dn, Sm|Dm (`_FP2`).
pub fn fp_scalar(op: &str, size: u32, vd: u32, vn: u32, vm: u32) -> u32 {
    r(&[vd, vn, vm]);
    let base = match op {
        "add" => 0x1E20_2800,
        "sub" => 0x1E20_3800,
        "mul" => 0x1E20_0800,
        "div" => 0x1E20_1800,
        "min" => 0x1E20_5800,
        "max" => 0x1E20_4800,
        _ => panic!("fp {op}"),
    };
    base | ftype(size) << 22 | vm << 16 | vn << 5 | vd
}

/// FMADD, FMSUB, FNMADD, FNMSUB (`_FP3`).
pub fn fp_fused(op: &str, size: u32, vd: u32, vn: u32, vm: u32, va: u32) -> u32 {
    r(&[vd, vn, vm, va]);
    let (o1, o0) = match op {
        "fmadd" => (0, 0),
        "fmsub" => (0, 1),
        "fnmadd" => (1, 0),
        "fnmsub" => (1, 1),
        _ => panic!("fused {op}"),
    };
    0x1F00_0000 | ftype(size) << 22 | o1 << 21 | vm << 16 | o0 << 15 | va << 10 | vn << 5 | vd
}

/// FADD/... Vd.4S|2D, Vn, Vm (`_FPV`).
pub fn fp_vector(op: &str, size: u32, vd: u32, vn: u32, vm: u32) -> u32 {
    r(&[vd, vn, vm]);
    let base = match op {
        "add" => 0x4E20_D400,
        "sub" => 0x4EA0_D400,
        "mul" => 0x6E20_DC00,
        "div" => 0x6E20_FC00,
        "min" => 0x4EA0_F400,
        "max" => 0x4E20_F400,
        _ => panic!("fp vector {op}"),
    };
    base | u32::from(size == 8) << 22 | vm << 16 | vn << 5 | vd
}

pub fn fsqrt_scalar(size: u32, vd: u32, vn: u32) -> u32 {
    r(&[vd, vn]);
    0x1E21_C000 | ftype(size) << 22 | vn << 5 | vd
}

pub fn fsqrt_vector(size: u32, vd: u32, vn: u32) -> u32 {
    r(&[vd, vn]);
    0x6EA1_F800 | u32::from(size == 8) << 22 | vn << 5 | vd
}

/// FCMP (quiet) or FCMPE (signals on any NaN).
pub fn fcmp(size: u32, vn: u32, vm: u32, signal: bool) -> u32 {
    r(&[vn, vm]);
    0x1E20_2000 | ftype(size) << 22 | vm << 16 | vn << 5 | if signal { 0x10 } else { 0 }
}

/// FCVTZS Wd|Xd, Sn|Dn.
pub fn fcvtzs(bits: u32, size: u32, rd: u32, vn: u32) -> u32 {
    r(&[rd, vn]);
    0x1E38_0000 | u32::from(bits == 64) << 31 | ftype(size) << 22 | vn << 5 | rd
}

pub fn frintx(size: u32, vd: u32, vn: u32) -> u32 {
    r(&[vd, vn]);
    0x1E27_4000 | ftype(size) << 22 | vn << 5 | vd
}

/// SCVTF Sd|Dd, Wn|Xn.
pub fn scvtf(size: u32, bits: u32, vd: u32, rn: u32) -> u32 {
    r(&[vd, rn]);
    0x1E22_0000 | u32::from(bits == 64) << 31 | ftype(size) << 22 | rn << 5 | vd
}

/// FCVT Dd, Sn or Sd, Dn.
pub fn fcvt(dst_size: u32, src_size: u32, vd: u32, vn: u32) -> u32 {
    r(&[vd, vn]);
    match (dst_size, src_size) {
        (8, 4) => 0x1E22_C000 | vn << 5 | vd,
        (4, 8) => 0x1E62_4000 | vn << 5 | vd,
        _ => panic!("fcvt {dst_size} {src_size}"),
    }
}

/// A system register as (op0, op1, CRn, CRm, op2), Arm ARM D19.
pub type SysReg = (u32, u32, u32, u32, u32);
pub const FPCR: SysReg = (3, 3, 4, 4, 0);
pub const FPSR: SysReg = (3, 3, 4, 4, 1);
pub const ESR_EL1: SysReg = (3, 0, 5, 2, 0);
pub const FAR_EL1: SysReg = (3, 0, 6, 0, 0);
pub const ELR_EL1: SysReg = (3, 0, 4, 0, 1);
pub const SPSR_EL1: SysReg = (3, 0, 4, 0, 0);

fn sysreg((op0, op1, crn, crm, op2): SysReg) -> u32 {
    (op0 - 2) << 19 | op1 << 16 | crn << 12 | crm << 8 | op2 << 5
}

/// MRS Xt, <system register>.
pub fn mrs(rt: u32, reg: SysReg) -> u32 {
    r(&[rt]);
    0xD530_0000 | sysreg(reg) | rt
}

/// MSR <system register>, Xt.
pub fn msr(reg: SysReg, rt: u32) -> u32 {
    r(&[rt]);
    0xD510_0000 | sysreg(reg) | rt
}

pub const ERET: u32 = 0xD69F_03E0;
pub const IC_IALLU: u32 = 0xD508_751F;
/// Stage-1 EL1&0 entries of this VM, this core (Arm ARM C5.5).
pub const TLBI_VMALLE1: u32 = 0xD508_871F;
/// The same on every PE of the Inner Shareable domain.
pub const TLBI_VMALLE1IS: u32 = 0xD508_831F;
/// IC IALLU on every PE of the Inner Shareable domain.
pub const IC_IALLUIS: u32 = 0xD508_711F;
/// casa wzr, w18, [x16]: w18 stored only where [x16] held 0, acquire (Sprint 118).
pub const CASA_WZR_W18_X16: u32 = 0x88FF_7E12;
/// stlr wzr, [x16]: the runtime lock released, with release order.
pub const STLR_WZR_X16: u32 = 0x889F_FE1F;
pub const DSB_ISH: u32 = 0xD503_3B9F;
pub const ISB: u32 = 0xD503_3FDF;

#[cfg(test)]
mod tests {
    use super::*;

    /// `tests/test_a64_encoder.py` `test_mov_imm_halfwords`.
    #[test]
    fn mov_imm_emits_a_movk_only_for_each_nonzero_halfword() {
        assert_eq!(mov_imm(17, 0x7FFE_0000_0010), [movz(17, 0x10, 0), movk(17, 0x7FFE, 2)]);
        assert_eq!(mov_imm(0, 0), [movz(0, 0, 0)]);
        assert_eq!(mov_imm(0, u64::MAX).len(), 4);
    }
}
