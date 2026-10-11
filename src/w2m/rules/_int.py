# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Shared integer and control-flow rules (moved unchanged from lift.py in Sprint 041). Semantics
follow the Intel SDM Vol 2 page of each instruction; the commit rule (specs/guest_state.json
0.2.0) orders every register and flag write after the last memory access."""

from __future__ import annotations

from collections.abc import Callable
from typing import Any, cast

from w2m.lift import ALU, CC, RAX, RDX, REP_STRING, RSP, LiftError, _operand, _pop, _push, _val
from w2m.rules import Ctx


def nop(ctx: Ctx) -> None:
    pass


def df(ctx: Ctx) -> None:
    ctx.b.emit("df", int(ctx.ic == "STD"))


def cpuid(ctx: Ctx) -> None:
    ctx.b.emit("cpuid")                                  # EAX, EBX, ECX, EDX from the guest profile


def ret_near(ctx: Ctx) -> None:
    """RET, and RET imm16: in 64-bit mode RSP <- RSP + 8 + imm16, the count zero-extended (SDM RET)."""
    b = ctx.b
    if not ctx.ops:
        b.emit("br", "always", _pop(b))
        return
    _, count, _ = _operand(b, ctx.ops[0], ctx.nxt)
    sp, target, eight, popped, nsp = b.t(), b.t(), b.t(), b.t(), b.t()
    b.emit("ld", sp, ("r", RSP, 64, False))
    b.emit("ld", target, ("m", sp, 8))
    b.emit("imm", eight, 8)
    b.emit("alu", popped, "add", sp, eight, 64)
    b.emit("alu", nsp, "add", popped, count, 64)
    b.emit("st", ("r", RSP, 64, False), nsp)
    b.emit("br", "always", target)


def move(ctx: Ctx) -> None:
    """MOV, MOVZX, MOVSXD, LEA."""
    b, ic, ops, nxt = ctx.b, ctx.ic, ctx.ops, ctx.nxt
    dk, dst, dbits = _operand(b, ops[0], nxt)
    sk, src, sbits = _operand(b, ops[1], nxt, dbits)
    v = src if sk in ("addr", "imm") else _val(b, sk, src)
    if ic == "MOVSXD" and sbits == 32 and dbits == 64:
        w = b.t()
        b.emit("alu", w, "sext32", v, v, 64)
        v = w
    elif ic == "MOVSXD" and sbits != dbits:
        raise LiftError(f"MOVSXD {sbits}->{dbits}")
    b.emit("st", dst, v)


def alu(ctx: Ctx) -> None:
    """ADD SUB AND OR XOR CMP TEST."""
    b, ic, ops, nxt = ctx.b, ctx.ic, ctx.ops, ctx.nxt
    dk, dst, bits = _operand(b, ops[0], nxt)
    sk, src, _ = _operand(b, ops[1], nxt, bits)
    a, c, r = _val(b, dk, dst), _val(b, sk, src), b.t()
    b.emit("alu", r, ALU[ic], a, c, bits)
    if ic not in ("CMP", "TEST"):
        b.emit("st", dst, r)                              # store before flags: the commit rule
    b.emit("flags", "logic" if ALU[ic] in ("and", "or", "xor") else ALU[ic], a, c, r, bits)


def inc_dec(ctx: Ctx) -> None:
    b, ic, ops, nxt = ctx.b, ctx.ic, ctx.ops, ctx.nxt
    dk, dst, bits = _operand(b, ops[0], nxt)
    a, one, r = _val(b, dk, dst), b.t(), b.t()
    b.emit("imm", one, 1)
    b.emit("alu", r, "add" if ic == "INC" else "sub", a, one, bits)
    b.emit("st", dst, r)
    b.emit("flags", ic.lower(), a, one, r, bits)


def neg(ctx: Ctx) -> None:
    """NEG: 0 - dest. SDM NEG: CF is 0 when the operand is 0 and 1 otherwise, OF SF ZF AF PF by the
    result: SUB's flags for 0 - dest, whose borrow is exactly that CF."""
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    dk, dst, bits = _operand(b, ops[0], nxt)
    zero, a, r = b.t(), _val(b, dk, dst), b.t()
    b.emit("imm", zero, 0)
    b.emit("alu", r, "sub", zero, a, bits)
    b.emit("st", dst, r)
    b.emit("flags", "sub", zero, a, r, bits)


