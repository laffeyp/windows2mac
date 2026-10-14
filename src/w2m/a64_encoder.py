# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""a64.py — the AArch64 instructions the managed-VM back end emits (Sprint 030).

Each function returns one 32-bit instruction word. Encodings follow the Arm ARM (DDI 0487),
section C6 (A64 base instructions); tests/test_a64.py assembles the same text with clang and
requires equal words, so a transcription error here fails the gates.
Registers are integers 0-30; 31 is XZR or SP as the instruction defines it.
"""

from __future__ import annotations

COND = {"eq": 0, "ne": 1, "cs": 2, "hs": 2, "cc": 3, "lo": 3, "mi": 4, "pl": 5, "vs": 6, "vc": 7,
        "hi": 8, "ls": 9, "ge": 10, "lt": 11, "gt": 12, "le": 13, "al": 14}
ZR = 31


def _r(*regs: int) -> None:
    for r in regs:
        if not 0 <= r <= 31:
            raise ValueError(f"register {r}")


def movz(rd: int, imm16: int, hw: int = 0) -> int:
    _r(rd)
    return 0xD2800000 | hw << 21 | (imm16 & 0xFFFF) << 5 | rd


def movk(rd: int, imm16: int, hw: int) -> int:
    _r(rd)
    return 0xF2800000 | hw << 21 | (imm16 & 0xFFFF) << 5 | rd


def mov_imm(rd: int, value: int) -> list[int]:
    """MOVZ then MOVK for each further nonzero halfword of a 64-bit value."""
    value &= (1 << 64) - 1
    out = [movz(rd, value & 0xFFFF)]
    for hw in range(1, 4):
        h = (value >> (16 * hw)) & 0xFFFF
        if h:
            out.append(movk(rd, h, hw))
    return out


def _sreg(base: int, rd: int, rn: int, rm: int, shift: int = 0, amount: int = 0) -> int:
    _r(rd, rn, rm)
    return base | shift << 22 | rm << 16 | (amount & 63) << 10 | rn << 5 | rd


LSL, LSR = 0, 1


def add(rd: int, rn: int, rm: int, shift: int = LSL, amount: int = 0) -> int:
    return _sreg(0x8B000000, rd, rn, rm, shift, amount)


def adds(rd: int, rn: int, rm: int) -> int:
    return _sreg(0xAB000000, rd, rn, rm)


def sub(rd: int, rn: int, rm: int) -> int:
    return _sreg(0xCB000000, rd, rn, rm)


def subs(rd: int, rn: int, rm: int) -> int:
    return _sreg(0xEB000000, rd, rn, rm)


def and_(rd: int, rn: int, rm: int) -> int:
    return _sreg(0x8A000000, rd, rn, rm)


def bic(rd: int, rn: int, rm: int) -> int:
    return _sreg(0x8A200000, rd, rn, rm)


def orr(rd: int, rn: int, rm: int) -> int:
    return _sreg(0xAA000000, rd, rn, rm)


def eor(rd: int, rn: int, rm: int, shift: int = LSL, amount: int = 0) -> int:
    return _sreg(0xCA000000, rd, rn, rm, shift, amount)


def mov(rd: int, rm: int) -> int:
    return orr(rd, ZR, rm)


def mov32(rd: int, rm: int) -> int:
    """MOV Wd, Wm: writes the low 32 bits and zeroes the upper 32."""
    _r(rd, rm)
    return 0x2A0003E0 | rm << 16 | rd


def addi(rd: int, rn: int, imm12: int) -> int:
    _r(rd, rn)
    assert 0 <= imm12 < 4096
    return 0x91000000 | imm12 << 10 | rn << 5 | rd


def subi(rd: int, rn: int, imm12: int) -> int:
    _r(rd, rn)
    assert 0 <= imm12 < 4096
    return 0xD1000000 | imm12 << 10 | rn << 5 | rd


def cmpi(rn: int, imm12: int) -> int:
    _r(rn)
    assert 0 <= imm12 < 4096
    return 0xF1000000 | imm12 << 10 | rn << 5 | ZR


def cmp(rn: int, rm: int) -> int:
    return subs(ZR, rn, rm)


def ubfm(rd: int, rn: int, immr: int, imms: int) -> int:
    _r(rd, rn)
    return 0xD3400000 | immr << 16 | imms << 10 | rn << 5 | rd


def ubfx(rd: int, rn: int, lsb: int, width: int) -> int:
    assert 0 < width and lsb + width <= 64
    return ubfm(rd, rn, lsb, lsb + width - 1)


def lsri(rd: int, rn: int, shift: int) -> int:
    return ubfm(rd, rn, shift, 63)


def bfi(rd: int, rn: int, lsb: int, width: int) -> int:
    _r(rd, rn)
    assert 0 < width and lsb + width <= 64
    return 0xB3400000 | ((-lsb) % 64) << 16 | (width - 1) << 10 | rn << 5 | rd


def sxtw(rd: int, rn: int) -> int:
    _r(rd, rn)
    return 0x93407C00 | rn << 5 | rd


def udiv(rd: int, rn: int, rm: int) -> int:
    """UDIV Xd, Xn, Xm (Sprint 044, DIV): Xn / Xm unsigned; a zero divisor gives 0, no trap."""
    _r(rd, rn, rm)
    return 0x9AC00800 | rm << 16 | rn << 5 | rd


def ldaxp(rt: int, rt2: int, rn: int) -> int:
    """LDAXP Xt, Xt2, [Xn]: a 16-byte exclusive load-acquire (CMPXCHG16B; Sprint 046)."""
    _r(rt, rt2, rn)
    return 0xC87F8000 | rt2 << 10 | rn << 5 | rt


def stlxp(rs: int, rt: int, rt2: int, rn: int) -> int:
    """STLXP Ws, Xt, Xt2, [Xn]: Ws = 0 when the 16-byte exclusive store-release succeeded."""
    _r(rs, rt, rt2, rn)
    return 0xC8208000 | rs << 16 | rt2 << 10 | rn << 5 | rt


def sdiv(rd: int, rn: int, rm: int) -> int:
    """SDIV Xd, Xn, Xm: signed, toward zero."""
    _r(rd, rn, rm)
    return 0x9AC00C00 | rm << 16 | rn << 5 | rd


def msub(rd: int, rn: int, rm: int, ra: int) -> int:
    """MSUB Xd, Xn, Xm, Xa: Xa - Xn * Xm (the remainder after UDIV)."""
    _r(rd, rn, rm, ra)
    return 0x9B008000 | rm << 16 | ra << 10 | rn << 5 | rd


def rev(rd: int, rn: int, bits: int = 64) -> int:
    """REV Xd, Xn (or REV Wd, Wn for bits 32): byte order reversed (BSWAP)."""
    _r(rd, rn)
    return (0xDAC00C00 if bits == 64 else 0x5AC00800) | rn << 5 | rd


def clz(rd: int, rn: int) -> int:
    """CLZ Xd, Xn: leading zero bits (BSR: bits - 1 - CLZ)."""
    _r(rd, rn)
    return 0xDAC01000 | rn << 5 | rd


def rbit(rd: int, rn: int) -> int:
    """RBIT Xd, Xn: bits reversed (BSF: CLZ of the reversal)."""
    _r(rd, rn)
    return 0xDAC00000 | rn << 5 | rd


def lsrv(rd: int, rn: int, rm: int) -> int:
    _r(rd, rn, rm)
    return 0x9AC02400 | rm << 16 | rn << 5 | rd


def lslv(rd: int, rn: int, rm: int) -> int:
    _r(rd, rn, rm)
    return 0x9AC02000 | rm << 16 | rn << 5 | rd


def asrv(rd: int, rn: int, rm: int) -> int:
    _r(rd, rn, rm)
    return 0x9AC02800 | rm << 16 | rn << 5 | rd


def rorv(rd: int, rn: int, rm: int, bits: int = 64) -> int:
    """ROR Xd, Xn, Xm, or Wd, Wn, Wm for bits=32 (rotates within 32 bits, zeroing the upper half)."""
    _r(rd, rn, rm)
    return (0x9AC02C00 if bits == 64 else 0x1AC02C00) | rm << 16 | rn << 5 | rd


def sbfx(rd: int, rn: int, lsb: int, width: int) -> int:
    """SBFX Xd, Xn, #lsb, #width: sign-extended bit field."""
    _r(rd, rn)
    assert 0 < width and lsb + width <= 64
    return 0x93400000 | lsb << 16 | (lsb + width - 1) << 10 | rn << 5 | rd


