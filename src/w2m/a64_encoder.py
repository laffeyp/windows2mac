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


def lsrv(rd: int, rn: int, rm: int) -> int:
    _r(rd, rn, rm)
    return 0x9AC02400 | rm << 16 | rn << 5 | rd


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


IC_IALLU = 0xD508751F
DSB_ISH = 0xD5033B9F
ISB = 0xD5033FDF