def not_(ctx: Ctx) -> None:
    """NOT: one's complement; no flag changes (SDM NOT)."""
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    dk, dst, bits = _operand(b, ops[0], nxt)
    a, ones, r = _val(b, dk, dst), b.t(), b.t()
    b.emit("imm", ones, (1 << bits) - 1)
    b.emit("alu", r, "xor", a, ones, bits)
    b.emit("st", dst, r)


def cdqe(ctx: Ctx) -> None:
    """CDQE: RAX = sign-extended EAX (SDM CBW/CWDE/CDQE)."""
    b = ctx.b
    v, r = b.t(), b.t()
    b.emit("ld", v, ("r", RAX, 32, False))
    b.emit("alu", r, "sext32", v, v, 64)
    b.emit("st", ("r", RAX, 64, False), r)


def cqo(ctx: Ctx) -> None:
    """CQO: RDX = every bit RAX's sign (SDM CWD/CDQ/CQO; Sprint 046)."""
    b = ctx.b
    v, n, r = b.t(), b.t(), b.t()
    b.emit("ld", v, ("r", RAX, 64, False))
    b.emit("imm", n, 63)
    b.emit("alu", r, "sar", v, n, 64)
    b.emit("st", ("r", RDX, 64, False), r)


def cwde(ctx: Ctx) -> None:
    """CWDE: EAX = sign-extended AX; the 32-bit write zero-extends RAX (SDM CBW/CWDE/CDQE; Sprint 046)."""
    b = ctx.b
    v, r = b.t(), b.t()
    b.emit("ld", v, ("r", RAX, 16, False))
    b.emit("alu", r, "sext16", v, v, 32)
    b.emit("st", ("r", RAX, 32, False), r)


def cdq(ctx: Ctx) -> None:
    """CDQ: EDX = every bit EAX's sign (SDM CWD/CDQ/CQO); a 32-bit write zero-extends RDX."""
    b = ctx.b
    v, n, r = b.t(), b.t(), b.t()
    b.emit("ld", v, ("r", RAX, 32, False))
    b.emit("imm", n, 31)
    b.emit("alu", r, "sar", v, n, 32)
    b.emit("st", ("r", RDX, 32, False), r)


def movsx(ctx: Ctx) -> None:
    """MOVSX r, r/m8 or r/m16: sign-extended to the destination (SDM MOVSX/MOVSXD)."""
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    dk, dst, dbits = _operand(b, ops[0], nxt)
    sk, src, sbits = _operand(b, ops[1], nxt)
    if sbits not in (8, 16):
        raise LiftError(f"MOVSX from {sbits} bits")
    v, r = _val(b, sk, src), b.t()
    b.emit("alu", r, f"sext{sbits}", v, v, dbits)
    b.emit("st", dst, r)


def bswap(ctx: Ctx) -> None:
    """BSWAP r32/r64 (SDM BSWAP; the 16-bit form is undefined and not lifted)."""
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    dk, dst, bits = _operand(b, ops[0], nxt)
    if bits not in (32, 64):
        raise LiftError(f"BSWAP r{bits}: undefined (SDM)")
    v, r = _val(b, dk, dst), b.t()
    b.emit("alu", r, "bswap", v, v, bits)
    b.emit("st", dst, r)


def xchg(ctx: Ctx) -> None:
    """XCHG: the two operands swap; the memory store comes first (commit rule). A memory operand is
    locked by the processor (SDM XCHG); atomicity across vCPUs is judged in Sprint 047, with threads."""
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    ak, a_loc, bits = _operand(b, ops[0], nxt)
    bk, b_loc, _ = _operand(b, ops[1], nxt)
    a, c = _val(b, ak, a_loc), _val(b, bk, b_loc)
    first, second = (a_loc, b_loc) if a_loc[0] == "m" else (b_loc, a_loc)       # type: ignore[index]
    b.emit("st", first, c if first is a_loc else a)
    b.emit("st", second, a if second is b_loc else c)


def sbb(ctx: Ctx) -> None:
    """SBB: dest - src - CF (SDM SBB); flags as a subtraction with borrow."""
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    dk, dst, bits = _operand(b, ops[0], nxt)
    sk, src, _ = _operand(b, ops[1], nxt, bits)
    a, c, r = _val(b, dk, dst), _val(b, sk, src), b.t()
    b.emit("alu", r, "sbb", a, c, bits)
    b.emit("st", dst, r)
    b.emit("flags", "sbb", a, c, r, bits)


