# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The MMX-register forms in A64 (Sprint 065), as src/w2m/mmx.py computes them: frontier
643a6283510c-1791810726153034000 ran 18.0M MMX instructions as model steps (hvc #11), 11.6M of them MOVQ.

MMn is bits 63:0 of physical register Rn; the state block keeps ST0-ST7 in stack order, so slot n holds Rn when TOP
is 0. The code takes the native path only then, and only with no x87 exception pending (FSW.ES): any other state, and
every form below that names an XMM register, is a model step. Sprint 068 item C adds MOVD and MOVQ to and from a
general register, PEXTRW into one, and PSADBW (a 10-billion-instruction frontier with worker threads on time ran them
63,188, 59,860 and 60,800 times as model steps), then PMADDUBSW and EMMS (63,480 and 10,128 in the full batch run). On the native path TOP stays 0; a write to
MMn sets bits 79:64 to ones, and every tag is set valid after the write (a faulting load or store changes nothing,
as set 065b has it). x16 holds the state block, x17 is scratch, V30 and V31 hold the operands."""

from __future__ import annotations

from collections.abc import Callable

from w2m import a64_encoder as a64
from w2m import vm_backend as vb
from w2m.lift import Op

STATE = vb.PROCESS_VA + vb.STATE_OFF
TAGS = vb.X87_SW_OFF + 2                     # x87_tag_abridged (specs/guest_state.json)

# Lane operations on 64 bits: (NEON three-same name, lane size)
LANES = {"PADDW": ("add", 1), "PADDD": ("add", 2), "PSUBW": ("sub", 1), "PMULLW": ("mul", 1), "PADDSW": ("sqadd", 1),
         "PUNPCKLBW": ("zip1", 0), "PUNPCKLWD": ("zip1", 1), "PUNPCKLDQ": ("zip1", 2),
         "PUNPCKHBW": ("zip2", 0), "PUNPCKHWD": ("zip2", 1), "PUNPCKHDQ": ("zip2", 2)}
SOURCE_FORMS = set(LANES) | {"PXOR", "PMULHW", "PACKUSWB", "PSHUFW"}
SHIFTS = {"PSRAW": 1, "PSRAD": 2}
SOURCE_FORMS |= {"PSADBW", "PMADDUBSW"}


def _slot(n: int) -> int:
    return vb.X87_ST_OFF + 16 * n


def native(op: Op) -> bool:
    """Whether the form runs in A64: MMX-register and memory operands only."""
    ic: str = op[1]
    d: str = op[2][0]
    s: str = op[3][0]
    if ic in ("MOVQ", "MOVD"):
        return (d == "mm" and s in ("mm", "m", "r")) or (d in ("m", "r") and s == "mm")
    if ic == "PEXTRW":
        return d == "r" and s == "mm"
    if ic == "EMMS":
        return True
    if ic in SHIFTS:
        return d == "mm" and s == "i"
    if ic in SOURCE_FORMS:
        return d == "mm" and s in ("mm", "m")
    return False


def lower(a: vb.Assembler, op: Op, reg_of: Callable[[int], int]) -> None:
    _, ic, dst, src, imm = op
    if not native(op):
        a.emit(a64.hvc(vb.HVC_SLOW_STEP))
        return
    fast = a.label()
    a.emit(a64.mov_imm(16, STATE), a64.ldr(2, 17, 16, vb.X87_SW_OFF),
           a64.ubfx(17, 17, 7, 7),                          # bit 0: ES; bits 6:4: TOP
           a64.bfi(17, a64.ZR, 1, 3))                       # C0-C2 do not matter
    a.branch(lambda off: a64.cbz(17, off), fast)
    a.emit(a64.hvc(vb.HVC_SLOW_STEP))
    a.bind(fast)
    if ic == "EMMS":                                          # every tag empty; TOP as it is (SDM EMMS)
        a.emit(a64.str_(1, a64.ZR, 16, TAGS))
        return

    def get(o: Op, v: int) -> None:
        if o[0] == "mm":
            a.emit(a64.ldr_v(8, v, 16, _slot(o[1])))
        elif o[0] == "r":                                     # MOVD r32 (zero-extended), MOVQ r64
            a.emit(a64.fmov_to_v(o[2] // 8, v, o[1]))
        else:
            a.emit(a64.ldr_v(o[2], v, reg_of(o[1])))       # 4 bytes (MOVD, a low unpack) zero the rest

    if ic in ("MOVQ", "MOVD", "PEXTRW"):
        get(src, 30)
    else:
        get(dst, 30)
        if ic in LANES:
            name, size = LANES[ic]
            get(src, 31)
            a.emit(a64.three_same_d(name, 30, 30, 31, size))
        elif ic == "PXOR":
            get(src, 31)
            a.emit(a64.eor_v(30, 30, 31))
        elif ic == "PMULHW":                                  # the four products, then their high halves
            get(src, 31)
            a.emit(a64.smull(30, 30, 31, False), a64.three_same("uzp2", 30, 30, 30, 1))
        elif ic == "PACKUSWB":                                # the destination's words, then the source's
            get(src, 31)
            a.emit(a64.three_same("zip1", 30, 30, 31, 3), a64.sqxtun(30, 30, False))
        elif ic == "PSHUFW":
            get(src, 31)
            for lane in range(4):
                a.emit(a64.ins_h_element(30, lane, 31, imm >> (2 * lane) & 3))
        elif ic in SHIFTS:                                    # past the lane's bits, the sign
            size = SHIFTS[ic]
            n = min(imm, (8 << size) - 1)
            if n:
                a.emit(a64.shift_right(30, 30, n, size, True))
        elif ic == "PMADDUBSW":                               # as the XMM form on eight bytes: four saturated words
            get(src, 31)
            a.emit(a64.extend_bytes(29, 31, False, True), a64.extend_bytes(28, 30, False, False),
                   a64.three_same("mul", 28, 28, 29, 1), a64.add_pairwise_long(28, 28, 1, True), a64.sqxtn_h(30, 28, False))
        elif ic == "PSADBW":                                  # the absolute differences' sum, bits 63:16 zero
            get(src, 31)
            a.emit(a64.three_same_d("uabd", 30, 30, 31, 0), a64.uaddlv_8b(30, 30))
    if ic == "PEXTRW":                                        # word imm & 3, zero-extended into the register
        a.emit(a64.umov_h(dst[1], 30, imm & 3))
    elif dst[0] == "r":                                       # MOVD r32 (zero-extended), MOVQ r64
        a.emit(a64.fmov_from_v(dst[2] // 8, dst[1], 30))
    elif dst[0] == "mm":
        a.emit(a64.str_v(8, 30, 16, _slot(dst[1])), a64.movz(17, 0xFFFF), a64.str_(2, 17, 16, _slot(dst[1]) + 8))
    else:
        a.emit(a64.str_v(dst[2], 30, reg_of(dst[1])))
    a.emit(a64.movz(17, 0xFF), a64.str_(1, 17, 16, TAGS))
