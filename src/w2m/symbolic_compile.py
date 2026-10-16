# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""symbolic_compile.py — compile_insn run on a symbolic template gives stencils (Sprint 090; Decision 2026-10-16, the
back end by symbolic execution).

A template (symbolic_lift.Template) holds a rule's ops with its instruction's numbers as symbols. Here
vm_backend.compile_insn runs on those ops, its own arguments symbolic too: the code's base address (`base`), the exit
(`exit`), the runtime's symbols (`sym.<name>`), the flags read after the instruction (`live`, six bits) and whether the
next Jcc branches on its NZCV (`nzcv_out`); the kind of NZCV before it (`nzcv_from`) stays a key. Each word comes out
as an expression of those symbols: a stencil whose holes are known by construction. A choice compile_insn or an
encoder makes on a symbol's value forks the run, unless the symbols' domains (each operand's range) decide it; each
stencil carries the conditions of its path. A path that raises (an encoder refusing its arguments, compile_insn's own
CompileError) is a stencil that refuses: the host compiles that instruction.
"""

from __future__ import annotations

import builtins
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import dataclass
from typing import Any, cast

from w2m import symbolic_lift as sl
from w2m import vm_backend as vb
from w2m.lift import Lifted, lift
from w2m.symbolic_lift import Expr, Sym, Template
from w2m.x86dec import decode
from w2m.x86ops import operands

RUNTIME_SYMBOLS = ("w2m_cpuid", "w2m_approximate", "w2m_pcmpistri")
NZCV_FROM: tuple[str | None, ...] = (None, *vb.NZCV_CONDITIONS)
LO, HI = vb.MONITOR_VA, vb.MONITOR_VA + vb.MONITOR_SIZE


def domains(shape: tuple[str, ...]) -> dict[str, tuple[int, int]]:
    """Each symbol's range: what its operand allows (a register number 0-15, a 32-bit displacement, an immediate of its
    width), the code and the exit in the monitor, a runtime symbol in the runtime image, `live` six bits."""
    out: dict[str, tuple[int, int]] = {
        "nxt": (1 << 12, 1 << 47), "address": (1 << 12, 1 << 47), "base": (LO, HI), "exit": (LO, HI),
        "live": (0, vb.ALL_FLAGS), "nzcv_out": (0, 1)}
    for name in RUNTIME_SYMBOLS:
        out[f"sym.{name}"] = (LO + vb.RUNTIME_IMAGE, LO + vb.RUNTIME_IMAGE + vb.RUNTIME_IMAGE_SIZE)
    for k, s in enumerate(shape):
        o = f"o{k}"
        if s.startswith("R"):
            out[f"{o}.reg"] = (0, 15)
        elif s == "MM":
            out[f"{o}.reg"] = (0, 7)
        elif s.startswith(("M:", "A:")):
            out.update({f"{o}.base": (0, 15), f"{o}.index": (0, 15), f"{o}.disp": (-(1 << 31), (1 << 31) - 1)})
        elif s.startswith("I"):
            out[f"{o}.imm"] = (0, (1 << (8 * int(s[1:]))) - 1)
        elif s == "J":
            out[f"{o}.disp"] = (-(1 << 31), (1 << 31) - 1)
    return out


class _SymAssembler(vb.Assembler):
    """vm_backend.Assembler with a symbolic base: a branch's target may be the symbolic exit."""

    def finish(self) -> list[int]:
        for i, make, target in self.fix:
            va = self.labels[target] if isinstance(target, builtins.int) and not isinstance(target, Expr) and \
                target < 0 else target
            self.words[i] = make(cast(Any, va) - (self.base + 4 * i))
        return self.words


def _isinstance(x: Any, t: Any) -> bool:
    """isinstance for a symbolic run: an expression stands for an int (vm_backend asks `isinstance(base, int)`)."""
    if isinstance(x, Expr):
        types = t if isinstance(t, tuple) else (t,)
        return builtins.int in types or Expr in types
    return builtins.isinstance(x, t)


@contextmanager
def _compiling() -> Iterator[None]:
    from w2m.rules import _mmx, _sse, _vex
    mods = (vb, _sse, _vex, _mmx)
    for m in mods:
        m.isinstance = _isinstance  # type: ignore[attr-defined]
    try:
        yield
    finally:
        for m in mods:
            del m.isinstance