def adc(ctx: Ctx) -> None:
    """ADC: dest + src + CF (SDM ADC); flags as an addition with carry."""
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    dk, dst, bits = _operand(b, ops[0], nxt)
    sk, src, _ = _operand(b, ops[1], nxt, bits)
    a, c, r = _val(b, dk, dst), _val(b, sk, src), b.t()
    b.emit("alu", r, "adc", a, c, bits)
    b.emit("st", dst, r)
    b.emit("flags", "adc", a, c, r, bits)


def bit_scan(ctx: Ctx) -> None:
    """BSF, BSR (SDM): the index of the lowest (highest) set bit; ZF set and the destination not
    written when the source is 0 (the SDM: destination undefined; the CPU's cases decide what the
    model keeps); CF OF SF AF PF undefined."""
    b, ic, ops, nxt = ctx.b, ctx.ic, ctx.ops, ctx.nxt
    dk, dst, bits = _operand(b, ops[0], nxt)
    sk, src, _ = _operand(b, ops[1], nxt, bits)
    v, r = _val(b, sk, src), b.t()
    b.emit("alu", r, ic.lower(), v, v, bits)
    b.emit("cst", v, dst, r)
    b.emit("flags", "bsf", v, v, r, bits)


def bit_test(ctx: Ctx) -> None:
    """BT, BTS, BTR, BTC (SDM BT): CF = the selected bit; BTS sets it, BTR clears it, BTC complements it. A register or an
    immediate offset selects bit (offset mod operand size); a register offset with a memory base
    addresses the bit string: the operand at base + size * floor(offset / bits), signed, wrapping at
    32 bits under a 0x67 prefix. Every temporary is written once (the VM gives each write its own
    register)."""
    b, ic, ops, nxt = ctx.b, ctx.ic, ctx.ops, ctx.nxt
    dk, dst, bits = _operand(b, ops[0], nxt)
    ok, off_loc, obits = _operand(b, ops[1], nxt)
    off = _val(b, ok, off_loc)
    log = bits.bit_length() - 1                         # 16 -> 4, 32 -> 5, 64 -> 6
    if dst[0] == "m" and ok == "loc":                   # type: ignore[index]
        wide = off
        if obits != 64:
            wide = b.t()
            b.emit("alu", wide, "sext32" if obits == 32 else "sext16", off, off, 64)
        shift, words, step, scaled, addr = b.t(), b.t(), b.t(), b.t(), b.t()
        b.emit("imm", shift, log)
        b.emit("alu", words, "sar", wide, shift, 64)
        b.emit("imm", step, log - 3)
        b.emit("alu", scaled, "shl", words, step, 64)
        b.emit("alu", addr, "add", dst[1], scaled, 64)  # type: ignore[index]
        abits = next(op[6] for op in b.ops if op[0] == "addr" and op[1] == dst[1])    # type: ignore[index]
        if abits == 32:                                 # a 0x67 prefix: the address wraps at 32 bits
            low, wrapped = b.t(), b.t()
            b.emit("imm", low, 0xFFFF_FFFF)
            b.emit("alu", wrapped, "and", addr, low, 64)
            addr = wrapped
        dst = ("m", addr, dst[2])                       # type: ignore[index]
    mask, bit, shifted, one, cf = b.t(), b.t(), b.t(), b.t(), b.t()
    b.emit("imm", mask, bits - 1)
    b.emit("alu", bit, "and", off, mask, 64)
    value = _val(b, "loc", dst)
    b.emit("alu", shifted, "shr", value, bit, bits)
    b.emit("imm", one, 1)
    b.emit("alu", cf, "and", shifted, one, bits)
    if ic in ("BTS", "BTR", "BTC"):
        sel, result = b.t(), b.t()
        b.emit("alu", sel, "shl", one, bit, bits)
        if ic == "BTR":
            ones, notsel = b.t(), b.t()
            b.emit("imm", ones, (1 << bits) - 1)
            b.emit("alu", notsel, "xor", sel, ones, bits)
            b.emit("alu", result, "and", value, notsel, bits)
        else:
            b.emit("alu", result, "xor" if ic == "BTC" else "or", value, sel, bits)
        b.emit("st", dst, result)
    b.emit("flags", "bt", cf, cf, cf, bits)


