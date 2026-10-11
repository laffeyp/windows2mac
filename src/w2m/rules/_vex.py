# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Shared VEX rule and AArch64 lowering (Sprint 049: the guest profile is x86-64-v3).

A VEX instruction is its legacy-SSE base with three operands: d = BASE(s1, s2) on bits 127:0, the same on bits
255:128 at VEX.256, and a VEX.128 write to an XMM register zeroes bits 255:128 (Intel SDM Vol 1 14.1.1). The
lowering copies s1 into d (s2 first to V29 when it is d), runs the base's own lowering, then clears d's upper half.
YMM upper halves live in the monitor data page at YMM_HI_OFF, 16 bytes each; V28 and V29 are VEX scratch. FMA runs
as AArch64's fused multiply-add, one rounding (FMADD a*b+c, x86 FMSUB as FNMSUB, FNMADD as FMSUB, FNMSUB as
FNMADD). What has no native lowering yet runs in the model (hvc #10), which executes the whole instruction.
"""

from __future__ import annotations

from collections.abc import Callable
from typing import Any, cast

from w2m import a64_encoder as a64
from w2m.lift import LiftError, Op, _operand
from w2m.rules import Ctx
from w2m.vm_backend import (FP_CLASSES, HVC_SLOW_SSE, MXCSR_OFF, SPILL, YMM_HI_OFF, Assembler, CompileError)

# VEX iclass -> the legacy base whose semantics it has (the model's _sse and the base's lowering)
BASE = {"VMULSD": "MULSD", "VADDSD": "ADDSD", "VSUBSD": "SUBSD", "VDIVSD": "DIVSD", "VMOVSD": "MOVSD_XMM",
        "VMOVQ": "MOVQ", "VMOVD": "MOVD", "VMOVAPD": "MOVAPD", "VMOVDQA": "MOVDQA", "VMOVDQU": "MOVDQU",
        "VCOMISD": "COMISD", "VUCOMISD": "UCOMISD", "VPAND": "PAND", "VPOR": "POR", "VPXOR": "PXOR",
        "VXORPS": "XORPS", "VXORPD": "XORPD", "VORPD": "ORPD", "VANDPD": "ANDPD", "VANDPS": "ANDPS",
        "VPCMPEQB": "PCMPEQB", "VPCMPEQW": "PCMPEQW", "VPSUBQ": "PSUBQ", "VPSUBD": "PSUBD", "VPADDQ": "PADDQ",
        "VCVTDQ2PD": "CVTDQ2PD", "VPSRLQ": "PSRLQ", "VPSLLQ": "PSLLQ", "VPSRLDQ": "PSRLDQ", "VPMOVMSKB": "PMOVMSKB",
        "VCVTTPD2DQ": "CVTTPD2DQ", "VCVTPD2DQ": "CVTPD2DQ", "VCMPSD": "CMPSD_XMM_VEX", "VMOVLHPS": "MOVLHPS",
        "VHADDPD": "HADDPD"}
FMA = {f"V{k}{o}{t}" for k in ("FMADD", "FMSUB", "FNMADD", "FNMSUB") for o in ("132", "213", "231")
       for t in ("SS", "SD", "PS", "PD")}
L0_ONLY = {"VCVTDQ2PD", "VCVTTPD2DQ", "VCVTPD2DQ"}                 # their 256-bit forms change operand widths
ALIGNED = {"VMOVAPD", "VMOVDQA"}                                     # the only VEX forms that fault on alignment
NO_WRITE = {"COMISD", "UCOMISD"}                                     # flags only: the destination is a source
SLOW = {"CVTTPD2DQ", "CVTPD2DQ", "CMPSD_XMM_VEX", "HADDPD"}              # in the model (x86fp) for now
NATIVE_256 = {"PXOR", "XORPS", "XORPD", "PAND", "POR", "ORPD", "ANDPD", "ANDPS", "PCMPEQB", "PCMPEQW", "PSUBQ",
              "PSUBD", "PADDQ", "MOVDQU", "MOVDQA", "MOVAPD", "PMOVMSKB"}
FUSED = {"FMADD": "fmadd", "FMSUB": "fnmsub", "FNMADD": "fmsub", "FNMSUB": "fnmadd"}


def rule(ctx: Ctx) -> None:
    b, ic, ops, nxt = ctx.b, ctx.ic, ctx.ops, ctx.nxt
    template = ctx.f.ops if ctx.f is not None and ctx.f.ops else ""
    # VEX.L counts only for templates with x-forms; a scalar row ignores it (LIG: SDM Vol 2A 2.3.6.2), and its
    # destination's bits 255:128 are zeroed whatever L is
    vl = ctx.f.vl if ctx.f is not None and "x" in template else 0
    if ic == "VZEROUPPER":
        b.emit("vzeroupper")
        return
    locs = [cast(tuple[Any, ...], _operand(b, o, nxt)[1]) for o in ops if not o.startswith("I:")]
    imm = int(ops[-1].split(":")[2], 16) if ops[-1].startswith("I:") else None
    if ic in FMA:
        b.emit("fma", ic, locs[0], locs[1], locs[2], vl)
        return
    if ic in L0_ONLY and ctx.f is not None and ctx.f.vl:        # the encoded L: their 256-bit forms differ in width
        raise LiftError(f"{ic} at 0x{ctx.address:x}: the 256-bit form is not lifted")
    base = BASE[ic]
    if imm is not None and len(locs) == 2 and template == "Hx,Wx,Ib":
        d, s1, s2 = locs[0], locs[1], locs[1]                       # shift by imm8: vvvv = shift(r/m)
    elif len(locs) == 3:
        d, s1, s2 = locs
    else:
        d, s1, s2 = locs[0], None, locs[1]
    has_memory = d[0] == "m" or s2[0] == "m"
    align = (32 if vl else 16) if ic in ALIGNED and has_memory else 0
    b.emit("vex", base, d, s1, s2, vl, align, imm)


def _clear_upper(a: Assembler, reg: int) -> None:
    a.emit(a64.addi(16, 28, YMM_HI_OFF), a64.stp(a64.ZR, a64.ZR, 16, 16 * reg))


def _slow(a: Assembler) -> None:
    a.emit(a64.hvc(HVC_SLOW_SSE))


def _fp_guard(a: Assembler) -> None:
    """The model runs the instruction when MXCSR unmasks any exception (as the legacy FP classes do), checked
    before any register is staged, so the model sees the instruction's inputs."""
    fast = a.label()
    a.emit(a64.ldr(4, 16, 28, MXCSR_OFF), a64.ubfx(16, 16, 7, 6), a64.cmpi(16, 0x3F))
    a.branch(lambda off: a64.bcond("eq", off), fast)
    _slow(a)
    a.bind(fast)


def _base_lower(base: str) -> Callable[..., None]:
    from w2m.rules import registry
    found = registry().get(base)
    if found is None or found.lower is None:
        raise CompileError(f"vex {base}: the base class has no lowering")
    return found.lower


def lower(a: Assembler, op: Op, reg_of: Callable[[int], int], store: Callable[[Op, int], None]) -> None:
    k = op[0]
    if k == "vzeroupper":
        a.emit(a64.addi(16, 28, YMM_HI_OFF))
        for i in range(16):
            a.emit(a64.stp(a64.ZR, a64.ZR, 16, 16 * i))
        return
    if k == "fma":
        _lower_fma(a, op, reg_of)
        return
    _, base, d, s1, s2, vl, align, imm = op
    if base in SLOW or (vl and base not in NATIVE_256):
        _slow(a)
        return
    if base in FP_CLASSES:
        _fp_guard(a)
    for loc in (d, s2):
        if loc[0] == "m" and align:
            ok = a.label()
            a.emit(a64.ubfx(16, reg_of(loc[1]), 0, 5 if align == 32 else 4))
            a.branch(lambda off: a64.cbz(16, off), ok)
            a.emit(a64.movz(16, 0), a64.ldr(8, 16, 16, 0))           # #GP: an access violation at the instruction
            a.bind(ok)
    base_lower = _base_lower(base)
    if vl and base == "PMOVMSKB":                                    # 32 sign bits: high half first, then low
        rd = int(d[1])
        a.emit(a64.addi(16, 28, YMM_HI_OFF), a64.ldr_v(16, 29, 16, 16 * int(s2[1])))
        base_lower(a, ("sse", base, d, ("x", 29), 0), reg_of, store)
        a.emit(a64.eor(rd, a64.ZR, rd, a64.LSL, 16), a64.str_(8, rd, 28, SPILL))
        base_lower(a, ("sse", base, d, s2, 0), reg_of, store)
        a.emit(a64.ldr(8, 16, 28, SPILL), a64.orr(rd, rd, 16))
        return
    low_src = s2
    hi_src = 29
    if s1 is not None and s1[0] == "x" and d[0] == "x" and s1[1] != d[1]:
        if s2 == ("x", d[1]):                                        # s2 is d: keep it before d takes s1
            a.emit(a64.mov_v(29, int(d[1])))
            low_src, hi_src = ("x", 29), 27
        a.emit(a64.mov_v(int(d[1]), int(s1[1])))
    if vl and d[0] == "x":                                           # stage both upper halves before d changes
        a.emit(a64.addi(16, 28, YMM_HI_OFF))
        if s1 is not None:
            a.emit(a64.ldr_v(16, 28, 16, 16 * int(s1[1])))
        if s2[0] == "x":
            a.emit(a64.ldr_v(16, hi_src, 16, 16 * int(s2[1])))
        elif s2[0] == "m":
            a.emit(a64.ldr_v(16, hi_src, reg_of(s2[1]), 16))
    rest = (imm,) if imm is not None else ()
    if d[0] == "m" and vl:                                           # a 256-bit store: the upper half first, so a
        a.emit(a64.addi(16, 28, YMM_HI_OFF), a64.ldr_v(16, 28, 16, 16 * int(s2[1])),   # fault on the next page
               a64.str_v(16, 28, reg_of(d[1]), 16))                                   # leaves memory as it was
    base_lower(a, ("sse", base, d, low_src, 0, *rest), reg_of, store)
    if d[0] == "x" and base not in NO_WRITE:
        if not vl:
            _clear_upper(a, int(d[1]))
        else:
            if s1 is None:                                           # a 256-bit load or move: the source's upper half
                a.emit(a64.mov_v(28, hi_src))
            else:
                base_lower(a, ("sse", base, ("x", 28), ("x", hi_src), 0, *rest), reg_of, store)
            a.emit(a64.addi(16, 28, YMM_HI_OFF), a64.str_v(16, 28, 16, 16 * int(d[1])))


def _lower_fma(a: Assembler, op: Op, reg_of: Callable[[int], int]) -> None:
    """Scalar FMA natively when MXCSR masks every exception; packed forms in the model for now."""
    _, ic, d, s2, s3, _vl = op
    kind, order, form = ic[1:-5], ic[-5:-2], ic[-2:]
    if form[0] == "P":
        _slow(a)
        return
    n = 4 if form == "SS" else 8
    _fp_guard(a)
    o1, o2 = int(d[1]), int(s2[1])
    if s3[0] == "m":
        a.emit(a64.ldr_v(n, 31, reg_of(s3[1])))
        o3 = 31
    else:
        o3 = int(s3[1])
    regs = (o1, o2, o3)
    (m0, m1), add = {"132": ((0, 2), 1), "213": ((1, 0), 2), "231": ((1, 2), 0)}[order]
    a.emit(a64.fp_fused(FUSED[kind], n, 30, regs[m0], regs[m1], regs[add]), a64.ins_lane0(n, o1, 30))
    _clear_upper(a, o1)