@dataclass(frozen=True)
class Stencil:
    template: Template
    nzcv_from: str | None
    conditions: tuple[tuple[Expr, bool], ...]    # compile_insn's tests on symbols, and this path's outcome
    pins: tuple[tuple[str, int], ...]
    words: tuple[Any, ...] | None                # None: this path refuses (the host compiles it)
    ends: Any
    refusal: str = ""

    def applies(self, env: dict[str, int]) -> bool:
        return (all(env.get(n) == v for n, v in self.pins)
                and all(bool(c.eval(env)) == want for c, want in self.conditions))


def stencils(t: Template, nzcv_from: str | None, sample: dict[str, int]) -> list[Stencil]:
    """Every path of compile_insn on template `t`; `sample` gives values for pins (an instruction of the template)."""
    out: list[Stencil] = []
    pending: list[list[bool]] = [[]]
    syms = {n: Sym(f"sym.{n}") for n in RUNTIME_SYMBOLS}
    dom = domains(t.shape)
    with _compiling():
        while pending:
            forced = pending.pop()
            sl._path = sl._Path(forced, sample, dom)
            a = _SymAssembler(cast(int, Sym("base")))
            words: tuple[Any, ...] | None = None
            ends: Any = None
            refusal = ""
            try:
                ends = vb.compile_insn(a, Lifted(0, 0, t.iclass, t.ops), Sym("exit"), syms, nzcv_from,  # type: ignore[arg-type]
                                       Sym("live"), Sym("nzcv_out"))  # type: ignore[arg-type]
                words = tuple(a.finish())
            except sl._Fork:
                pending.append([*forced, True])
                pending.append([*forced, False])
                continue
            except Exception as e:                       # an encoder or compile_insn refuses: the host compiles it
                refusal = f"{type(e).__name__}: {e}"[:200]
            finally:
                path = sl._path
                sl._path = None
            assert path is not None
            out.append(Stencil(t, nzcv_from, tuple(path.taken), tuple(sorted(path.pins.items())), words, ends,
                               refusal))
    return out


@dataclass
class Table:
    """Templates and their stencils, made on first need, by (class, shape, mode) and NZCV kind."""
    templates: dict[tuple[Any, ...], list[Template]]
    stencils: dict[tuple[int, str | None], list[Stencil]]

    def __init__(self) -> None:
        self.templates = {}
        self.stencils = {}


def words_by_stencil(table: Table, code: bytes, address: int, base: int, exit_va: int, symbols: dict[str, int],
                     nzcv_from: str | None, live: int, nzcv_out: bool) -> tuple[list[int] | None, str]:
    """The instruction's words from its stencil (None and why, where it refuses or none applies)."""
    d = decode(code)
    f = d.fields
    ops = operands(f) if f is not None and f.ops else ()
    key = (d.iclass, sl.shape(ops), sl.MODE)
    env = sl.values(ops, address + d.length, address)
    if key not in table.templates:
        table.templates[key] = sl.templates(code, address)
    t = next((t for t in table.templates[key] if t.applies(f, env)), None)
    if t is None:
        return None, "no template applies"
    env.update(base=base, exit=exit_va, live=live, nzcv_out=int(nzcv_out),
               **{f"sym.{n}": v for n, v in symbols.items() if n in RUNTIME_SYMBOLS})
    skey = (id(t), nzcv_from)
    if skey not in table.stencils:
        table.stencils[skey] = stencils(t, nzcv_from, env)
    s = next((s for s in table.stencils[skey] if s.applies(env)), None)
    if s is None:
        return None, "no stencil applies"
    if s.words is None:
        return None, s.refusal
    return [w.eval(env) & 0xFFFFFFFF if isinstance(w, Expr) else w for w in s.words], ""


def compile_words(code: bytes, address: int, base: int, exit_va: int, symbols: dict[str, int], nzcv_from: str | None,
                  live: int, nzcv_out: bool) -> list[int]:
    """compile_insn's own words for the instruction (the judge's reference)."""
    a = vb.Assembler(base)
    vb.compile_insn(a, lift(code, address), exit_va, symbols, nzcv_from, live, nzcv_out)
    return a.finish()