def rep_string(ctx: Ctx) -> None:
    """REP STOS and REP MOVS with 64-bit addresses (SDM STOS, MOVS, REP)."""
    f = ctx.f
    if f is not None and 0x67 in f.prefixes:
        raise LiftError(f"{ctx.ic} with 32-bit addresses at 0x{ctx.address:x}")
    kind, size = REP_STRING[ctx.ic]
    ctx.b.emit("rep", kind, size)


def mxcsr(ctx: Ctx) -> None:
    """STMXCSR, LDMXCSR m32 (SDM), and their VEX forms VSTMXCSR, VLDMXCSR (the same; Sprint 049)."""
    _, loc, _ = _operand(ctx.b, ctx.ops[0], ctx.nxt)
    ctx.b.emit("mxcsr", "st" if ctx.ic.endswith("STMXCSR") else "ld", loc)


def andn(ctx: Ctx) -> None:
    """ANDN r, vvvv, r/m (SDM; BMI1): r = ~vvvv & r/m; SF ZF by the result, CF OF 0, AF PF undefined."""
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    _, dst, bits = _operand(b, ops[0], nxt)
    vk, vsrc, _ = _operand(b, ops[1], nxt)
    sk, src, _ = _operand(b, ops[2], nxt, bits)
    a, c, ones, inv, r = _val(b, vk, vsrc), _val(b, sk, src), b.t(), b.t(), b.t()
    b.emit("imm", ones, (1 << 64) - 1)
    b.emit("alu", inv, "xor", a, ones, bits)
    b.emit("alu", r, "and", inv, c, bits)
    b.emit("st", dst, r)
    b.emit("flags", "andn", inv, c, r, bits)


def shift_x(ctx: Ctx) -> None:
    """SHLX, SHRX r, r/m, vvvv (SDM; BMI2): the count is vvvv masked to 5 or 6 bits; no flag changes."""
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    _, dst, bits = _operand(b, ops[0], nxt)
    sk, src, _ = _operand(b, ops[1], nxt, bits)
    ck, cnt, _ = _operand(b, ops[2], nxt)
    a, c, r = _val(b, sk, src), _val(b, ck, cnt), b.t()
    b.emit("alu", r, "shl" if ctx.ic == "SHLX" else "shr", a, c, bits)
    b.emit("st", dst, r)


def rorx(ctx: Ctx) -> None:
    """RORX r, r/m, imm8 (SDM; BMI2): rotate right by imm8 masked to 5 or 6 bits; no flag changes."""
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    _, dst, bits = _operand(b, ops[0], nxt)
    sk, src, _ = _operand(b, ops[1], nxt, bits)
    ik, count, _ = _operand(b, ops[2], nxt, bits)
    a, c, r = _val(b, sk, src), _val(b, ik, count), b.t()
    b.emit("alu", r, "ror", a, c, bits)
    b.emit("st", dst, r)


def mulx(ctx: Ctx) -> None:
    """MULX r_hi, r_lo(vvvv), r/m (SDM; BMI2): RDX (EDX) * r/m unsigned; the low half to vvvv first, then the
    high half to ModRM.reg, so the high half wins when both name one register; no flag changes."""
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    _, hi_dst, bits = _operand(b, ops[0], nxt)
    _, lo_dst, _ = _operand(b, ops[1], nxt)
    sk, src, _ = _operand(b, ops[2], nxt, bits)
    rdx, c = b.t(), _val(b, sk, src)
    b.emit("ld", rdx, ("r", 2, bits, False))
    lo, hi = b.t(), b.t()
    if bits == 64:
        b.emit("alu", lo, "mul_lo", rdx, c, 64)
        b.emit("alu", hi, "mul_hi", rdx, c, 64)
    else:                                                 # 32 x 32 bits: the 64-bit product, split
        k = b.t()
        b.emit("alu", lo, "mul_lo", rdx, c, 64)
        b.emit("imm", k, 32)
        b.emit("alu", hi, "shr", lo, k, 64)
    b.emit("st", lo_dst, lo)
    b.emit("st", hi_dst, hi)


def fpu_control(ctx: Ctx) -> None:
    """FNSTCW m2byte (SDM FSTCW/FNSTCW; Sprint 049): the x87 control word to memory, with no check for pending
    x87 exceptions (the N form); w2m runs no x87 arithmetic, so none is ever pending."""
    _, loc, _ = _operand(ctx.b, ctx.ops[0], ctx.nxt)
    ctx.b.emit("fcw", "st", loc)


