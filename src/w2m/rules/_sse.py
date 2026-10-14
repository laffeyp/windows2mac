# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Shared SSE rule and AArch64 lowering (moved unchanged from lift.py and vm_backend.py in Sprint
041). XMM i is V i; V30 and V31 are scratch; FPCR follows MXCSR (vm_backend.fpcr_for). An
instruction that can raise an SIMD exception leaves the fast path when MXCSR unmasks any."""

from __future__ import annotations

from collections.abc import Callable
from typing import Any, cast

from w2m import vm_backend as vb
from w2m.lift import SSE_050, SSE_ALIGNED, Op, _operand
from w2m.rules import Ctx
from w2m.vm_backend import (AF_OPERANDS, FP_CLASSES, HVC_SLOW_SSE, MXCSR_OFF, REG_CF, REG_OF, REG_PF, REG_SF,
                            REG_ZF, U_AF, U_CF, U_OF, U_PF, U_SF, U_ZF, Assembler, CompileError, _mark_defined)
from w2m import a64_encoder as a64


native_conversions = True        # Sprint 064's lowered conversions; False sends them to the model as before


def _unmasked_to_model(a: Assembler) -> None:
    """The model runs the instruction when MXCSR unmasks any exception: only the model raises the SIMD fault."""
    fast = a.label()
    a.emit(a64.ldr(4, 16, 28, MXCSR_OFF), a64.ubfx(16, 16, 7, 6), a64.cmpi(16, 0x3F))
    a.branch(lambda off: a64.bcond("eq", off), fast)
    a.emit(a64.hvc(HVC_SLOW_SSE))
    a.bind(fast)


def rule(ctx: Ctx) -> None:
    """One legacy-SSE instruction as one `sse` operation; the runtime (x86fp) and the lowering below
    compute it (lift.py's SSE rule, Sprint 039)."""
    b, ic, ops, nxt = ctx.b, ctx.ic, ctx.ops, ctx.nxt
    dst = cast(tuple[Any, ...], _operand(b, ops[0], nxt)[1])
    src = cast(tuple[Any, ...], _operand(b, ops[1], nxt)[1])
    has_memory = dst[0] == "m" or src[0] == "m"
    b.emit("sse", ic, dst, src, 16 if has_memory and ic in SSE_ALIGNED else 0)


def int_rule(ctx: Ctx) -> None:
    """The SSE2 integer block (Sprint 044): as `rule`, with an immediate when the template has one
    (PSHUFD V,W,Ib; the shift-by-immediate forms W,Ib, whose source is the destination itself)."""
    b, ic, ops, nxt = ctx.b, ctx.ic, ctx.ops, ctx.nxt
    imm = int(ops[-1].split(":")[2], 16) if ops[-1].startswith("I:") else None
    dst = cast(tuple[Any, ...], _operand(b, ops[0], nxt)[1])
    src = dst if len(ops) == 2 and imm is not None else cast(tuple[Any, ...], _operand(b, ops[1], nxt)[1])
    has_memory = dst[0] == "m" or src[0] == "m"
    align = 16 if has_memory and ic in SSE_ALIGNED else 0
    b.emit("sse", ic, dst, src, align) if imm is None else b.emit("sse", ic, dst, src, align, imm)


def memory_only_rule(ctx: Ctx) -> None:
    """MOVLPS, MOVHPD, MOVNTDQ (Sprint 050): the SDM lists memory forms only; a register operand is #UD."""
    if any(o.startswith("R:XMM") for o in ctx.ops) and all(not o.startswith("M:") for o in ctx.ops):
        ctx.b.emit("ud")
        return
    int_rule(ctx)


def register_only_rule(ctx: Ctx) -> None:
    """PEXTRW 66 0F C5 (Sprint 050): its source is an XMM register only (SDM: 0F C5 /r with mod 11); memory is #UD."""
    if any(o.startswith("M:") for o in ctx.ops):
        ctx.b.emit("ud")
        return
    int_rule(ctx)


def _sse_src(a: Assembler, loc: Op, size: int, reg_of: Callable[[int], int], scratch: int = 31) -> int:
    """The V register holding a source operand: an XMM register, or `scratch` loaded from memory."""
    if loc[0] == "x":
        return int(loc[1])
    a.emit(a64.ldr_v(size, scratch, reg_of(loc[1])))
    return scratch


def _check_alignment(a: Assembler, rt: int) -> None:
    """#GP on a misaligned 16-byte operand: Windows reports it as an access violation at the
    instruction. A load from address 0 (never mapped) raises it through the same exception path,
    before the instruction changes anything."""
    ok = a.label()
    a.emit(a64.ubfx(16, rt, 0, 4))
    a.branch(lambda off: a64.cbz(16, off), ok)
    a.emit(a64.movz(16, 0), a64.ldr(8, 16, 16, 0))
    a.bind(ok)


def _set_flags_from_fcmp(a: Assembler) -> None:
    """(U)COMIS[SD]: FCMP gives NZCV 0011 unordered, 1000 less, 0110 equal, 0010 greater; x86 wants
    ZF PF CF 111, 001, 100, 000 and OF SF AF cleared (SDM COMISS)."""
    # PF is unordered, kept lazily (vm_backend.PF_RESULT: 0 for PF set, 1 for clear, so V clear stores 1)
    a.emit(a64.cset(REG_PF, "vs"), a64.cset(REG_CF, "lt"), a64.cset(REG_ZF, "eq"),
           a64.orr(REG_ZF, REG_ZF, REG_PF), a64.mov(REG_OF, a64.ZR), a64.mov(REG_SF, a64.ZR),
           a64.cset(16, "vc"), a64.stp(a64.ZR, a64.ZR, 28, AF_OPERANDS), a64.str_(8, 16, 28, AF_OPERANDS + 16))
    _mark_defined(a, U_CF | U_ZF | U_SF | U_OF | U_PF | U_AF)


SSE2_INT_LOWERED = {"MOVDQA", "MOVDQU", "MOVNTPS", "PAND", "POR", "ORPS", "PSUBD", "PSUBQ", "PSRLQ", "PSRLDQ",
                    "PSLLDQ", "PUNPCKLBW", "PCMPEQB", "PSHUFD", "PMOVMSKB", "CVTDQ2PD"}
# Sprint 046 (third block): one NEON three-same instruction each, d = op(d, source): (name, lane size)
LANE_LOWERED = {"PUNPCKLQDQ": ("zip1", 3), "UNPCKLPD": ("zip1", 3), "UNPCKHPD": ("zip2", 3), "PADDQ": ("add", 3), "PADDW": ("add", 1),
                "PSUBW": ("sub", 1), "PCMPEQD": ("cmeq", 2), "PCMPGTD": ("cmgt", 2), "PSUBUSB": ("uqsub", 0),
                "PSUBSB": ("sqsub", 0), "PADDSB": ("sqadd", 0), "PADDSW": ("sqadd", 1), "PMAXUB": ("umax", 0),
                "PMULLW": ("mul", 1), "PUNPCKHBW": ("zip2", 0), "PUNPCKHDQ": ("zip2", 2), "PUNPCKHWD": ("zip2", 1),
                "PUNPCKLWD": ("zip1", 1), "PCMPEQW": ("cmeq", 1),
                "PUNPCKHQDQ": ("zip2", 3),      # Sprint 065: 1.4M model steps at UnityPlayer.dll+0x3cf4 and its neighbours
                "PADDUSB": ("uqadd", 0), "PCMPGTB": ("cmgt", 0), "PADDB": ("add", 0)}     # Sprint 065
SSE2_INT_LOWERED_APPROXIMATE = {"RCPPS", "RSQRTPS", "RCPSS", "RSQRTSS"}
SHIFTS = {"PSRLD": (2, "right"), "PSLLD": (2, "left"), "PSRAD": (2, "arith"), "PSRLW": (1, "right"),
          "PSRAW": (1, "arith"), "PSLLQ": (3, "left")}
SSE2_INT_LOWERED |= {'PADDSB', 'PACKUSWB', 'PUNPCKHDQ', 'PMAXUB', 'PSUBSB', 'PSRAW', 'PADDQ', 'PCMPEQD', 'PSHUFLW', 'PSUBUSB', 'PSRLW', 'MOVHPS', 'PUNPCKHBW', 'PUNPCKHWD', 'PUNPCKLWD', 'PANDN', 'PADDW', 'PADDSW', 'PSUBW', 'PUNPCKLQDQ', 'PSHUFHW', 'PMULLW', 'PINSRW', 'UNPCKLPD', 'PCMPGTD', 'PSLLQ'}
SSE2_INT_LOWERED |= SSE2_INT_LOWERED_APPROXIMATE
SSE2_INT_LOWERED |= {"PCMPISTRI", "PCMPISTRI64", "UNPCKHPD"}                    # Sprint 047
SSE2_INT_LOWERED |= {"PCMPEQW"}                                                  # Sprint 049
SSE2_INT_LOWERED |= {"PUNPCKHQDQ"}                                               # Sprint 065
# Sprint 065: the frontier's hottest slow-SSE classes (frontier 643a6283510c-1791810726153034000: 39.5M exits)
SSE2_INT_LOWERED |= {"PADDUSB", "PCMPGTB", "PADDB", "PACKSSWB", "PMULHW", "PSHUFB", "MOVLPS"}
SSE2_INT_LOWERED |= {"PSADBW", "PMADDUBSW"}                                     # Sprint 068
SSE2_INT_LOWERED |= {'MOVMSKPS', 'PADDD', 'MOVLHPS', 'PSRAD', 'PSRLD', 'PSLLD', 'UNPCKHPS', 'ANDNPS', 'MOVHLPS', 'PXOR', 'PUNPCKLDQ', 'SHUFPS', 'UNPCKLPS'}     # Sprint 046


def _lower_int(a: Assembler, ic: str, dst: Op, src: Op, imm: int | None, reg_of: Callable[[int], int]) -> None:
    """The SSE2 integer block in NEON (Sprint 044). V31 and V30 are scratch; x16 and x17 too."""
    if ic in ("MOVDQA", "MOVDQU", "MOVNTPS"):
        if dst[0] == "x" and src[0] == "x":
            a.emit(a64.mov_v(dst[1], src[1]))
        elif dst[0] == "x":
            a.emit(a64.ldr_v(16, dst[1], reg_of(src[1])))
        else:
            a.emit(a64.str_v(16, src[1], reg_of(dst[1])))
        return
    if ic == "PMOVMSKB":                                         # the 16 sign bits into a general register
        v, rd = int(src[1]), int(dst[1])
        for lane, shift in ((1, 8), (0, 0)):
            a.emit(a64.umov_d(16, v, lane), a64.mov_imm(17, 0x8080_8080_8080_8080), a64.and_(16, 16, 17),
                   a64.lsri(16, 16, 7), a64.mov_imm(17, 0x0102_0408_1020_4080), a64.mul(16, 16, 17),
                   a64.lsri(16, 16, 56))
            if shift:
                a.emit(a64.eor(rd, a64.ZR, 16, a64.LSL, 8))
            else:
                a.emit(a64.orr(rd, rd, 16))
        return
    d = int(dst[1])
    if ic == "CVTDQ2PD":                                         # two signed words to doubles, exactly
        v = _sse_src(a, src, 8, reg_of)
        a.emit(a64.sxtl_2d(30, v), a64.scvtf_2d(d, 30))
        return
    if ic in ("PSRLDQ", "PSLLDQ"):                               # whole-register byte shifts by imm8
        assert imm is not None
        if imm > 15:
            a.emit(a64.movi_zero(d))
        elif imm:
            a.emit(a64.movi_zero(31))
            a.emit(a64.ext_v(d, d, 31, imm) if ic == "PSRLDQ" else a64.ext_v(d, 31, d, 16 - imm))
        return
    if ic == "PSRLQ" and imm is not None:
        if imm > 63:
            a.emit(a64.movi_zero(d))
        elif imm:
            a.emit(a64.ushr_2d(d, d, imm))
        return
    if ic == "PSRLQ":                                            # by a count in the source's low 64 bits
        v = _sse_src(a, src, 16, reg_of)
        big, done = a.label(), a.label()
        a.emit(a64.umov_d(16, v, 0), a64.cmpi(16, 63))
        a.branch(lambda off: a64.bcond("hi", off), big)
        for lane in (0, 1):
            a.emit(a64.umov_d(17, d, lane), a64.lsrv(17, 17, 16), a64.ins_d(d, lane, 17))
        a.branch(a64.b, done)
        a.bind(big)
        a.emit(a64.movi_zero(d))
        a.bind(done)
        return
    if ic in ("PCMPISTRI", "PCMPISTRI64"):                     # run in the model (src/w2m/string_compare.py; a
        a.emit(a64.hvc(HVC_SLOW_SSE))                            # NEON lowering later)
        return
    if ic in ("RCPPS", "RSQRTPS", "RCPSS", "RSQRTSS"):         # the reference CPU's tables: run in the model
        a.emit(a64.hvc(HVC_SLOW_SSE))                            # (src/w2m/approximate.py; a NEON lookup later)
        return
    if ic == "MOVMSKPS":                                         # the four sign bits into a general register
        v, rd = int(src[1]), int(dst[1])
        a.emit(a64.mov(rd, a64.ZR))
        for lane in range(4):
            a.emit(a64.umov_s(16, v, lane), a64.lsri(16, 16, 31))
            a.emit(a64.eor(rd, rd, 16, a64.LSL, lane))            # the bits do not overlap: EOR is OR
        return
    if ic in SHIFTS and imm is not None:                         # by imm8: past the lane, zero (logical) or the sign
        size, kind = SHIFTS[ic]
        bits = 8 << size
        n = min(imm, bits)
        if kind == "arith":
            if n:
                a.emit(a64.shift_right(d, d, n, size, True))
        elif n >= bits:
            a.emit(a64.movi_zero(d))
        elif n:
            a.emit(a64.shift_right(d, d, n, size, False) if kind == "right" else a64.shift_left(d, d, n, size))
        return
    if ic in SHIFTS:                                             # by the source's low 64 bits, clamped to the lane's bits
        size, kind = SHIFTS[ic]
        bits = 8 << size
        v = _sse_src(a, src, 16, reg_of)
        small = a.label()
        a.emit(a64.umov_d(16, v, 0), a64.cmpi(16, bits))
        a.branch(lambda off: a64.bcond("ls", off), small)
        a.emit(a64.movz(16, bits))
        a.bind(small)
        if kind != "left":
            a.emit(a64.sub(16, a64.ZR, 16))                      # a negative count shifts right
        a.emit(a64.dup_gpr(30, 16, size))                        # the shift count is the lane's low byte
        a.emit(a64.three_same("sshl" if kind == "arith" else "ushl", d, d, 30, size))
        return
    if ic == "PINSRW":                                           # the source's low word into word imm & 7
        assert imm is not None
        if src[0] == "m":
            a.emit(a64.ldr(2, 16, reg_of(src[1]), 0))
            a.emit(a64.ins_h(d, imm & 7, 16))
        else:
            a.emit(a64.ins_h(d, imm & 7, int(src[1])))
        return
    if ic == "MOVHPS":                                           # the high quadword from or to memory
        if dst[0] == "x":
            a.emit(a64.ldr(8, 16, reg_of(src[1]), 0), a64.ins_d(d, 1, 16))
        else:
            a.emit(a64.umov_d(16, int(src[1]), 1), a64.str_(8, 16, reg_of(dst[1]), 0))
        return
    if ic in ("PSHUFLW", "PSHUFHW"):                             # four words permuted by imm; the other four kept
        assert imm is not None
        v = _sse_src(a, src, 16, reg_of, scratch=31)
        a.emit(a64.mov_v(30, v), a64.mov_v(d, 30))
        base = 0 if ic == "PSHUFLW" else 4
        for lane in range(4):
            a.emit(a64.ins_h_element(d, base + lane, 30, base + (imm >> (2 * lane) & 3)))
        return
    if ic == "SHUFPS":                                           # words 0, 1 from the destination; 2, 3 from the source
        assert imm is not None
        v = _sse_src(a, src, 16, reg_of, scratch=31)
        a.emit(a64.mov_v(30, d))
        if v != 31:
            a.emit(a64.mov_v(31, v))
        for lane in range(4):
            a.emit(a64.ins_s_element(d, lane, 30 if lane < 2 else 31, imm >> (2 * lane) & 3))
        return
    if ic == "MOVLPS":                                           # the low quadword from or to memory
        if dst[0] == "x":
            a.emit(a64.ldr(8, 16, reg_of(src[1]), 0), a64.ins_d(d, 0, 16))
        else:
            a.emit(a64.umov_d(16, int(src[1]), 0), a64.str_(8, 16, reg_of(dst[1]), 0))
        return
    if ic in ("MOVHLPS", "MOVLHPS"):                             # one quadword between halves; the other kept
        v = int(src[1])
        a.emit(a64.umov_d(16, v, 1 if ic == "MOVHLPS" else 0), a64.ins_d(d, 0 if ic == "MOVHLPS" else 1, 16))
        return
    if ic == "PSHUFD":                                           # word i of the result = source word (imm >> 2i) & 3
        assert imm is not None
        v = _sse_src(a, src, 16, reg_of, scratch=31)
        a.emit(a64.mov_v(30, v))
        for lane in range(4):
            a.emit(a64.ins_s_element(d, lane, 30, imm >> (2 * lane) & 3))
        return
    v = _sse_src(a, src, 16, reg_of)
    if ic in ("PAND", "POR", "ORPS"):
        a.emit((a64.and_v if ic == "PAND" else a64.orr_v)(d, d, v))
    elif ic in ("PSUBD", "PSUBQ"):
        a.emit(a64.sub_v(d, d, v, 2 if ic == "PSUBD" else 3))
    elif ic == "PUNPCKLBW":
        a.emit(a64.zip1_v(d, d, v))
    elif ic == "PCMPEQB":
        a.emit(a64.cmeq_v(d, d, v))
    elif ic == "PXOR":
        a.emit(a64.eor_v(d, d, v))
    elif ic == "ANDNPS":
        a.emit(a64.bic_v(d, v, d))
    elif ic in ("UNPCKLPS", "PUNPCKLDQ"):
        a.emit(a64.zip1_4s(d, d, v))
    elif ic == "UNPCKHPS":
        a.emit(a64.zip2_4s(d, d, v))
    elif ic == "PADDD":
        a.emit(a64.add_v(d, d, v, 2))
    elif ic in LANE_LOWERED:
        name, size = LANE_LOWERED[ic]
        a.emit(a64.three_same(name, d, d, v, size))
    elif ic == "PANDN":
        a.emit(a64.bic_v(d, v, d))
    elif ic == "PACKUSWB":                                       # low half from the destination, high from the source
        a.emit(a64.sqxtun(30, d, False), a64.sqxtun(30, v, True), a64.mov_v(d, 30))
    elif ic == "PACKSSWB":                                       # as PACKUSWB, to signed bytes
        a.emit(a64.sqxtn(30, d, False), a64.sqxtn(30, v, True), a64.mov_v(d, 30))
    elif ic == "PMULHW":                                         # each word's signed product, its high half
        a.emit(a64.smull(30, d, v, False), a64.smull(31, d, v, True), a64.three_same("uzp2", d, 30, 31, 1))
    elif ic == "PSHUFB":                                         # a byte's index with bit 7 set gives 0; TBL gives 0
        a.emit(a64.movi_bytes(30, 0x8F), a64.and_v(30, v, 30), a64.tbl(d, d, 30))     # for any index past 15
    elif ic == "PSADBW":
        # each quadword's eight absolute byte differences summed into its low word, the rest zeroed (SDM PSADBW):
        # UABD, then three pairwise widening adds leave each sum (at most 2,040) alone in its 64-bit lane. Native
        # from Sprint 068: frontier 1ecc22b2 ran 152,246 model steps here past 41.7 billion instructions
        a.emit(a64.three_same("uabd", 30, d, v, 0), a64.add_pairwise_long(30, 30, 0, False),
               a64.add_pairwise_long(30, 30, 1, False), a64.add_pairwise_long(30, 30, 2, False), a64.mov_v(d, 30))
    elif ic == "PMADDUBSW":
        # the destination's unsigned bytes times the source's signed bytes, each adjacent pair's products added with
        # signed saturation to a word (SDM PMADDUBSW). A product fits a halfword (255 x -128 = -32,640), SADDLP adds
        # the pairs into words without overflow, SQXTN saturates. The source is read whole before any scratch it
        # may be (V31 from memory, V29 under VEX) is written. Native from Sprint 068: 344,613 model steps
        t1, t2 = [t for t in (30, 31, 29) if t != v][:2]
        t3 = v if v in (29, 30, 31) else 29
        a.emit(a64.extend_bytes(t1, v, True, True), a64.extend_bytes(t2, d, True, False),
               a64.three_same("mul", t2, t2, t1, 1), a64.add_pairwise_long(t2, t2, 1, True),      # high sums in t2
               a64.extend_bytes(t1, v, False, True), a64.extend_bytes(t3, d, False, False),
               a64.three_same("mul", t3, t3, t1, 1), a64.add_pairwise_long(t3, t3, 1, True),      # low sums in t3
               a64.sqxtn_h(t1, t3, False), a64.sqxtn_h(t1, t2, True), a64.mov_v(d, t1))
    else:
        raise CompileError(f"sse {ic}")


def lower(a: Assembler, op: Op, reg_of: Callable[[int], int], store: Callable[[Op, int], None]) -> None:
    _, ic, dst, src, align, *rest = op
    imm = rest[0] if rest else None
    for loc in (dst, src):
        if loc[0] == "m" and align:
            _check_alignment(a, reg_of(loc[1]))
    if ic in FP_CLASSES:                                         # fast path only when MXCSR masks all six
        fast = a.label()
        a.emit(a64.ldr(4, 16, 28, MXCSR_OFF), a64.ubfx(16, 16, 7, 6), a64.cmpi(16, 0x3F))
        a.branch(lambda off: a64.bcond("eq", off), fast)
        a.emit(a64.hvc(HVC_SLOW_SSE))
        a.bind(fast)
    if ic in SSE2_INT_LOWERED:
        _lower_int(a, ic, dst, src, imm, reg_of)
        return
    if ic == "PEXTRW":
        # word imm & 7 of the XMM source, zero-extended into the register (SDM PEXTRW; a memory operand is #UD in
        # register_only_rule). Native from Sprint 068: 150,337 model steps at frontier 1ecc22b2
        assert imm is not None
        a.emit(a64.umov_h(16, int(src[1]), imm & 7))
        store(dst, 16)
        return
    if ic in ("MOVAPS", "MOVAPD", "MOVUPS", "MOVUPD", "LDDQU"):     # LDDQU: a 16-byte load, memory only (Sprint 065)
        if dst[0] == "x" and src[0] == "x":
            a.emit(a64.mov_v(dst[1], src[1]))
        elif dst[0] == "x":
            a.emit(a64.ldr_v(16, dst[1], reg_of(src[1])))
        else:
            a.emit(a64.str_v(16, src[1], reg_of(dst[1])))
    elif ic in ("MOVSS", "MOVSD_XMM"):
        n = 4 if ic == "MOVSS" else 8
        if dst[0] == "x" and src[0] == "x":
            a.emit(a64.ins_lane0(n, dst[1], src[1]))             # register form merges
        elif dst[0] == "x":
            a.emit(a64.ldr_v(n, dst[1], reg_of(src[1])))         # load zeroes the rest
        else:
            a.emit(a64.str_v(n, src[1], reg_of(dst[1])))
    elif ic in ("MOVD", "MOVQ"):
        if src[0] == "r":
            n = src[2] // 8
            a.emit(a64.mov(16, src[1]) if n == 8 else a64.mov32(16, src[1]), a64.fmov_to_v(n, dst[1], 16))
        elif dst[0] == "r":
            n = dst[2] // 8
            a.emit(a64.fmov_from_v(n, 16, src[1]))
            store(dst, 16)
        elif dst[0] == "x" and src[0] == "m":
            a.emit(a64.ldr_v(4 if ic == "MOVD" else 8, dst[1], reg_of(src[1])))
        elif dst[0] == "m":
            a.emit(a64.str_v(4 if ic == "MOVD" else 8, src[1], reg_of(dst[1])))
        else:                                                    # MOVQ xmm, xmm: low 64 bits, the rest zeroed
            a.emit(a64.fmov_from_v(8, 16, src[1]), a64.fmov_to_v(8, dst[1], 16))
    elif ic in ("XORPS", "XORPD", "ANDPS", "ANDPD", "ORPD"):
        v = _sse_src(a, src, 16, reg_of)
        a.emit((a64.eor_v if ic.startswith("XOR") else a64.orr_v if ic == "ORPD" else a64.and_v)(dst[1], dst[1], v))
    elif ic == "CVTDQ2PS" and native_conversions:
        # SCVTF rounds in FPCR's mode, which is MXCSR's RC (fpcr_for), and reports an inexact result as FPSR.IXC, MXCSR's
        # PE; x86 reports only PE here (SDM CVTDQ2PS: Precision). Native from Sprint 064: the frontier ran 24,291 of its
        # model steps here. An unmasked exception still goes to the model, which raises it.
        _unmasked_to_model(a)
        v = _sse_src(a, src, 16, reg_of)
        a.emit(a64.scvtf_4s(dst[1], v))
    elif ic == "CVTPS2PD" and native_conversions:
        # FCVTL widens the low two singles exactly; an SNaN becomes a QNaN with FPSR.IOC (MXCSR's IE), and FPCR.AH=1
        # gives x86's NaN and denormal-input behavior (Sprint 039 probe). The memory form reads 8 bytes (SDM CVTPS2PD:
        # m64). Native from Sprint 064: the frontier ran 50,333 of its model steps at one VCVTPS2PD.
        _unmasked_to_model(a)
        v = _sse_src(a, src, 8 if src[0] == "m" else 16, reg_of)
        a.emit(a64.fcvtl_2d(dst[1], v))
    elif ic == "CVTPD2PS" and native_conversions:
        # FCVTN narrows each double in FPCR's rounding mode (MXCSR's RC) into the low half and clears the high
        # half, as x86 zeroes bits 127:64 (SDM CVTPD2PS); OE, UE, PE and IE (an SNaN) come as FPSR's OFC, UFC, IXC
        # and IOC, with FPCR.AH's x86 tininess and NaN rules (Sprint 039 probe). Native from Sprint 064: a
        # 20-billion-instruction frontier ran 663,562 model steps at one CVTPD2PS.
        _unmasked_to_model(a)
        v = _sse_src(a, src, 16, reg_of)
        a.emit(a64.fcvtn_2s(dst[1], v))
    elif ic == "CVTPD2DQ" and native_conversions:
        # Each double as CVTSD2SI converts it, the results in the low two lanes and the rest zeroed (SDM CVTPD2DQ):
        # FRINTX rounds in MXCSR's mode and FCVTZS converts; NaN or a value at or past 2^31 gives the integer
        # indefinite 0x80000000 and IE (FCVTZS saturates, and gives 0 for NaN), with this lane's other flags dropped;
        # in range, the lane keeps PE and FPSR's IDC as it was. x18 holds FPSR before the lane. The source goes to
        # V30, V29 holds the bound, and the two results pass through the spill slot. Native from Sprint 064: the
        # frontier ran 47,825 of its model steps at VCVTPD2DQ and CVTPD2DQ.
        _unmasked_to_model(a)
        v = _sse_src(a, src, 16, reg_of)
        if v != 30:
            a.emit(a64.mov_v(30, v))
        for lane in (0, 1):
            ok, low, done, indefinite = a.label(), a.label(), a.label(), a.label()
            a.emit(a64.mrs(18, a64.FPSR), a64.umov_d(17, 30, lane), a64.fmov_to_v(8, 31, 17),
                   a64.frintx(8, 31, 31), a64.fcvtzs(32, 8, 16, 31),
                   a64.mov_imm(17, 0x41E0_0000_0000_0000), a64.fmov_to_v(8, 29, 17), a64.fcmp(8, 31, 29))
            # in range when -2^31 <= the rounded value < 2^31 (x86fp.to_int); NaN is unordered, so not below 2^31.
            # The lower bound matters for the flags only: below it FCVTZS gives 0x80000000 too, but FRINTX's IXC
            # would survive as PE where the Xeon reports IE alone (set 050 case 227, MXCSR 0x9f80 -> 0x9f81)
            a.branch(lambda off: a64.bcond("mi" if vb.CVT_INDEFINITE else "al", off), low)
            a.branch(a64.b, indefinite)
            a.bind(low)
            a.emit(a64.mov_imm(17, 0xC1E0_0000_0000_0000), a64.fmov_to_v(8, 29, 17), a64.fcmp(8, 31, 29))
            a.branch(lambda off: a64.bcond("ge" if vb.CVT_INDEFINITE else "al", off), ok)
            a.bind(indefinite)
            a.emit(a64.mov_imm(16, 0x8000_0000), a64.movz(17, 1), a64.orr(18, 18, 17), a64.msr(a64.FPSR, 18))
            a.branch(a64.b, done)
            a.bind(ok)                                           # in range: IDC as it was before the lane
            a.emit(a64.mrs(17, a64.FPSR), a64.ubfx(18, 18, 7, 1), a64.bfi(17, 18, 7, 1), a64.msr(a64.FPSR, 17))
            a.bind(done)
            a.emit(a64.str_(4, 16, 28, vb.SPILL + 4 * lane))
        a.emit(a64.ldr(8, 16, 28, vb.SPILL), a64.fmov_to_v(8, dst[1], 16))   # FMOV Dd zeroes bits 127:64
    elif ic in ("CVTTPD2DQ", "CVTPD2DQ", "CMPSD_XMM", "HADDPD", "MPSADBW") or ic in SSE_050:  # in the model (x86fp; Sprints
        a.emit(a64.hvc(HVC_SLOW_SSE))                                                # 049, 050), whole
    elif ic[:-2] in ("ADD", "SUB", "MUL", "DIV", "MIN", "MAX", "SQRT"):
        kind, form = ic[:-2].lower(), ic[-2:]
        n = 4 if form in ("SS", "PS") else 8
        if form in ("SS", "SD"):
            v = _sse_src(a, src, n, reg_of)
            if kind == "sqrt":
                a.emit(a64.fsqrt_scalar(n, 30, v))
            else:
                a.emit(a64.fp_scalar(kind, n, 30, dst[1], v))
            a.emit(a64.ins_lane0(n, dst[1], 30))
        else:
            v = _sse_src(a, src, 16, reg_of)
            a.emit(a64.fsqrt_vector(n, dst[1], v) if kind == "sqrt" else a64.fp_vector(kind, n, dst[1], dst[1], v))
    elif ic in ("CVTTSS2SI", "CVTTSD2SI", "CVTSS2SI", "CVTSD2SI"):
        # x86 reports only IE and PE here (SDM CVTTSS2SI: Invalid, Precision), and only IE when the
        # result is the integer indefinite (Intel runs, Sprint 041). FRINTX, FCVTZS and the range
        # check can set IDC, and FRINTX sets IXC before an invalid conversion, so FPSR is rebuilt from
        # its value before the instruction (in x18: the only temporary of this instruction, a memory
        # source's address, is dead once the source is loaded).
        n = 4 if "SS" in ic else 8
        bits = dst[2]
        v = _sse_src(a, src, n, reg_of)
        a.emit(a64.mrs(18, a64.FPSR))
        if not ic.startswith("CVTT"):                            # round in MXCSR's mode first
            a.emit(a64.frintx(n, 30, v))
            v = 30
        a.emit(a64.fcvtzs(bits, n, 16, v))
        # the integer indefinite 1 << (bits-1) on NaN or input >= 2^(bits-1) (SDM CVTTSS2SI); FCVTZS
        # saturates and gives 0 for NaN, which FPCR.AH does not change (Sprint 039 probe)
        bound = {(4, 32): 0x4F00_0000, (4, 64): 0x5F00_0000, (8, 32): 0x41E0_0000_0000_0000,
                 (8, 64): 0x43E0_0000_0000_0000}[(n, bits)]
        ok, done = a.label(), a.label()
        breg = 31 if v != 31 else 30
        a.emit(a64.mov_imm(17, bound), a64.fmov_to_v(n, breg, 17), a64.fcmp(n, v, breg))
        a.branch(lambda off: a64.bcond("mi" if vb.CVT_INDEFINITE else "al", off), ok)
        a.emit(a64.mov_imm(16, 1 << (bits - 1)), a64.movz(17, 1), a64.orr(18, 18, 17), a64.msr(a64.FPSR, 18))
        a.branch(a64.b, done)
        a.bind(ok)                                               # in range: IDC as it was before
        a.emit(a64.mrs(17, a64.FPSR), a64.ubfx(18, 18, 7, 1), a64.bfi(17, 18, 7, 1), a64.msr(a64.FPSR, 17))
        a.bind(done)
        store(dst, 16)
    elif ic in ("CVTSI2SS", "CVTSI2SD"):
        n = 4 if ic.endswith("SS") else 8
        bits = src[2] if src[0] == "r" else 8 * src[2]
        if src[0] == "r":
            a.emit(a64.mov(16, src[1]) if bits == 64 else a64.mov32(16, src[1]))
        else:
            a.emit(a64.ldr(bits // 8, 16, reg_of(src[1])))
        if bits == 32:
            a.emit(a64.sxtw(16, 16))
        a.emit(a64.scvtf(n, 64, 30, 16), a64.ins_lane0(n, dst[1], 30))
    elif ic in ("CVTSS2SD", "CVTSD2SS"):
        sn, dn = (4, 8) if ic == "CVTSS2SD" else (8, 4)
        v = _sse_src(a, src, sn, reg_of)
        a.emit(a64.fcvt(dn, sn, 30, v), a64.ins_lane0(dn, dst[1], 30))
    elif ic in ("UCOMISS", "UCOMISD", "COMISS", "COMISD"):
        n = 4 if ic.endswith("SS") else 8
        v = _sse_src(a, src, n, reg_of)
        a.emit(a64.fcmp(n, dst[1], v, signal=ic.startswith("COMI")))
        _set_flags_from_fcmp(a)
    else:
        raise CompileError(f"sse {ic}")