def smulh(rd: int, rn: int, rm: int) -> int:
    _r(rd, rn, rm)
    return 0x9B407C00 | rm << 16 | rn << 5 | rd


_SZ = {1: 0, 2: 1, 4: 2, 8: 3}


def ldaxr(size: int, rt: int, rn: int) -> int:
    """LDAXRB/LDAXRH/LDAXR Wt/LDAXR Xt, [Xn]: load-acquire exclusive (natural alignment required)."""
    _r(rt, rn)
    return 0x085FFC00 | _SZ[size] << 30 | rn << 5 | rt


def stlxr(size: int, rs: int, rt: int, rn: int) -> int:
    """STLXRB/STLXRH/STLXR Ws, Wt|Xt, [Xn]: store-release exclusive; Ws = 0 on success."""
    _r(rs, rt, rn)
    return 0x0800FC00 | _SZ[size] << 30 | rs << 16 | rn << 5 | rt


def mul(rd: int, rn: int, rm: int) -> int:
    _r(rd, rn, rm)
    return 0x9B007C00 | rm << 16 | rn << 5 | rd


def umulh(rd: int, rn: int, rm: int) -> int:
    _r(rd, rn, rm)
    return 0x9BC07C00 | rm << 16 | rn << 5 | rd


def cset(rd: int, cond: str) -> int:
    _r(rd)
    return 0x9A9F07E0 | (COND[cond] ^ 1) << 12 | rd