def sfence(ctx: Ctx) -> None:
    """SFENCE, LFENCE, MFENCE (SDM): the ordering each gives, as one fence kind (Sprint 047)."""
    ctx.b.emit("fence", {"SFENCE": "st", "LFENCE": "ld", "MFENCE": "full"}[ctx.ic])


def undefined_by_profile(ctx: Ctx) -> None:
    """An instruction the guest profile makes #UD (none since Sprint 049; XGETBV was one while OSXSAVE was clear)."""
    ctx.b.emit("ud")


def xgetbv(ctx: Ctx) -> None:
    """XGETBV (SDM): EDX:EAX = XCR[ECX]. The profile reports OSXSAVE and XCR0 = x87 | SSE | AVX (7) (Sprint 049,
    specs/guest_cpuid.json 0.2.0); XGETBV1 is not reported, so any ECX but 0 is #GP."""
    ctx.b.emit("xgetbv")


def div(ctx: Ctx) -> None:
    """DIV r/m (unsigned; SDM DIV)."""
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    sk, src, bits = _operand(b, ops[0], nxt)
    b.emit("div", bits, _val(b, sk, src))


def idiv(ctx: Ctx) -> None:
    """IDIV r/m (signed; SDM IDIV; Sprint 046)."""
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    sk, src, bits = _operand(b, ops[0], nxt)
    b.emit("div", bits, _val(b, sk, src), True)


def shr(ctx: Ctx) -> None:
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    dk, dst, bits = _operand(b, ops[0], nxt)
    sk, src, _ = _operand(b, ops[1], nxt)
    a, r = _val(b, dk, dst), b.t()
    cnt = _val(b, sk, src)
    b.emit("alu", r, "shr", a, cnt, bits)
    b.emit("st", dst, r)
    b.emit("flags", "shr", a, cnt, r, bits)


def mul(ctx: Ctx) -> None:
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    sk, src, bits = _operand(b, ops[0], nxt)
    if bits != 64:                  # SDM MUL: AX = AL * r/m8, DX:AX = AX * r/m16, EDX:EAX = EAX * r/m32 (Sprint 047):
        a, c, p, k, hi = b.t(), _val(b, sk, src), b.t(), b.t(), b.t()     # the exact product in 64 bits, split
        b.emit("ld", a, ("r", RAX, bits, False))
        b.emit("alu", p, "mul_lo", a, c, 64)
        b.emit("imm", k, bits)
        b.emit("alu", hi, "shr", p, k, 64)
        b.emit("flags", "mul", a, c, hi, bits)
        if bits == 8:
            b.emit("st", ("r", RAX, 16, False), p)
        else:
            b.emit("st", ("r", RAX, bits, False), p)
            b.emit("st", ("r", RDX, bits, False), hi)
        return
    a, c, lo, hi = b.t(), _val(b, sk, src), b.t(), b.t()
    b.emit("ld", a, ("r", RAX, 64, False))
    b.emit("alu", lo, "mul_lo", a, c, 64)
    b.emit("alu", hi, "mul_hi", a, c, 64)
    b.emit("flags", "mul", a, c, hi, 64)
    b.emit("st", ("r", RAX, 64, False), lo)
    b.emit("st", ("r", RDX, 64, False), hi)


def setcc(ctx: Ctx) -> None:
    _, dst, _ = _operand(ctx.b, ctx.ops[0], ctx.nxt)
    ctx.b.emit("set", CC[ctx.ic[3:]], dst)


def jcc(ctx: Ctx) -> None:
    _, t, _ = _operand(ctx.b, ctx.ops[0], ctx.nxt)
    ctx.b.emit("br", CC[ctx.ic[1:]], t)


def jmp_call(ctx: Ctx) -> None:
    b, ic, nxt = ctx.b, ctx.ic, ctx.nxt
    k, x, _ = _operand(b, ctx.ops[0], nxt)
    target = _val(b, k, x)
    if ic == "CALL_NEAR":
        ret = b.t()
        b.emit("imm", ret, nxt)
        _push(b, ret)
    b.emit("br", "always", target)


def push(ctx: Ctx) -> None:
    # operand size 64 by default, 16 with 66 (SDM PUSH: "decrements the stack pointer by 2" for a
    # 16-bit operand); the semantics oracle found a 66 PUSH lifted as 8 bytes (Sprint 037)
    b, f = ctx.b, ctx.f
    size = 2 if f is not None and f.opsize == 16 else 8
    k, x, bits = _operand(b, ctx.ops[0], ctx.nxt, 8 * size)
    if bits not in (8, 16, 32, 64) or (k == "loc" and bits != 8 * size):
        raise LiftError(f"PUSH {bits}-bit")
    _push(b, _val(b, k, x), size)


