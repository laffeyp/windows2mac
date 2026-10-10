"""Shared SSE rule and AArch64 lowering (moved unchanged from lift.py and vm_backend.py in Sprint
041). XMM i is V i; V30 and V31 are scratch; FPCR follows MXCSR (vm_backend.fpcr_for). An
instruction that can raise an SIMD exception leaves the fast path when MXCSR unmasks any."""

from __future__ import annotations

from collections.abc import Callable
from typing import Any, cast

from w2m import vm_backend as vb
from w2m.lift import SSE_ALIGNED, Op, _operand
from w2m.rules import Ctx
from w2m.vm_backend import (AF_OPERANDS, FP_CLASSES, HVC_SLOW_SSE, MXCSR_OFF, REG_CF, REG_OF, REG_PF, REG_SF,
                            REG_ZF, U_AF, U_CF, U_OF, U_PF, U_SF, U_ZF, Assembler, CompileError, _mark_defined)
from w2m import a64_encoder as a64


def rule(ctx: Ctx) -> None:
    """One legacy-SSE instruction as one `sse` operation; the runtime (x86fp) and the lowering below
    compute it (lift.py's SSE rule, Sprint 039)."""
    b, ic, ops, nxt = ctx.b, ctx.ic, ctx.ops, ctx.nxt
    dst = cast(tuple[Any, ...], _operand(b, ops[0], nxt)[1])
    src = cast(tuple[Any, ...], _operand(b, ops[1], nxt)[1])
    has_memory = dst[0] == "m" or src[0] == "m"
    b.emit("sse", ic, dst, src, 16 if has_memory and ic in SSE_ALIGNED else 0)


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
    a.emit(a64.cset(REG_PF, "vs"), a64.cset(REG_CF, "lt"), a64.cset(REG_ZF, "eq"),
           a64.orr(REG_ZF, REG_ZF, REG_PF), a64.mov(REG_OF, a64.ZR), a64.mov(REG_SF, a64.ZR),
           a64.stp(a64.ZR, a64.ZR, 28, AF_OPERANDS), a64.str_(8, a64.ZR, 28, AF_OPERANDS + 16))
    _mark_defined(a, U_CF | U_ZF | U_SF | U_OF | U_PF | U_AF)


def lower(a: Assembler, op: Op, reg_of: Callable[[int], int], store: Callable[[Op, int], None]) -> None:
    _, ic, dst, src, align = op
    for loc in (dst, src):
        if loc[0] == "m" and align:
            _check_alignment(a, reg_of(loc[1]))
    if ic in FP_CLASSES:                                         # fast path only when MXCSR masks all six
        fast = a.label()
        a.emit(a64.ldr(4, 16, 28, MXCSR_OFF), a64.ubfx(16, 16, 7, 6), a64.cmpi(16, 0x3F))
        a.branch(lambda off: a64.bcond("eq", off), fast)
        a.emit(a64.hvc(HVC_SLOW_SSE))
        a.bind(fast)
    if ic in ("MOVAPS", "MOVAPD", "MOVUPS", "MOVUPD"):
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
    elif ic in ("XORPS", "XORPD", "ANDPS", "ANDPD"):
        v = _sse_src(a, src, 16, reg_of)
        a.emit((a64.eor_v if ic.startswith("XOR") else a64.and_v)(dst[1], dst[1], v))
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
