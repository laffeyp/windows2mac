"""Shared integer and control-flow rules (moved unchanged from lift.py in Sprint 041). Semantics
follow the Intel SDM Vol 2 page of each instruction; the commit rule (specs/guest_state.json
0.2.0) orders every register and flag write after the last memory access."""

from __future__ import annotations

from typing import Any, cast

from w2m.lift import ALU, CC, RAX, RDX, RSP, LiftError, _operand, _pop, _push, _val
from w2m.rules import Ctx


def nop(ctx: Ctx) -> None:
    pass


def df(ctx: Ctx) -> None:
    ctx.b.emit("df", int(ctx.ic == "STD"))


def cpuid(ctx: Ctx) -> None:
    ctx.b.emit("cpuid")                                  # EAX, EBX, ECX, EDX from the guest profile


def ret_near(ctx: Ctx) -> None:
    if ctx.ops:
        raise LiftError(f"RET_NEAR imm16 at 0x{ctx.address:x}")
    ctx.b.emit("br", "always", _pop(ctx.b))


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
    if bits != 64:
        raise LiftError(f"MUL r/m{bits}")
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