def pop(ctx: Ctx) -> None:
    b = ctx.b
    _, pop_dst, bits = _operand(b, ctx.ops[0], ctx.nxt)
    dst = cast(tuple[Any, ...], pop_dst)
    if bits not in (16, 64):
        raise LiftError(f"POP {bits}-bit")
    size = bits // 8                                       # 66 POP: RSP += 2
    if dst[0] == "m":
        # SDM POP: "If the ESP register is used as a base register for addressing a destination
        # operand in memory, the POP instruction computes the effective address of the operand
        # after it increments the ESP register." The address was computed from the old RSP; the
        # new one is `size` higher.
        for i, op in enumerate(b.ops):
            if op[0] == "addr" and op[1] == dst[1] and op[2] == RSP:
                b.ops[i] = op[:5] + (op[5] + size,) + op[6:]
    sp, step, nsp, v = b.t(), b.t(), b.t(), b.t()
    b.emit("ld", sp, ("r", RSP, 64, False))
    b.emit("ld", v, ("m", sp, size))
    b.emit("imm", step, size)
    b.emit("alu", nsp, "add", sp, step, 64)
    if dst[0] == "m":
        b.emit("st", dst, v)                               # the store before RSP moves: the commit rule
        b.emit("st", ("r", RSP, 64, False), nsp)
    else:
        b.emit("st", ("r", RSP, 64, False), nsp)           # POP RSP: the popped value wins (SDM)
        b.emit("st", dst, v)


def _count(ctx: Ctx, bits: int) -> tuple[str, Any]:
    """A shift's count operand: CL or an immediate; the D0/D1 forms carry an implicit 1."""
    if len(ctx.ops) > 1:
        k, x, _ = _operand(ctx.b, ctx.ops[1], ctx.nxt)
        return k, x
    one = ctx.b.t()
    ctx.b.emit("imm", one, 1)
    return "imm", one


def shift(ctx: Ctx) -> None:
    """SHL (and SAL), SAR, ROL, ROR: the result, stored before flags (commit rule)."""
    b, ic = ctx.b, ctx.ic
    op = {"SHL": "shl", "SAR": "sar", "ROL": "rol", "ROR": "ror"}[ic]
    dk, dst, bits = _operand(b, ctx.ops[0], ctx.nxt)
    a = _val(b, dk, dst)
    ck, cx = _count(ctx, bits)
    cnt = _val(b, ck, cx)
    r = b.t()
    b.emit("alu", r, op, a, cnt, bits)
    b.emit("st", dst, r)
    b.emit("flags", op, a, cnt, r, bits)


def imul(ctx: Ctx) -> None:
    """SDM IMUL: one operand (AX, DX:AX, EDX:EAX or RDX:RAX = accumulator * r/m), two (r = r * r/m)
    or three (r = r/m * imm); CF and OF on a truncated signed product."""
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    if len(ops) == 1:
        sk, src, bits = _operand(b, ops[0], nxt)
        a, c = b.t(), _val(b, sk, src)
        b.emit("ld", a, ("r", RAX, bits, False))
        lo, hi = b.t(), b.t()
        b.emit("alu", lo, "smul_lo", a, c, bits)
        b.emit("alu", hi, "smul_hi", a, c, bits)
        if bits == 8:                                      # AX = AL * r/m8
            b.emit("st", ("r", RAX, 8, False), lo)
            b.emit("st", ("r", RAX, 8, True), hi)
        else:
            b.emit("st", ("r", RAX, bits, False), lo)
            b.emit("st", ("r", RDX, bits, False), hi)
        b.emit("flags", "imul", a, c, lo, bits)
        return
    dk, dst, bits = _operand(b, ops[0], nxt)
    if len(ops) == 2:
        sk, src, _ = _operand(b, ops[1], nxt, bits)
        a, c = _val(b, dk, dst), _val(b, sk, src)
    else:
        sk, src, _ = _operand(b, ops[1], nxt, bits)
        ik, imm, _ = _operand(b, ops[2], nxt, bits)
        a, c = _val(b, sk, src), _val(b, ik, imm)
    r = b.t()
    b.emit("alu", r, "smul_lo", a, c, bits)
    b.emit("st", dst, r)
    b.emit("flags", "imul", a, c, r, bits)


