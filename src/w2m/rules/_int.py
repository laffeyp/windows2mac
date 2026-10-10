"""Shared integer and control-flow rules (moved unchanged from lift.py in Sprint 041). Semantics
follow the Intel SDM Vol 2 page of each instruction; the commit rule (specs/guest_state.json
0.2.0) orders every register and flag write after the last memory access."""

from __future__ import annotations

from collections.abc import Callable
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


def locked(base: Callable[[Ctx], None]) -> Callable[[Ctx], None]:
    """A LOCK-prefixed form (iclass X_LOCK): the base rule's operations under the base iclass. One
    thread sees the same result; the VM lowers the locked forms atomically (rules/<x>_lock.py)."""
    def rule(ctx: Ctx) -> None:
        ctx.ic = ctx.ic.removesuffix("_LOCK")
        base(ctx)
    return rule