_LD = {1: 0x39400000, 2: 0x79400000, 4: 0xB9400000, 8: 0xF9400000}
_ST = {1: 0x39000000, 2: 0x79000000, 4: 0xB9000000, 8: 0xF9000000}


_LD_REG = {1: 0x38606800, 2: 0x78606800, 4: 0xB8606800, 8: 0xF8606800}   # LDR (register), option LSL
_ST_REG = {1: 0x38206800, 2: 0x78206800, 4: 0xB8206800, 8: 0xF8206800}
_LDUR = {1: 0x38400000, 2: 0x78400000, 4: 0xB8400000, 8: 0xF8400000}
_STUR = {1: 0x38000000, 2: 0x78000000, 4: 0xB8000000, 8: 0xF8000000}


def ldr_reg(size: int, rt: int, rn: int, rm: int, scaled: bool) -> int:
    """LDRB/LDRH/LDR Wt/LDR Xt [Xn, Xm{, LSL #log2(size)}] (checked against Apple clang's assembler: 0xF8627820 for
    LDR X0, [X1, X2, LSL #3]; recentering step 5)."""
    _r(rt, rn, rm)
    return _LD_REG[size] | rm << 16 | (1 << 12 if scaled and size > 1 else 0) | rn << 5 | rt


def str_reg(size: int, rt: int, rn: int, rm: int, scaled: bool) -> int:
    """STRB/STRH/STR Wt/STR Xt [Xn, Xm{, LSL #log2(size)}] (0xF8227820 for STR X0, [X1, X2, LSL #3])."""
    _r(rt, rn, rm)
    return _ST_REG[size] | rm << 16 | (1 << 12 if scaled and size > 1 else 0) | rn << 5 | rt


def ldur(size: int, rt: int, rn: int, offset: int) -> int:
    """LDURB/LDURH/LDUR [Xn, #offset], offset -256..255 unscaled (0xF85F8020 for LDUR X0, [X1, #-8])."""
    _r(rt, rn)
    assert -256 <= offset <= 255
    return _LDUR[size] | (offset & 0x1FF) << 12 | rn << 5 | rt


def stur(size: int, rt: int, rn: int, offset: int) -> int:
    """STURB/STURH/STUR [Xn, #offset], offset -256..255 unscaled (0xF81F8020 for STUR X0, [X1, #-8])."""
    _r(rt, rn)
    assert -256 <= offset <= 255
    return _STUR[size] | (offset & 0x1FF) << 12 | rn << 5 | rt