def cmov(ctx: Ctx) -> None:
    """SDM CMOVcc: the source is read whether or not the condition holds."""
    b = ctx.b
    _, dst, bits = _operand(b, ctx.ops[0], ctx.nxt)
    sk, src, _ = _operand(b, ctx.ops[1], ctx.nxt, bits)
    b.emit("cmov", CC[ctx.ic[4:]], dst, _val(b, sk, src))


def xadd(ctx: Ctx) -> None:
    """SDM XADD: TEMP = SRC + DEST; SRC = DEST; DEST = TEMP. A memory destination is stored first
    (commit rule); with two registers the destination is written last, as the SDM orders it."""
    b = ctx.b
    dk, dst, bits = _operand(b, ctx.ops[0], ctx.nxt)
    sk, src, _ = _operand(b, ctx.ops[1], ctx.nxt)
    a, c, r = _val(b, dk, dst), _val(b, sk, src), b.t()
    b.emit("alu", r, "add", a, c, bits)
    if cast(tuple[Any, ...], dst)[0] == "m":
        b.emit("st", dst, r)
        b.emit("st", src, a)
    else:
        b.emit("st", src, a)
        b.emit("st", dst, r)
    b.emit("flags", "add", a, c, r, bits)


def cmpxchg(ctx: Ctx) -> None:
    """SDM CMPXCHG: compare the accumulator with DEST; equal: DEST = SRC; else the accumulator =
    DEST; flags as CMP accumulator, DEST. A memory DEST is always written ("never produces a locked
    read without also producing a locked write"). A register DEST is written only when equal: on
    Intel a 32-bit register DEST keeps its upper half when the values differ (12 cases of case set
    041, Xeon Platinum 8573C run 38029980267), so there is no write to zero-extend it."""
    b = ctx.b
    dk, dst, bits = _operand(b, ctx.ops[0], ctx.nxt)
    sk, src, _ = _operand(b, ctx.ops[1], ctx.nxt)
    acc = ("r", RAX, bits, False)
    a, d, s = b.t(), _val(b, dk, dst), _val(b, sk, src)
    b.emit("ld", a, acc)
    r = b.t()
    b.emit("alu", r, "sub", a, d, bits)                    # nonzero exactly when they differ
    if cast(tuple[Any, ...], dst)[0] == "m":
        new = b.t()
        b.emit("sel", new, r, d, s)                        # differ: DEST written back; equal: SRC
        b.emit("st", dst, new)
    else:
        b.emit("cstz", r, dst, s)                          # equal: DEST = SRC
    b.emit("cst", r, acc, d)                               # differ: the accumulator gets DEST
    b.emit("flags", "sub", a, d, r, bits)


def cmpxchg16b(ctx: Ctx) -> None:
    """CMPXCHG16B m128 (SDM CMPXCHG8B/CMPXCHG16B; Sprint 046): RDX:RAX against the 16 bytes; equal: ZF set and
    RCX:RBX stored; else ZF clear, RDX:RAX loaded and the old value written back (the destination is always
    written); other flags unchanged; #GP unless 16-byte aligned. One op ("cx16", address): atomic in the VM
    with or without LOCK. CMPXCHG8B (no REX.W) is not lifted."""
    b = ctx.b
    _, loc, bits = _operand(b, ctx.ops[0], ctx.nxt)
    if bits != 128:
        raise LiftError(f"CMPXCHG8B at 0x{ctx.address:x}")
    b.emit("cx16", cast(tuple[Any, ...], loc)[1])


def locked(base: Callable[[Ctx], None]) -> Callable[[Ctx], None]:
    """A LOCK-prefixed form (iclass X_LOCK): the base rule's operations under the base iclass. One
    thread sees the same result; the VM lowers the locked forms atomically (rules/<x>_lock.py)."""
    def rule(ctx: Ctx) -> None:
        ctx.ic = ctx.ic.removesuffix("_LOCK")
        base(ctx)
    return rule


def _model(ctx: Ctx, dst: Any, srcs: tuple[int, ...], bits: int) -> None:
    """One instruction the model computes whole (Sprint 050; integer_model.py): the VM runs it as a model step."""
    ctx.b.emit("x86", ctx.ic, dst, srcs, bits)


