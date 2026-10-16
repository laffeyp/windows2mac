# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""symbolic_lift.py — a lifting rule's ops as a template, by running the rule on symbols (Sprint 089; Decision
2026-10-16, the compiler as per-op lowering from symbolic templates).

`lift()` turns one instruction into ops; its rule (src/w2m/rules) reads the operands' numbers (register numbers, a
displacement, an immediate, the next address) and a few decoder fields. Here the numbers are symbols (`Sym`): what a
rule computes from them is an expression (`Expr`), which the template keeps in place of a number, so its holes are
known by construction. Where a rule branches on a symbol, the run forks and each template carries the condition it
took (`Template.conditions`); where it needs a symbol as a plain integer (an index, a range), the symbol is pinned to
the sample instruction's value and joins the key (`Template.pins`). The decoder fields a rule reads join the key too
(`Template.fields`). The rules are not changed: `_operand` and the two places that parse an immediate themselves
(`_sse`, `_vex`) are replaced while a symbolic run lasts.

A template is keyed by the instruction class, its operand shape (`shape`: each operand's kind and width, a memory
operand's segment, base and index kinds and size), and the mode (64-bit until the decoder has a 32-bit mode).
`fill(template, env)` evaluates its ops on an instruction's values (`values`); `lifted_by_template` judges it against
`lift()`.
"""

from __future__ import annotations

import builtins
from collections.abc import Callable, Iterator
from contextlib import contextmanager
from dataclasses import dataclass
from typing import Any, cast

from w2m import lift as lift_module
from w2m.lift import IMPLICIT_050, MMX_ONLY, REG, REP_STRING, LiftError, _OpBuilder, lift_mmx
from w2m.x86dec import DecodeError, decode
from w2m.x86ops import operands

MODE = 64
M64 = (1 << 64) - 1


class Expr:
    """A value a rule computed from symbols: op and arguments (Expr or int)."""

    __slots__ = ("op", "args")

    def __init__(self, op: str, *args: Any) -> None:
        self.op = op
        self.args = args

    def __repr__(self) -> str:
        return f"{self.op}({', '.join(map(repr, self.args))})"

    def key(self) -> tuple[Any, ...]:
        return (self.op, *(a.key() if isinstance(a, Expr) else a for a in self.args))

    def eval(self, env: dict[str, int]) -> int:
        a = [x.eval(env) if isinstance(x, Expr) else x for x in self.args]
        return int(_OPS[self.op](*a))

    # arithmetic builds expressions
    def __add__(self, o: Any) -> Expr: return Expr("add", self, o)
    def __radd__(self, o: Any) -> Expr: return Expr("add", o, self)
    def __sub__(self, o: Any) -> Expr: return Expr("sub", self, o)
    def __rsub__(self, o: Any) -> Expr: return Expr("sub", o, self)
    def __mul__(self, o: Any) -> Expr: return Expr("mul", self, o)
    def __rmul__(self, o: Any) -> Expr: return Expr("mul", o, self)
    def __and__(self, o: Any) -> Expr: return Expr("and", self, o)
    def __rand__(self, o: Any) -> Expr: return Expr("and", o, self)
    def __or__(self, o: Any) -> Expr: return Expr("or", self, o)
    def __ror__(self, o: Any) -> Expr: return Expr("or", o, self)
    def __xor__(self, o: Any) -> Expr: return Expr("xor", self, o)
    def __rxor__(self, o: Any) -> Expr: return Expr("xor", o, self)
    def __lshift__(self, o: Any) -> Expr: return Expr("shl", self, o)
    def __rshift__(self, o: Any) -> Expr: return Expr("shr", self, o)
    def __mod__(self, o: Any) -> Expr: return Expr("mod", self, o)
    def __floordiv__(self, o: Any) -> Expr: return Expr("div", self, o)
    def __neg__(self) -> Expr: return Expr("neg", self)
    def __invert__(self) -> Expr: return Expr("not", self)

    # a test on a symbol forks the run; a plain integer pins it
    def __eq__(self, o: object) -> bool: return _decide(Expr("eq", self, o))
    def __ne__(self, o: object) -> bool: return _decide(Expr("ne", self, o))
    def __lt__(self, o: Any) -> bool: return _decide(Expr("lt", self, o))
    def __le__(self, o: Any) -> bool: return _decide(Expr("le", self, o))
    def __gt__(self, o: Any) -> bool: return _decide(Expr("gt", self, o))
    def __ge__(self, o: Any) -> bool: return _decide(Expr("ge", self, o))
    def __bool__(self) -> bool: return _decide(Expr("ne", self, 0))
    def bit_length(self) -> Expr: return Expr("bitlen", self)
    def __index__(self) -> int: return _pin(self)
    def __int__(self) -> int: return _pin(self)
    def __hash__(self) -> int: return hash(self.key())


class Sym(Expr):
    """A number of the instruction: a register number, a displacement, an immediate, the next address."""

    def __init__(self, name: str) -> None:
        super().__init__("sym", name)

    def eval(self, env: dict[str, int]) -> int:
        return env[self.args[0]]


_OPS: dict[str, Callable[..., int]] = {
    "add": lambda a, b: a + b, "sub": lambda a, b: a - b, "mul": lambda a, b: a * b, "and": lambda a, b: a & b,
    "or": lambda a, b: a | b, "xor": lambda a, b: a ^ b, "shl": lambda a, b: a << b, "shr": lambda a, b: a >> b,
    "mod": lambda a, b: a % b, "div": lambda a, b: a // b, "neg": lambda a: -a, "not": lambda a: ~a,
    "eq": lambda a, b: a == b, "ne": lambda a, b: a != b, "lt": lambda a, b: a < b, "le": lambda a, b: a <= b,
    "gt": lambda a, b: a > b, "ge": lambda a, b: a >= b, "bitlen": lambda a: a.bit_length(),
}
Interval = tuple[int, int] | None


def interval(x: Any, domains: dict[str, tuple[int, int]]) -> Interval:
    """Bounds of x over the symbols' domains (inclusive), or None where they are not known: interval arithmetic, so a
    test the domains decide needs no fork (Sprint 090: a register number 0-15 passes an encoder's range check)."""
    if isinstance(x, bool):
        return (int(x), int(x))
    if isinstance(x, int):
        return (x, x)
    if not isinstance(x, Expr):
        return None
    if isinstance(x, Sym):
        return domains.get(x.args[0])
    a = [interval(y, domains) for y in x.args]
    if any(v is None for v in a):
        if x.op == "and":                                    # a known nonnegative mask bounds the result
            known = [v for v in a if v is not None and v[0] >= 0]
            return (0, min(v[1] for v in known)) if known else None
        return None
    b = cast(list[tuple[int, int]], a)                 # every bound known from here
    op = x.op
    if op == "add":
        return (b[0][0] + b[1][0], b[0][1] + b[1][1])
    if op == "sub":
        return (b[0][0] - b[1][1], b[0][1] - b[1][0])
    if op == "mul":
        p = [i * j for i in b[0] for j in b[1]]
        return (min(p), max(p))
    if op == "neg":
        return (-b[0][1], -b[0][0])
    if op == "not":
        return (-b[0][1] - 1, -b[0][0] - 1)
    if op == "bitlen" and b[0][0] >= 0:
        return (b[0][0].bit_length(), b[0][1].bit_length())
    nonneg = all(v[0] >= 0 for v in b)
    if op == "and" and nonneg:
        return (0, min(b[0][1], b[1][1]))
    if op == "and" and (b[0][0] >= 0 or b[1][0] >= 0):
        return (0, max(v[1] for v in b if v[0] >= 0))
    if op in ("or", "xor") and nonneg:
        return (0, (1 << max(b[0][1], b[1][1]).bit_length()) - 1)
    if op == "shl" and b[0][0] >= 0 and b[1][0] >= 0 and b[1][1] < 256:
        return (b[0][0] << b[1][0], b[0][1] << b[1][1])
    if op == "shr" and b[0][0] >= 0 and b[1][0] >= 0:
        return (b[0][0] >> b[1][1], b[0][1] >> b[1][0])
    if op == "mod" and b[1][0] > 0 and b[0][0] >= 0:
        return (0, b[1][1] - 1)
    if op in ("eq", "ne", "lt", "le", "gt", "ge"):
        decided = _compare(op, b[0], b[1])
        return None if decided is None else (int(decided), int(decided))
    return None


def _compare(op: str, x: tuple[int, int], y: tuple[int, int]) -> bool | None:
    if op == "lt":
        return True if x[1] < y[0] else False if x[0] >= y[1] else None
    if op == "le":
        return True if x[1] <= y[0] else False if x[0] > y[1] else None
    if op == "gt":
        return _compare("lt", y, x)
    if op == "ge":
        return _compare("le", y, x)
    if op == "eq":
        return False if x[1] < y[0] or y[1] < x[0] else True if x[0] == x[1] == y[0] == y[1] else None
    if op == "ne":
        eq = _compare("eq", x, y)
        return None if eq is None else not eq
    return None


class _Path:
    """The decisions a symbolic run takes: forced outcomes for its first tests, the tests it met, the pins."""

    def __init__(self, forced: list[bool], env: dict[str, int],
                 domains: dict[str, tuple[int, int]] | None = None) -> None:
        self.forced = forced
        self.taken: list[tuple[Expr, bool]] = []
        self.pins: dict[str, int] = {}
        self.env = env
        self.domains = domains or {}


class _Fork(Exception):
    def __init__(self, cond: Expr) -> None:
        self.cond = cond


_path: _Path | None = None


def _decide(cond: Expr) -> bool:
    assert _path is not None, "a symbol tested outside a symbolic run"
    if cond.op in ("eq", "ne") and len(cond.args) == 2:
        x, y = cond.args
        same: bool | None = None
        if not isinstance(y, (int, Expr)) or not isinstance(x, (int, Expr)):
            same = False                                 # a number against None, a string, a tuple
        elif isinstance(x, Expr) and isinstance(y, Expr) and x.key() == y.key():
            same = True                                  # an expression against itself
        if same is not None:
            return same if cond.op == "eq" else not same
    if cond.op in ("eq", "ne") and isinstance(cond.args[0], Expr) and isinstance(cond.args[1], Expr) and \
            repr(cond.args[1].key()) < repr(cond.args[0].key()):
        cond = Expr(cond.op, cond.args[1], cond.args[0])   # one order for an equality: x == y is y == x
    bits = _known_bits(cond)
    if bits is not None:
        return bits
    key = cond.key()
    flipped = ({"eq": "ne", "ne": "eq", "lt": "ge", "ge": "lt", "le": "gt", "gt": "le"}.get(key[0]), *key[1:])
    for c, outcome in _path.taken:                       # a test this path already took, or its negation
        if c.key() == key:
            return outcome
        if c.key() == flipped:
            return not outcome
    if _path.domains:
        decided = interval(cond, _path.domains)
        if decided is not None and decided[0] == decided[1]:
            return bool(decided[0])                      # the domains settle it: no fork
    i = len(_path.taken)
    if i < len(_path.forced):
        _path.taken.append((cond, _path.forced[i]))
        return _path.forced[i]
    raise _Fork(cond)


def _mask_test(cond: Expr) -> tuple[str, int, bool] | None:
    """(symbol, mask, wants nonzero) for a test `sym & mask != 0` or `== 0` (and `sym != 0`, `sym == 0`)."""
    if cond.op not in ("eq", "ne"):
        return None
    x, zero = cond.args
    if not isinstance(zero, int) or isinstance(zero, Expr) or zero != 0:
        return None
    if isinstance(x, Sym):
        return x.args[0], -1, cond.op == "ne"
    if isinstance(x, Expr) and x.op == "and" and isinstance(x.args[0], Sym) and isinstance(x.args[1], int):
        return x.args[0].args[0], x.args[1], cond.op == "ne"
    return None


def _known_bits(cond: Expr) -> bool | None:
    """A mask test the path's earlier mask tests on the same symbol decide: a bit known set makes `& m != 0` true for
    every m holding it; every bit of m known clear makes it false (live's flags, tested one by one and in groups)."""
    assert _path is not None
    test = _mask_test(cond)
    if test is None:
        return None
    name, mask, want_nonzero = test
    ones = zeros = 0
    for c, outcome in _path.taken:
        t = _mask_test(c)
        if t is None or t[0] != name:
            continue
        m, nonzero = t[1], t[2] == outcome
        if not nonzero:
            zeros |= m                                    # every bit of m clear
        elif m != -1 and m & (m - 1) == 0:
            ones |= m                                     # a single bit, set
    if ones & mask:
        result = True
    elif mask != -1 and zeros & mask == mask:
        result = False
    elif mask == -1 and zeros == -1:
        result = False
    else:
        return None
    return result if want_nonzero else not result


def _pin(e: Expr) -> int:
    """The sample's value of `e`, every symbol in it pinned to the sample's value (the template then holds for that value
    only)."""
    assert _path is not None, "a symbol read as an integer outside a symbolic run"

    def leaves(x: Any) -> Iterator[str]:
        if isinstance(x, Sym):
            yield x.args[0]
        elif isinstance(x, Expr):
            for a in x.args:
                yield from leaves(a)

    for name in leaves(e):
        _path.pins[name] = _path.env[name]
    return e.eval(_path.env)


# operands ------------------------------------------------------------------------------------------------------
class SymPart(str):
    """A field of an operand string that is a number of the instruction (an immediate's hex): `int` of it is a Sym."""

    sym: str


class SymOperand(str):
    """An operand string whose immediate field `split` returns as a SymPart."""

    sym: str

    def __getitem__(self, i: Any) -> str:                   # lift_mmx: int(o[3:]) of "MM:n" is the register
        part = str.__getitem__(self, i)
        if isinstance(i, slice) and i.start == 3 and i.stop is None and self.startswith("MM:"):
            p = SymPart(part)
            p.sym = self.sym.split(".")[0] + ".reg"
            return p
        return part

    def split(self, sep: Any = None, maxsplit: int = -1) -> list[str]:  # type: ignore[override]
        parts = str.split(self, sep, maxsplit)
        if self.startswith("I:") and len(parts) > 2:
            p = SymPart(parts[2])
            p.sym = self.sym
            parts[2] = p
        return parts


class _IntMeta(type):
    def __instancecheck__(cls, x: Any) -> bool:            # isinstance(x, int) where int is replaced
        return isinstance(x, builtins.int)

    def __call__(cls, x: Any = 0, base: Any = None) -> Any:
        if isinstance(x, SymPart):
            return Sym(x.sym)
        return builtins.int(x) if base is None else builtins.int(x, base)


class _sym_int(metaclass=_IntMeta):
    """`int` where a rule parses an immediate itself: a SymPart gives its symbol; anything else, builtins.int."""


def shape(ops: tuple[str, ...]) -> tuple[str, ...]:
    """Each operand with its numbers taken out: a register's width class, a memory operand's segment, base and index
    kinds, scale kind and size, an immediate's width."""
    out = []
    for s in ops:
        p = s.split(":")
        if p[0] == "R" and p[1] in REG:
            _, bits, hi = REG[p[1]]
            out.append(f"R{bits}{'h' if hi else ''}")
        elif p[0] == "R":
            out.append(f"R{p[1].rstrip('0123456789')}")         # XMM, YMM
        elif p[0] in ("M", "A"):
            base, index = (p[2], p[3]) if p[0] == "M" else (p[1], p[2])
            # lift._operand reads a base's name: an E-named base (or none with an E index) gives 32-bit addressing
            def kind(r: str) -> str:
                return r if r in ("INVALID", "RIP", "EIP") else f"r{REG[r][1]}{'E' if r.startswith('E') else ''}"
            scale = p[4] if p[0] == "M" else p[3]          # in the key: compare-by-hash code (a set of sizes against
            out.append(":".join([p[0], p[1] if p[0] == "M" else "", kind(base), kind(index), scale,   # {scale}) cannot
                                 p[6] if p[0] == "M" else ""]))                                        # see a symbol
        elif p[0] == "I":
            out.append(f"I{p[1]}")
        elif p[0] == "MM":
            out.append("MM")
        else:
            out.append(p[0])
    return tuple(out)


def values(ops: tuple[str, ...], nxt: int, address: int) -> dict[str, int]:
    """The numbers `shape` takes out, by the symbol names `_sym_operand` gives them."""
    env = {"nxt": nxt, "address": address}
    for k, s in enumerate(ops):
        p = s.split(":")
        if p[0] == "R" and p[1] in REG:
            env[f"o{k}.reg"] = REG[p[1]][0]
        elif p[0] == "R":
            env[f"o{k}.reg"] = int(p[1].lstrip("XMYM"))
        elif p[0] in ("M", "A"):
            base, index, scale, disp = (p[2], p[3], p[4], p[5]) if p[0] == "M" else (p[1], p[2], p[3], p[4])
            if base in REG:
                env[f"o{k}.base"] = REG[base][0]
            if index in REG:
                env[f"o{k}.index"] = REG[index][0]
            env[f"o{k}.disp"] = int(disp)
        elif p[0] == "I":
            env[f"o{k}.imm"] = int(p[2], 16)
        elif p[0] == "J":
            env[f"o{k}.disp"] = int(p[1])
        elif p[0] == "MM":
            env[f"o{k}.reg"] = int(p[1])
    return env


def _sym_operand(b: _OpBuilder, s: str, next_ip: Any, width_hint: int | None = None) -> tuple[str, object, int]:
    """lift._operand line for line, the numbers of operand `s` (a SymOperand named o<k>) as symbols."""
    k = getattr(s, "sym", None)
    if k is None:
        return _real_operand(b, s, next_ip, width_hint)
    k = k.split(".")[0]
    p = str.split(s, ":")
    if p[0] == "R" and p[1].startswith("XMM"):
        return "loc", ("x", Sym(f"{k}.reg")), 128
    if p[0] == "R" and p[1].startswith("YMM"):
        return "loc", ("x", Sym(f"{k}.reg")), 256
    if p[0] == "R":
        _, bits, hi = REG[p[1]]
        return "loc", ("r", Sym(f"{k}.reg"), bits, hi), bits
    if p[0] in ("M", "A"):
        base, index, scale, disp = (p[2], p[3], p[4], p[5]) if p[0] == "M" else (p[1], p[2], p[3], p[4])
        t = b.t()
        abits = 32 if base.startswith("E") or (base == "INVALID" and index.startswith("E")) else 64
        seg = p[1] if p[0] == "M" and p[1] in ("FS", "GS") else None
        if seg == "FS":
            raise LiftError(f"FS-relative memory operand {s}: the FS base is not modeled")
        d = Sym(f"{k}.disp")
        if base in ("RIP", "EIP"):
            b.emit("addr", t, None, None, 1, next_ip + d, abits, seg)
        else:
            b.emit("addr", t, None if base == "INVALID" else Sym(f"{k}.base"),
                   None if index == "INVALID" else Sym(f"{k}.index"), int(scale), d, abits, seg)
        if p[0] == "A":
            return "addr", t, abits
        return "loc", ("m", t, int(p[6])), int(p[6]) * 8
    if p[0] == "I":
        width, raw = int(p[1]) * 8, Sym(f"{k}.imm")
        t = b.t()
        target = width_hint or width
        value: Any = raw
        if width < target and raw >> (width - 1) & 1:
            value = raw | (((1 << target) - 1) ^ ((1 << width) - 1))
        b.emit("imm", t, value)
        return "imm", t, width
    if p[0] == "J":
        t = b.t()
        b.emit("imm", t, (next_ip + Sym(f"{k}.disp")) & M64)
        return "imm", t, 64
    raise LiftError(f"operand {s}")


_real_operand = lift_module._operand


class _Fields:
    """The decoder's fields as a rule reads them: each attribute read is recorded, with its value, for the key."""

    def __init__(self, f: Any) -> None:
        object.__setattr__(self, "_f", f)
        object.__setattr__(self, "read", {})

    def __getattr__(self, name: str) -> Any:
        v = getattr(object.__getattribute__(self, "_f"), name)
        object.__getattribute__(self, "read")[name] = v
        return v


@contextmanager
def _symbolic() -> Iterator[None]:
    """`_operand` and `int` replaced where the rules read operands, for the length of a symbolic run."""
    from w2m.rules import _int, _sse, _vex, x87
    saved = [(m, "_operand", getattr(m, "_operand")) for m in (lift_module, _int, _sse, _vex, x87)]
    for m, name, _ in saved:
        setattr(m, name, _sym_operand)
    for m in (lift_module, _sse, _vex):                  # lift_mmx, _sse and _vex parse an immediate themselves
        m.int = _sym_int  # type: ignore[attr-defined]
    try:
        yield
    finally:
        for m, name, v in saved:
            setattr(m, name, v)
        for m in (lift_module, _sse, _vex):
            del m.int


# templates -----------------------------------------------------------------------------------------------------
@dataclass(frozen=True)
class Template:
    iclass: str
    shape: tuple[str, ...]
    mode: int
    fields: tuple[tuple[str, Any], ...]      # decoder fields the rule read, and their values
    conditions: tuple[tuple[Expr, bool], ...]  # tests on symbols, and the outcome this template took
    pins: tuple[tuple[str, int], ...]        # symbols the rule needed as integers, and their values
    ops: tuple[Any, ...]

    def applies(self, f: Any, env: dict[str, int]) -> bool:
        return (all(getattr(f, n) == v for n, v in self.fields)
                and all(env.get(n) == v for n, v in self.pins)
                and all(bool(c.eval(env)) == want for c, want in self.conditions))


def fill(x: Any, env: dict[str, int]) -> Any:
    """The template's ops (or any part of them) with every expression evaluated on `env`."""
    if isinstance(x, Expr):
        return x.eval(env)
    if isinstance(x, tuple):
        return tuple(fill(y, env) for y in x)
    return x


def templates(code: bytes, address: int) -> list[Template]:
    """Every template of the rule for the instruction at the start of `code`: one per path through the rule's tests on
    symbols, the sample's values used only for pins."""
    global _path
    try:
        d = decode(code)
    except DecodeError as e:
        raise LiftError(f"{e} at 0x{address:x}") from None
    f, ic = d.fields, d.iclass
    ops = operands(f) if f is not None and f.ops else ()
    declared_none = f is not None and f.ops == "-"
    if not ops and not declared_none and ic not in ("NOP", "RET_NEAR", "CLD", "STD", "CPUID", "CDQ", "CDQE", "CQO",
                                                    "CWDE", "SFENCE", "LFENCE", "MFENCE", "XGETBV", "VZEROUPPER",
                                                    "VZEROALL", *REP_STRING, *IMPLICIT_050):
        raise LiftError(f"no operand template for {ic}")
    from w2m.rules import Ctx, registry
    sym_ops = []
    for k, s in enumerate(ops):
        o = SymOperand(s)
        o.sym = f"o{k}.imm"
        sym_ops.append(o)
    env = values(ops, address + d.length, address)
    out: list[Template] = []
    pending: list[list[bool]] = [[]]
    with _symbolic():
        while pending:
            forced = pending.pop()
            _path = _Path(forced, env)
            b = _OpBuilder()
            fields = _Fields(f)
            try:
                if ic in MMX_ONLY or any(o.startswith("MM:") for o in ops):
                    lift_mmx(b, ic, tuple(sym_ops), cast(int, Sym("nxt")))
                else:
                    rule = registry().get(ic)
                    if rule is None:
                        raise LiftError(f"no lifting rule for {ic}")
                    rule.rule(Ctx(b, ic, fields, tuple(sym_ops), Sym("nxt"), Sym("address")))  # type: ignore[arg-type]
            except _Fork:
                pending.append([*forced, True])
                pending.append([*forced, False])
                continue
            finally:
                path = _path
                _path = None
            assert path is not None
            out.append(Template(ic, shape(ops), MODE, tuple(sorted(fields.read.items(), key=lambda kv: kv[0])),
                                tuple(path.taken), tuple(sorted(path.pins.items())), tuple(b.ops)))
    return out


def lifted_by_template(code: bytes, address: int, made: dict[tuple[Any, ...], list[Template]]) -> tuple[Any, ...] | None:
    """The ops of the instruction at `code` from a template of `made` (keyed by class, shape and mode), made on first
    need; None where no template applies."""
    d = decode(code)
    f = d.fields
    ops = operands(f) if f is not None and f.ops else ()
    key = (d.iclass, shape(ops), MODE)
    env = values(ops, address + d.length, address)
    for t in made.get(key, ()):
        if t.applies(f, env):
            return tuple(fill(t.ops, env))
    made.setdefault(key, []).extend(templates(code, address))
    for t in made[key]:
        if t.applies(f, env):
            return tuple(fill(t.ops, env))
    return None
