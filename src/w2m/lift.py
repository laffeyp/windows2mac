"""lift.py — x86-64 instructions to explicit operations (v0.2 §5 machine-semantics IR; Sprint 029).

Each decoded instruction (src/w2m/x86dec.py) with its operands (src/w2m/x86ops.py, equal to
XED's on every compared corpus position) becomes a short list of operations over numbered
temporaries. The operations are target-independent: `runtime.py` executes them on the host
(profile `model`); an AArch64 back end for the Hypervisor.framework VM consumes the same list.

Operations (t* are temporaries, loc is ("r", index, bits, high8) or ("m", t_addr, bytes)):
  ("addr", t, base, index, scale, disp, abits)   effective address; base/index are register
                                                  indices, "rip" resolved at lift time, or None
  ("imm", t, value)        ("ld", t, loc)        ("st", loc, t)
  ("alu", t, op, a, b, bits)      op: add sub and or xor shr mul_lo mul_hi sext32
  ("flags", kind, a, b, r, bits)  kind: add sub logic inc dec shr mul (SDM Vol 2 flag tables)
  ("br", cond, t)                 cond: always or an SDM condition code (Vol 1 B.1)
  ("set", cond, loc)              SETcc
Semantics follow Intel SDM Vol 2 instruction pages; a 32-bit register write zero-extends to 64
bits and 8/16-bit writes keep the other bits (SDM Vol 1 3.4.1.1).
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from w2m.x86dec import Decoded, decode
from w2m.x86ops import GPR8_LEGACY, GPR8_REX, GPR16, GPR32, GPR64, operands

REG: dict[str, tuple[int, int, bool]] = {}
for _i, _n in enumerate(GPR64):
    REG[_n] = (_i, 64, False)
for _i, _n in enumerate(GPR32):
    REG[_n] = (_i, 32, False)
for _i, _n in enumerate(GPR16):
    REG[_n] = (_i, 16, False)
for _i, _n in enumerate(GPR8_REX):
    REG[_n] = (_i, 8, False)
for _i, _n in enumerate(GPR8_LEGACY[4:]):
    REG[_n] = (_i, 8, True)                  # AH, CH, DH, BH: bits 8-15 of registers 0-3
RSP, RAX, RDX = 4, 0, 2
CC = {"O": "o", "NO": "no", "B": "b", "NB": "nb", "Z": "z", "NZ": "nz", "BE": "be", "NBE": "nbe",
      "S": "s", "NS": "ns", "P": "p", "NP": "np", "L": "l", "NL": "nl", "LE": "le", "NLE": "nle"}
ALU = {"ADD": "add", "SUB": "sub", "AND": "and", "OR": "or", "XOR": "xor", "CMP": "sub", "TEST": "and"}

Op = tuple[Any, ...]


class LiftError(ValueError):
    """The instruction decodes but has no lifting rule yet; the runtime stops on it."""


@dataclass(frozen=True)
class Lifted:
    address: int
    length: int
    iclass: str
    ops: tuple[Op, ...]


class _OpBuilder:
    def __init__(self) -> None:
        self.ops: list[Op] = []
        self.n = 0

    def t(self) -> int:
        self.n += 1
        return self.n

    def emit(self, *op: object) -> None:
        self.ops.append(op)


def _operand(b: _OpBuilder, s: str, next_ip: int, width_hint: int | None = None) -> tuple[str, object, int]:
    """Parse one xedshim-format operand; return (kind, loc-or-temp, bits)."""
    p = s.split(":")
    if p[0] == "R":
        idx, bits, hi = REG[p[1]]
        return "loc", ("r", idx, bits, hi), bits
    if p[0] in ("M", "A"):
        base, index, scale, disp = (p[2], p[3], p[4], p[5]) if p[0] == "M" else (p[1], p[2], p[3], p[4])
        t = b.t()
        abits = 32 if base.startswith("E") or (base == "INVALID" and index.startswith("E")) else 64
        if base in ("RIP", "EIP"):
            b.emit("addr", t, None, None, 1, next_ip + int(disp), abits)
        else:
            b.emit("addr", t, None if base == "INVALID" else REG[base][0],
                   None if index == "INVALID" else REG[index][0], int(scale), int(disp), abits)
        if p[0] == "A":
            return "addr", t, abits
        return "loc", ("m", t, int(p[6])), int(p[6]) * 8
    if p[0] == "I":
        width, raw = int(p[1]) * 8, int(p[2], 16)
        t = b.t()
        target = width_hint or width
        if width < target and raw >> (width - 1) & 1:          # imm sign-extends to operand size
            raw |= ((1 << target) - 1) ^ ((1 << width) - 1)
        b.emit("imm", t, raw)
        return "imm", t, width
    if p[0] == "J":
        t = b.t()
        b.emit("imm", t, (next_ip + int(p[1])) & (1 << 64) - 1)
        return "imm", t, 64
    raise LiftError(f"operand {s}")


def _val(b: _OpBuilder, kind: str, x: object) -> int:
    if kind == "loc":
        t = b.t()
        b.emit("ld", t, x)
        return t
    assert isinstance(x, int)
    return x


def _push(b: _OpBuilder, value_t: int) -> None:
    sp, eight, nsp = b.t(), b.t(), b.t()
    b.emit("ld", sp, ("r", RSP, 64, False))
    b.emit("imm", eight, 8)
    b.emit("alu", nsp, "sub", sp, eight, 64)
    b.emit("st", ("r", RSP, 64, False), nsp)
    b.emit("st", ("m", nsp, 8), value_t)


def _pop(b: _OpBuilder) -> int:
    sp, eight, nsp, v = b.t(), b.t(), b.t(), b.t()
    b.emit("ld", sp, ("r", RSP, 64, False))
    b.emit("ld", v, ("m", sp, 8))
    b.emit("imm", eight, 8)
    b.emit("alu", nsp, "add", sp, eight, 64)
    b.emit("st", ("r", RSP, 64, False), nsp)
    return v


def lift(code: bytes, address: int) -> Lifted:
    """Lift the instruction at the start of `code`, located at guest `address`."""
    d: Decoded = decode(code)
    f = d.fields
    ic = d.iclass
    nxt = address + d.length
    b = _OpBuilder()
    ops = operands(f) if f is not None and f.ops else ()
    if ic == "NOP":
        pass
    elif ic == "RET_NEAR" and not ops:
        b.emit("br", "always", _pop(b))
    elif ic in ("MOV", "MOVZX", "MOVSXD", "LEA"):
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
    elif ic in ALU:
        dk, dst, bits = _operand(b, ops[0], nxt)
        sk, src, _ = _operand(b, ops[1], nxt, bits)
        a, c, r = _val(b, dk, dst), _val(b, sk, src), b.t()
        b.emit("alu", r, ALU[ic], a, c, bits)
        b.emit("flags", "logic" if ALU[ic] in ("and", "or", "xor") else ALU[ic], a, c, r, bits)
        if ic not in ("CMP", "TEST"):
            b.emit("st", dst, r)
    elif ic in ("INC", "DEC"):
        dk, dst, bits = _operand(b, ops[0], nxt)
        a, one, r = _val(b, dk, dst), b.t(), b.t()
        b.emit("imm", one, 1)
        b.emit("alu", r, "add" if ic == "INC" else "sub", a, one, bits)
        b.emit("flags", ic.lower(), a, one, r, bits)
        b.emit("st", dst, r)
    elif ic == "SHR":
        dk, dst, bits = _operand(b, ops[0], nxt)
        sk, src, _ = _operand(b, ops[1], nxt)
        a, r = _val(b, dk, dst), b.t()
        cnt = _val(b, sk, src)
        b.emit("alu", r, "shr", a, cnt, bits)
        b.emit("flags", "shr", a, cnt, r, bits)
        b.emit("st", dst, r)
    elif ic == "MUL":
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
    elif ic.startswith("SET") and ic[3:] in CC:
        _, dst, _ = _operand(b, ops[0], nxt)
        b.emit("set", CC[ic[3:]], dst)
    elif ic.startswith("J") and ic[1:] in CC:
        _, t, _ = _operand(b, ops[0], nxt)
        b.emit("br", CC[ic[1:]], t)
    elif ic in ("JMP", "CALL_NEAR"):
        k, x, _ = _operand(b, ops[0], nxt)
        target = _val(b, k, x)
        if ic == "CALL_NEAR":
            ret = b.t()
            b.emit("imm", ret, nxt)
            _push(b, ret)
        b.emit("br", "always", target)
    elif ic == "PUSH":
        k, x, bits = _operand(b, ops[0], nxt, 64)
        if bits not in (8, 32, 64) or (k == "loc" and bits != 64):
            raise LiftError(f"PUSH {bits}-bit")
        _push(b, _val(b, k, x))
    elif ic == "POP":
        _, dst, bits = _operand(b, ops[0], nxt)
        if bits != 64:
            raise LiftError(f"POP {bits}-bit")
        b.emit("st", dst, _pop(b))
    else:
        raise LiftError(f"no lifting rule for {ic} at 0x{address:x}")
    return Lifted(address, d.length, ic, tuple(b.ops))