def double_shift(ctx: Ctx) -> None:
    """SHLD, SHRD r/m, r, imm8 or CL (SDM SHLD/SHRD)."""
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    dk, dst, bits = _operand(b, ops[0], nxt)
    sk, src, _ = _operand(b, ops[1], nxt, bits)
    ck, cnt, _ = _operand(b, ops[2], nxt)
    _model(ctx, dst, (_val(b, dk, dst), _val(b, sk, src), _val(b, ck, cnt)), bits)


def rcr(ctx: Ctx) -> None:
    """RCR r/m, 1, CL or imm8 (SDM RCL/RCR/ROL/ROR): rotate right through CF."""
    b = ctx.b
    dk, dst, bits = _operand(b, ctx.ops[0], ctx.nxt)
    ck, cx = _count(ctx, bits)
    _model(ctx, dst, (_val(b, dk, dst), _val(b, ck, cx)), bits)


def implicit(ctx: Ctx) -> None:
    """STC, CLC, CMC, SAHF, PUSHFQ, POPFQ, and the single string forms (SDM, each page): every operand implicit. A
    0x67 prefix on a string form (32-bit RSI and RDI) or a 66 prefix on PUSHF and POPF is not lifted; on the flag
    instructions both prefixes change nothing (UnityPlayer has a 67 STC)."""
    f = ctx.f
    strings = ctx.ic not in ("STC", "CLC", "CMC", "SAHF", "PUSHFQ", "POPFQ")
    if f is not None and ((0x67 in f.prefixes and strings) or (0x66 in f.prefixes and ctx.ic in ("PUSHFQ", "POPFQ"))):
        raise LiftError(f"{ctx.ic} with a 66 or 67 prefix at 0x{ctx.address:x}")
    _model(ctx, None, (), 64)


def popcnt(ctx: Ctx) -> None:
    """POPCNT r, r/m (SDM POPCNT; reported by the guest profile)."""
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    _, dst, bits = _operand(b, ops[0], nxt)
    sk, src, _ = _operand(b, ops[1], nxt, bits)
    _model(ctx, dst, (_val(b, sk, src),), bits)


def loop(ctx: Ctx) -> None:
    """LOOP, LOOPE, LOOPNE, JRCXZ rel8 (SDM LOOPcc, Jcc): RCX in 64-bit mode; ECX (a 0x67 prefix) is not lifted."""
    f = ctx.f
    if f is not None and 0x67 in f.prefixes:
        raise LiftError(f"{ctx.ic} on ECX at 0x{ctx.address:x}")
    _, target, _ = _operand(ctx.b, ctx.ops[0], ctx.nxt)
    _model(ctx, None, (cast(int, target),), 64)


def leave(ctx: Ctx) -> None:
    """LEAVE (SDM LEAVE): RSP = RBP, then RBP = the popped quadword; the load first (commit rule). A 66 prefix
    (16-bit LEAVE) is not lifted."""
    b, f = ctx.b, ctx.f
    if f is not None and f.opsize == 16:
        raise LiftError(f"LEAVE with 16-bit operands at 0x{ctx.address:x}")
    rbp, v, eight, nsp = b.t(), b.t(), b.t(), b.t()
    b.emit("ld", rbp, ("r", 5, 64, False))
    b.emit("ld", v, ("m", rbp, 8))
    b.emit("imm", eight, 8)
    b.emit("alu", nsp, "add", rbp, eight, 64)
    b.emit("st", ("r", RSP, 64, False), nsp)
    b.emit("st", ("r", 5, 64, False), v)


def movbe(ctx: Ctx) -> None:
    """MOVBE r, m and m, r (SDM MOVBE): the bytes reversed on the way; a register in the memory operand's place
    is #UD. 16 bits: the two bytes swapped (a rotate by 8)."""
    b, ops, nxt = ctx.b, ctx.ops, ctx.nxt
    memory = 1 if ctx.f is not None and ctx.f.opcode == 0xF0 else 0
    if not ops[memory].startswith("M:"):
        b.emit("ud")
        return
    dk, dst, bits = _operand(b, ops[0], nxt)
    sk, src, _ = _operand(b, ops[1], nxt, bits)
    v, r = _val(b, sk, src), b.t()
    if bits == 16:
        eight = b.t()
        b.emit("imm", eight, 8)
        b.emit("alu", r, "rol", v, eight, 16)
    else:
        b.emit("alu", r, "bswap", v, v, bits)
    b.emit("st", dst, r)