def ldr(size: int, rt: int, rn: int, offset: int = 0) -> int:
    """LDRB/LDRH/LDR Wt/LDR Xt [Xn, #offset]; zero-extends; offset scaled by size."""
    _r(rt, rn)
    assert offset % size == 0 and 0 <= offset // size < 4096
    return _LD[size] | (offset // size) << 10 | rn << 5 | rt


def str_(size: int, rt: int, rn: int, offset: int = 0) -> int:
    _r(rt, rn)
    assert offset % size == 0 and 0 <= offset // size < 4096
    return _ST[size] | (offset // size) << 10 | rn << 5 | rt


def ldp(rt: int, rt2: int, rn: int, offset: int = 0) -> int:
    _r(rt, rt2, rn)
    assert offset % 8 == 0 and -64 <= offset // 8 < 64
    return 0xA9400000 | ((offset // 8) & 0x7F) << 15 | rt2 << 10 | rn << 5 | rt


def stp(rt: int, rt2: int, rn: int, offset: int = 0) -> int:
    """STP Xt, Xt2, [Xn|SP, #offset] (signed offset form)."""
    _r(rt, rt2, rn)
    assert offset % 8 == 0 and -64 <= offset // 8 < 64
    return 0xA9000000 | ((offset // 8) & 0x7F) << 15 | rt2 << 10 | rn << 5 | rt


def b(offset: int) -> int:
    assert offset % 4 == 0 and -(1 << 27) <= offset < 1 << 27
    return 0x14000000 | (offset >> 2) & 0x3FFFFFF


def bl(offset: int) -> int:
    assert offset % 4 == 0 and -(1 << 27) <= offset < 1 << 27
    return 0x94000000 | (offset >> 2) & 0x3FFFFFF


def ret() -> int:
    return 0xD65F03C0                                  # RET (X30)


def bcond(cond: str, offset: int) -> int:
    assert offset % 4 == 0 and -(1 << 20) <= offset < 1 << 20
    return 0x54000000 | ((offset >> 2) & 0x7FFFF) << 5 | COND[cond]


def cbz(rt: int, offset: int) -> int:
    _r(rt)
    assert offset % 4 == 0 and -(1 << 20) <= offset < 1 << 20
    return 0xB4000000 | ((offset >> 2) & 0x7FFFF) << 5 | rt


def cbnz(rt: int, offset: int) -> int:
    return cbz(rt, offset) | 1 << 24


def tbz(rt: int, bit: int, offset: int) -> int:
    _r(rt)
    assert offset % 4 == 0 and -(1 << 15) <= offset < 1 << 15
    return 0x36000000 | (bit >> 5) << 31 | (bit & 31) << 19 | ((offset >> 2) & 0x3FFF) << 5 | rt


def br(rn: int) -> int:
    _r(rn)
    return 0xD61F0000 | rn << 5


def blr(rn: int) -> int:
    _r(rn)
    return 0xD63F0000 | rn << 5


def hvc(imm16: int) -> int:
    return 0xD4000002 | (imm16 & 0xFFFF) << 5


# ---- SIMD and floating point (Sprint 041) ----------------------------------------------------
# Registers are V0-V31; `size` is 4 (single) or 8 (double); scalar forms act on the low element.

_FTYPE = {4: 0, 8: 1}


def _v(*regs: int) -> None:
    _r(*regs)


def ldr_v(size: int, vt: int, rn: int, offset: int = 0) -> int:
    """LDR St/Dt/Qt, [Xn, #offset] (unsigned offset, scaled); the rest of Vt is zeroed."""
    _v(vt, rn)
    assert offset % size == 0 and 0 <= offset // size < 4096
    base = {4: 0xBD400000, 8: 0xFD400000, 16: 0x3DC00000}[size]
    return base | (offset // size) << 10 | rn << 5 | vt


def str_v(size: int, vt: int, rn: int, offset: int = 0) -> int:
    """STR St/Dt/Qt, [Xn, #offset]."""
    _v(vt, rn)
    assert offset % size == 0 and 0 <= offset // size < 4096
    base = {4: 0xBD000000, 8: 0xFD000000, 16: 0x3D800000}[size]
    return base | (offset // size) << 10 | rn << 5 | vt


def stp_q(vt: int, vt2: int, rn: int, offset: int = 0) -> int:
    """STP Qt, Qt2, [Xn, #offset] (signed offset, scaled by 16)."""
    _v(vt, vt2, rn)
    assert offset % 16 == 0 and -64 <= offset // 16 < 64
    return 0xAD000000 | ((offset // 16) & 0x7F) << 15 | vt2 << 10 | rn << 5 | vt


def ldp_q(vt: int, vt2: int, rn: int, offset: int = 0) -> int:
    _v(vt, vt2, rn)
    assert offset % 16 == 0 and -64 <= offset // 16 < 64
    return 0xAD400000 | ((offset // 16) & 0x7F) << 15 | vt2 << 10 | rn << 5 | vt


def mov_v(vd: int, vn: int) -> int:
    """MOV Vd.16B, Vn.16B (ORR)."""
    _v(vd, vn)
    return 0x4EA01C00 | vn << 16 | vn << 5 | vd


def ins_lane0(size: int, vd: int, vn: int) -> int:
    """INS Vd.S[0], Vn.S[0] or Vd.D[0], Vn.D[0]: the low element; the rest of Vd kept."""
    _v(vd, vn)
    return {4: 0x6E040400, 8: 0x6E080400}[size] | vn << 5 | vd


def eor_v(vd: int, vn: int, vm: int) -> int:
    _v(vd, vn, vm)
    return 0x6E201C00 | vm << 16 | vn << 5 | vd


def and_v(vd: int, vn: int, vm: int) -> int:
    _v(vd, vn, vm)
    return 0x4E201C00 | vm << 16 | vn << 5 | vd


# Sprint 044: the SSE2 integer block (PAND POR PSUB* PSRL* PS*LDQ PUNPCKLBW PCMPEQB PSHUFD PMOVMSKB CVTDQ2PD)
def orr_v(vd: int, vn: int, vm: int) -> int:
    """ORR Vd.16B, Vn.16B, Vm.16B."""
    _v(vd, vn, vm)
    return 0x4EA01C00 | vm << 16 | vn << 5 | vd


def sub_v(vd: int, vn: int, vm: int, size: int) -> int:
    """SUB Vd.<T>, Vn.<T>, Vm.<T>, size 2 for .4S, 3 for .2D."""
    _v(vd, vn, vm)
    return 0x6E208400 | size << 22 | vm << 16 | vn << 5 | vd


def ext_v(vd: int, vn: int, vm: int, index: int) -> int:
    """EXT Vd.16B, Vn.16B, Vm.16B, #index: bytes index..15 of Vn, then 0..index-1 of Vm."""
    _v(vd, vn, vm)
    assert 0 <= index < 16
    return 0x6E000000 | vm << 16 | index << 11 | vn << 5 | vd


def zip1_v(vd: int, vn: int, vm: int) -> int:
    """ZIP1 Vd.16B, Vn.16B, Vm.16B: the low bytes interleaved, Vn's first."""
    _v(vd, vn, vm)
    return 0x4E003800 | vm << 16 | vn << 5 | vd


def cmeq_v(vd: int, vn: int, vm: int) -> int:
    """CMEQ Vd.16B, Vn.16B, Vm.16B: all ones where the bytes are equal."""
    _v(vd, vn, vm)
    return 0x6E208C00 | vm << 16 | vn << 5 | vd


def sxtl_2d(vd: int, vn: int) -> int:
    """SXTL Vd.2D, Vn.2S (SSHLL #0): the low two words sign-extended."""
    _v(vd, vn)
    return 0x0F20A400 | vn << 5 | vd


def scvtf_2d(vd: int, vn: int) -> int:
    """SCVTF Vd.2D, Vn.2D."""
    _v(vd, vn)
    return 0x4E61D800 | vn << 5 | vd


def scvtf_4s(vd: int, vn: int) -> int:
    """SCVTF Vd.4S, Vn.4S (CVTDQ2PS; Sprint 064)."""
    _v(vd, vn)
    return 0x4E21D800 | vn << 5 | vd


def fcvtl_2d(vd: int, vn: int) -> int:
    """FCVTL Vd.2D, Vn.2S: the low two singles widened (CVTPS2PD; Sprint 064)."""
    _v(vd, vn)
    return 0x0E617800 | vn << 5 | vd


def fcvtn_2s(vd: int, vn: int) -> int:
    """FCVTN Vd.2S, Vn.2D: two doubles narrowed into the low half, the high half cleared (CVTPD2PS; Sprint 064)."""
    _v(vd, vn)
    return 0x0E616800 | vn << 5 | vd


def umov_d(rd: int, vn: int, lane: int) -> int:
    """UMOV Xd, Vn.D[lane]."""
    _v(rd, vn)
    return 0x4E003C00 | ((lane << 4) | 0b1000) << 16 | vn << 5 | rd


def ins_d(vd: int, lane: int, rn: int) -> int:
    """INS Vd.D[lane], Xn."""
    _v(vd, rn)
    return 0x4E001C00 | ((lane << 4) | 0b1000) << 16 | rn << 5 | vd


def ins_s_element(vd: int, lane: int, vn: int, src_lane: int) -> int:
    """INS Vd.S[lane], Vn.S[src_lane]."""
    _v(vd, vn)
    return 0x6E000400 | ((lane << 3) | 0b100) << 16 | (src_lane << 2) << 11 | vn << 5 | vd


def movi_zero(vd: int) -> int:
    """MOVI Vd.2D, #0."""
    _v(vd)
    return 0x6F00E400 | vd


def ushr_2d(vd: int, vn: int, shift: int) -> int:
    """USHR Vd.2D, Vn.2D, #shift (1-64)."""
    _v(vd, vn)
    assert 1 <= shift <= 64
    return 0x6F400400 | (128 - shift) << 16 | vn << 5 | vd


# Sprint 046: the second SSE block (SHUFPS PXOR ANDNPS UNPCK*PS MOVHLPS MOVLHPS PUNPCKLDQ PADDD PSRLD PSLLD
# PSRAD MOVMSKPS)
def add_v(vd: int, vn: int, vm: int, size: int) -> int:
    """ADD Vd.<T>, Vn.<T>, Vm.<T>, size 2 for .4S."""
    _v(vd, vn, vm)
    return 0x4E208400 | size << 22 | vm << 16 | vn << 5 | vd


def bic_v(vd: int, vn: int, vm: int) -> int:
    """BIC Vd.16B, Vn.16B, Vm.16B: Vn AND NOT Vm."""
    _v(vd, vn, vm)
    return 0x4E601C00 | vm << 16 | vn << 5 | vd


def zip1_4s(vd: int, vn: int, vm: int) -> int:
    """ZIP1 Vd.4S, Vn.4S, Vm.4S: Vn[0], Vm[0], Vn[1], Vm[1]."""
    _v(vd, vn, vm)
    return 0x4E803800 | vm << 16 | vn << 5 | vd


def zip2_4s(vd: int, vn: int, vm: int) -> int:
    """ZIP2 Vd.4S, Vn.4S, Vm.4S: Vn[2], Vm[2], Vn[3], Vm[3]."""
    _v(vd, vn, vm)
    return 0x4E807800 | vm << 16 | vn << 5 | vd


def ushr_4s(vd: int, vn: int, shift: int) -> int:
    """USHR Vd.4S, Vn.4S, #shift (1-32)."""
    _v(vd, vn)
    assert 1 <= shift <= 32
    return 0x6F000400 | (64 - shift) << 16 | vn << 5 | vd


def sshr_4s(vd: int, vn: int, shift: int) -> int:
    """SSHR Vd.4S, Vn.4S, #shift (1-32)."""
    _v(vd, vn)
    assert 1 <= shift <= 32
    return 0x4F000400 | (64 - shift) << 16 | vn << 5 | vd


def shl_4s(vd: int, vn: int, shift: int) -> int:
    """SHL Vd.4S, Vn.4S, #shift (0-31)."""
    _v(vd, vn)
    assert 0 <= shift <= 31
    return 0x4F005400 | (32 + shift) << 16 | vn << 5 | vd


def dup_4s(vd: int, rn: int) -> int:
    """DUP Vd.4S, Wn."""
    _v(vd, rn)
    return 0x4E040C00 | rn << 5 | vd


def ushl_4s(vd: int, vn: int, vm: int) -> int:
    """USHL Vd.4S, Vn.4S, Vm.4S: each lane shifted by the signed low byte of Vm's lane (negative: right)."""
    _v(vd, vn, vm)
    return 0x6EA04400 | vm << 16 | vn << 5 | vd


def sshl_4s(vd: int, vn: int, vm: int) -> int:
    """SSHL Vd.4S, Vn.4S, Vm.4S: as USHL, right shifts arithmetic."""
    _v(vd, vn, vm)
    return 0x4EA04400 | vm << 16 | vn << 5 | vd


def umov_s(rd: int, vn: int, lane: int) -> int:
    """UMOV Wd, Vn.S[lane]."""
    _v(rd, vn)
    return 0x0E003C00 | ((lane << 3) | 0b100) << 16 | vn << 5 | rd


# Sprint 046: the third SSE block. NEON three-same instructions on a whole register (Q=1), by lane size
# (0 bytes, 1 halfwords, 2 words, 3 doublewords).
THREE_SAME = {"add": 0x4E208400, "sub": 0x6E208400, "cmeq": 0x6E208C00, "cmgt": 0x4E203400, "uqsub": 0x6E202C00,
              "sqsub": 0x4E202C00, "sqadd": 0x4E200C00, "umax": 0x6E206400, "mul": 0x4E209C00,
              "zip1": 0x4E003800, "zip2": 0x4E007800, "ushl": 0x6E204400, "sshl": 0x4E204400,
              "uqadd": 0x6E200C00, "uzp2": 0x4E005800,                      # Sprint 065
              "uabd": 0x6E207400}                                           # Sprint 068 (PSADBW)


def three_same(name: str, vd: int, vn: int, vm: int, size: int) -> int:
    """<name> Vd.<T>, Vn.<T>, Vm.<T> for T = 16B, 8H, 4S, 2D by size 0-3."""
    _v(vd, vn, vm)
    return THREE_SAME[name] | size << 22 | vm << 16 | vn << 5 | vd


def three_same_d(name: str, vd: int, vn: int, vm: int, size: int) -> int:
    """<name> on the low 64 bits (Q=0): T = 8B, 4H, 2S by size 0-2 (Sprint 065, the MMX registers)."""
    assert size < 3
    return three_same(name, vd, vn, vm, size) & ~(1 << 30)


def shift_right(vd: int, vn: int, shift: int, size: int, signed: bool) -> int:
    """USHR or SSHR Vd.<T>, Vn.<T>, #shift (1 to the lane's bits) for size 1-3."""
    _v(vd, vn)
    bits = 8 << size
    assert 1 <= shift <= bits and size >= 1
    return (0x4F000400 if signed else 0x6F000400) | (2 * bits - shift) << 16 | vn << 5 | vd


def shift_left(vd: int, vn: int, shift: int, size: int) -> int:
    """SHL Vd.<T>, Vn.<T>, #shift (0 to the lane's bits - 1) for size 1-3."""
    _v(vd, vn)
    bits = 8 << size
    assert 0 <= shift < bits and size >= 1
    return 0x4F005400 | (bits + shift) << 16 | vn << 5 | vd


def dup_gpr(vd: int, rn: int, size: int) -> int:
    """DUP Vd.<T>, Wn (sizes 0-2) or Xn (size 3)."""
    _v(vd, rn)
    return 0x4E000C00 | (1 << size) << 16 | rn << 5 | vd


def sqxtun(vd: int, vn: int, upper: bool) -> int:
    """SQXTUN Vd.8B, Vn.8H (upper: SQXTUN2 Vd.16B): halfwords to unsigned bytes, saturated."""
    _v(vd, vn)
    return (0x6E212800 if upper else 0x2E212800) | vn << 5 | vd


def sqxtn(vd: int, vn: int, upper: bool) -> int:
    """SQXTN Vd.8B, Vn.8H (upper: SQXTN2 Vd.16B): halfwords to signed bytes, saturated (Sprint 065)."""
    _v(vd, vn)
    return (0x4E214800 if upper else 0x0E214800) | vn << 5 | vd


def adr(rd: int, offset: int) -> int:
    """ADR Xd, pc + offset (within 1 MB; checked against Apple clang's assembler: 0x10000070 for x16 at +12)."""
    _r(rd)
    assert -(1 << 20) <= offset < (1 << 20)
    return 0x10000000 | (offset & 3) << 29 | ((offset >> 2) & 0x7FFFF) << 5 | rd


def sqxtn_h(vd: int, vn: int, upper: bool) -> int:
    """SQXTN Vd.4H, Vn.4S (upper: SQXTN2 Vd.8H): words to signed halfwords, saturated (Sprint 068; encodings checked
    against Apple clang's assembler, as are the four below)."""
    _v(vd, vn)
    return (0x4E614800 if upper else 0x0E614800) | vn << 5 | vd


def add_pairwise_long(vd: int, vn: int, size: int, signed: bool) -> int:
    """SADDLP or UADDLP Vd, Vn.<T>: adjacent lanes of size 0-2 (16B, 8H, 4S) added into lanes twice as wide."""
    _v(vd, vn)
    assert size < 3
    return (0x4E202800 if signed else 0x6E202800) | size << 22 | vn << 5 | vd


def extend_bytes(vd: int, vn: int, upper: bool, signed: bool) -> int:
    """SXTL or UXTL Vd.8H, Vn.8B (SSHLL/USHLL #0; upper: the high eight bytes, SSHLL2/USHLL2)."""
    _v(vd, vn)
    return (0x0F08A400 if signed else 0x2F08A400) | (1 << 30 if upper else 0) | vn << 5 | vd


def umov_h(rd: int, vn: int, lane: int) -> int:
    """UMOV Wd, Vn.H[lane]: the halfword zero-extended."""
    _v(rd, vn)
    assert 0 <= lane < 8
    return 0x0E003C00 | ((lane << 2) | 0b10) << 16 | vn << 5 | rd


def smull(vd: int, vn: int, vm: int, upper: bool) -> int:
    """SMULL Vd.4S, Vn.4H, Vm.4H (upper: SMULL2 on the high halfwords): signed products, widened (Sprint 065)."""
    _v(vd, vn, vm)
    return (0x4E60C000 if upper else 0x0E60C000) | vm << 16 | vn << 5 | vd


def tbl(vd: int, vn: int, vm: int) -> int:
    """TBL Vd.16B, {Vn.16B}, Vm.16B: an index of 16 or more gives 0 (Sprint 065)."""
    _v(vd, vn, vm)
    return 0x4E000000 | vm << 16 | vn << 5 | vd


def movi_bytes(vd: int, imm8: int) -> int:
    """MOVI Vd.16B, #imm8 (Sprint 065)."""
    _v(vd)
    return 0x4F00E400 | (imm8 >> 5) << 16 | (imm8 & 31) << 5 | vd


def ins_h_element(vd: int, lane: int, vn: int, src_lane: int) -> int:
    """INS Vd.H[lane], Vn.H[src_lane]."""
    _v(vd, vn)
    return 0x6E000400 | ((lane << 2) | 0b10) << 16 | (src_lane << 1) << 11 | vn << 5 | vd


def ins_h(vd: int, lane: int, rn: int) -> int:
    """INS Vd.H[lane], Wn."""
    _v(vd, rn)
    return 0x4E001C00 | ((lane << 2) | 0b10) << 16 | rn << 5 | vd


def dmb_ishst() -> int:
    """DMB ISHST: earlier stores before later stores (SFENCE)."""
    return 0xD5033ABF


def dmb_ishld() -> int:
    """DMB ISHLD: earlier loads before later loads and stores (LFENCE)."""
    return 0xD50339BF


def dmb_ish() -> int:
    """DMB ISH: every earlier access before every later one (MFENCE)."""
    return 0xD5033BBF


def fmov_to_v(size: int, vd: int, rn: int) -> int:
    """FMOV Sd, Wn / Dd, Xn: the rest of Vd zeroed."""
    _v(vd, rn)
    return {4: 0x1E270000, 8: 0x9E670000}[size] | rn << 5 | vd


def fcm(cond: str, double: bool, vd: int, vn: int, vm: int) -> int:
    """FCMEQ / FCMGE / FCMGT Vd, Vn, Vm on four singles or two doubles: all ones in each lane where it holds."""
    _v(vd, vn, vm)
    return {"eq": 0x4E20E400, "ge": 0x6E20E400, "gt": 0x6EA0E400}[cond] | int(double) << 22 | vm << 16 | vn << 5 | vd


def not_16b(vd: int, vn: int) -> int:
    """NOT Vd.16B, Vn.16B."""
    _v(vd, vn)
    return 0x6E205800 | vn << 5 | vd


def ins_s(vd: int, lane: int, rn: int) -> int:
    """INS Vd.S[lane], Wn (MOV): one 32-bit lane from a general register, the others kept."""
    _v(vd, rn)
    assert 0 <= lane < 4
    return 0x4E001C00 | (lane << 3 | 4) << 16 | rn << 5 | vd


def uaddlv_8b(vd: int, vn: int) -> int:
    """UADDLV Hd, Vn.8B: the sum of the eight bytes as a halfword, the rest of Vd zeroed."""
    _v(vd, vn)
    return 0x2E303800 | vn << 5 | vd


def cnt_8b(vd: int, vn: int) -> int:
    """CNT Vd.8B, Vn.8B: the set bits of each byte."""
    _v(vd, vn)
    return 0x0E205800 | vn << 5 | vd


def addv_8b(vd: int, vn: int) -> int:
    """ADDV Bd, Vn.8B: the sum of the eight bytes."""
    _v(vd, vn)
    return 0x0E31B800 | vn << 5 | vd


def fmov_from_v(size: int, rd: int, vn: int) -> int:
    """FMOV Wd, Sn / Xd, Dn."""
    _v(rd, vn)
    return {4: 0x1E260000, 8: 0x9E660000}[size] | vn << 5 | rd


_FP2 = {"add": 0x1E202800, "sub": 0x1E203800, "mul": 0x1E200800, "div": 0x1E201800,
        "min": 0x1E205800, "max": 0x1E204800}


def fp_scalar(op: str, size: int, vd: int, vn: int, vm: int) -> int:
    """FADD/FSUB/FMUL/FDIV/FMIN/FMAX Sd|Dd, Sn|Dn, Sm|Dm."""
    _v(vd, vn, vm)
    return _FP2[op] | _FTYPE[size] << 22 | vm << 16 | vn << 5 | vd


_FP3 = {"fmadd": (0, 0), "fmsub": (0, 1), "fnmadd": (1, 0), "fnmsub": (1, 1)}


def fp_fused(op: str, size: int, vd: int, vn: int, vm: int, va: int) -> int:
    """FMADD Dd = Da + Dn*Dm; FMSUB Da - Dn*Dm; FNMADD -Da - Dn*Dm; FNMSUB -Da + Dn*Dm (one rounding; Arm ARM
    C7.2 FP data-processing, 3 source; Sprint 049)."""
    _v(vd, vn, vm, va)
    o1, o0 = _FP3[op]
    return 0x1F000000 | _FTYPE[size] << 22 | o1 << 21 | vm << 16 | o0 << 15 | va << 10 | vn << 5 | vd


_FPV = {"add": 0x4E20D400, "sub": 0x4EA0D400, "mul": 0x6E20DC00, "div": 0x6E20FC00,
        "min": 0x4EA0F400, "max": 0x4E20F400}


def fp_vector(op: str, size: int, vd: int, vn: int, vm: int) -> int:
    """FADD/... Vd.4S|2D, Vn, Vm."""
    _v(vd, vn, vm)
    return _FPV[op] | (size == 8) << 22 | vm << 16 | vn << 5 | vd


def fsqrt_scalar(size: int, vd: int, vn: int) -> int:
    _v(vd, vn)
    return 0x1E21C000 | _FTYPE[size] << 22 | vn << 5 | vd


def fsqrt_vector(size: int, vd: int, vn: int) -> int:
    _v(vd, vn)
    return 0x6EA1F800 | (size == 8) << 22 | vn << 5 | vd


def fcmp(size: int, vn: int, vm: int, signal: bool = False) -> int:
    """FCMP (quiet) or FCMPE (signals on any NaN) Sn|Dn, Sm|Dm."""
    _v(vn, vm)
    return 0x1E202000 | _FTYPE[size] << 22 | vm << 16 | vn << 5 | (0x10 if signal else 0)


def fcvtzs(bits: int, size: int, rd: int, vn: int) -> int:
    """FCVTZS Wd|Xd, Sn|Dn (toward zero; saturates; NaN gives 0)."""
    _v(rd, vn)
    return 0x1E380000 | (bits == 64) << 31 | _FTYPE[size] << 22 | vn << 5 | rd


def frintx(size: int, vd: int, vn: int) -> int:
    """FRINTX Sd|Dd, Sn|Dn: round to integral in FPCR's mode, inexact signalled."""
    _v(vd, vn)
    return 0x1E274000 | _FTYPE[size] << 22 | vn << 5 | vd


def scvtf(size: int, bits: int, vd: int, rn: int) -> int:
    """SCVTF Sd|Dd, Wn|Xn (FPCR's rounding mode)."""
    _v(vd, rn)
    return 0x1E220000 | (bits == 64) << 31 | _FTYPE[size] << 22 | rn << 5 | vd


def fcvt(dst_size: int, src_size: int, vd: int, vn: int) -> int:
    """FCVT Dd, Sn or Sd, Dn."""
    _v(vd, vn)
    if (dst_size, src_size) == (8, 4):
        return 0x1E22C000 | vn << 5 | vd
    if (dst_size, src_size) == (4, 8):
        return 0x1E624000 | vn << 5 | vd
    raise ValueError((dst_size, src_size))


FPCR = (3, 3, 4, 4, 0)
FPSR = (3, 3, 4, 4, 1)


# System registers as (op0, op1, CRn, CRm, op2), Arm ARM D19 (AArch64 system register encodings).
ESR_EL1 = (3, 0, 5, 2, 0)
FAR_EL1 = (3, 0, 6, 0, 0)
ELR_EL1 = (3, 0, 4, 0, 1)
SPSR_EL1 = (3, 0, 4, 0, 0)


def _sysreg(reg: tuple[int, int, int, int, int]) -> int:
    op0, op1, crn, crm, op2 = reg
    return (op0 - 2) << 19 | op1 << 16 | crn << 12 | crm << 8 | op2 << 5


def mrs(rt: int, reg: tuple[int, int, int, int, int]) -> int:
    """MRS Xt, <system register>."""
    _r(rt)
    return 0xD5300000 | _sysreg(reg) | rt


def msr(reg: tuple[int, int, int, int, int], rt: int) -> int:
    """MSR <system register>, Xt."""
    _r(rt)
    return 0xD5100000 | _sysreg(reg) | rt


ERET = 0xD69F03E0
IC_IALLU = 0xD508751F
TLBI_VMALLE1 = 0xD508871F           # stage-1 EL1&0 entries of this VM, this core (Arm ARM C5.5)
DSB_ISH = 0xD5033B9F
ISB = 0xD5033FDF
