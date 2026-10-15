# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The stencil census (Sprint 071, recentering step 6): does the compiler's output factor into stencils?

Copy-and-patch (Xu and Kjolstad, OOPSLA 2021; CPython 3.13's JIT, PEP 744) builds code from a library of machine-code
stencils with holes for an operation's values. This script takes the `compile_insn` calls a frontier run recorded
(scratch driver record_insns.py: each distinct call with its count) and compiles each again with every arm64 encoder
call logged (`a64_encoder`'s functions, the outermost call of a nest), so an instruction's words become a sequence of
encoder calls, words no encoder made (raw) and branch words the assembler fills at its end. Instructions are grouped by
form: the iclass and ops with guest registers and values taken out, and the flag arguments. Per form: the encoder
sequences it produced (one is a stencil; several mean values choose the structure) and the arguments that vary
across its instructions (the holes), each named by what it equals in the instruction (a guest register, a value or a
piece of one) or `other`.

Usage: uv run python scripts/stencil_census.py <record.pickle> [--top N] [--refine | --decoder-key]

`--decoder-key` (Sprint 074) keys each instruction by what the decoder gives (decoded_form) rather than by the lifted
ops: the key the compiler inside the VM can compute. Its record holds each Lifted's bytes (scratch driver
record_insn_bytes.py).
"""

from __future__ import annotations

import argparse
import functools
import pickle
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from w2m import a64_encoder, vm_backend, x86dec, x86ops  # noqa: E402
from w2m.lift import lift  # noqa: E402

NEAR = 0x40_0000                                         # each instruction is compiled again this far below its exit

Call = tuple[Any, ...]                                   # (lifted, exit_va, nzcv_from, live_after, nzcv_out)


class EncoderLog:
    """Wraps every public function of a64_encoder; while `on`, the outermost calls are logged as (name, args,
    words)."""

    def __init__(self) -> None:
        self.calls: list[tuple[str, tuple[Any, ...], list[int]]] = []
        self.depth = 0
        self.on = False
        for name in dir(a64_encoder):
            f = getattr(a64_encoder, name)
            if callable(f) and not name.startswith("_") and getattr(f, "__module__", "") == a64_encoder.__name__:
                setattr(a64_encoder, name, self._wrap(name, f))

    def _wrap(self, name: str, f: Any) -> Any:
        @functools.wraps(f)
        def logged(*args: Any, **kwargs: Any) -> Any:
            self.depth += 1
            try:
                out = f(*args, **kwargs)
            finally:
                self.depth -= 1
            if self.on and self.depth == 0:
                words = out if isinstance(out, list) else [out] if isinstance(out, int) else None
                if words is not None:
                    self.calls.append((name, args + tuple(sorted(kwargs.items())), list(words)))
            return out
        return logged


def compile_logged(log: EncoderLog, call: Call, symbols: dict[str, int]) -> tuple[list[int], list[Any]]:
    """The instruction's words and its items: ("enc", name, args), ("enc32", name, args) (the encoder's word with
    bit 31, sf, cleared: its 32-bit form), ("raw", word) or ("branch", name, args)."""
    ins, exit_va, nzcv_from, live_after, nzcv_out = call
    a = vm_backend.Assembler(exit_va - NEAR)
    log.calls, log.on = [], True
    try:
        vm_backend.compile_insn(a, ins, exit_va, symbols, nzcv_from, live_after, nzcv_out)
        body_calls = log.calls
        fixes = {i for i, _, _ in a.fix}
        log.calls = []
        words = a.finish()
        branch_calls = log.calls
    finally:
        log.on = False
    items: list[Any] = []
    j = 0                                                # the next body call
    i = 0
    used: set[int] = set()
    branches = iter(branch_calls)

    def match(k: int) -> str | None:
        ws = body_calls[k][2]
        if ws and words[i:i + len(ws)] == ws and not fixes & set(range(i, i + len(ws))):
            return "enc"
        if len(ws) == 1 and words[i] == ws[0] & ~(1 << 31) != ws[0] and i not in fixes:
            return "enc32"
        return None

    while i < len(words):
        if i in fixes:
            name, args, _ = next(branches)
            items.append(("branch", name, args))
            i += 1
            continue
        # the next logged call whose words come next; else an earlier one not yet placed (a call made before the
        # words before it were emitted: compile_insn's Jcc encodes its target's MOVZ/MOVK before its branch)
        k = next((k for k in range(j, len(body_calls)) if k not in used and match(k)), None)
        if k is None:
            k = next((k for k in range(j) if k not in used and match(k)), None)
        if k is not None:
            kind = match(k)
            name, args, ws = body_calls[k]
            items.append((kind, name, args))
            used.add(k)
            i += len(ws) if kind == "enc" else 1
            j = max(j, k + 1)
        else:
            items.append(("raw", words[i]))
            i += 1
    return words, items


def rebuild(items: list[Any], words_of: dict[str, Any]) -> list[int]:
    """The words the items stand for (the rebuild check): each encoder called again with its arguments; a branch
    item's word is taken as recorded."""
    out: list[int] = []
    for it in items:
        if it[0] in ("enc", "enc32"):
            pos = tuple(a for a in it[2] if not (isinstance(a, tuple) and len(a) == 2 and isinstance(a[0], str)))
            kw = dict(a for a in it[2] if isinstance(a, tuple) and len(a) == 2 and isinstance(a[0], str))
            w = words_of[it[1]](*pos, **kw)
            ws = w if isinstance(w, list) else [w]
            out.extend(ws if it[0] == "enc" else [ws[0] & ~(1 << 31)])
        elif it[0] == "raw":
            out.append(it[1])
        else:
            out.append(it[3])
    return out


def form(call: Call) -> tuple[tuple[Any, ...], list[int]]:
    """The instruction's form (ops with guest registers and values taken out, the iclass and the flag arguments) and
    its values in order: the guest registers (XMM registers as 1000 + n), then immediates and displacements."""
    ins, exit_va, nzcv_from, live_after, nzcv_out = call
    regs: list[int] = []                                 # guest registers, in order
    values: list[int] = []                               # immediates and displacements, in order

    def out(op: Any) -> Any:
        k = op[0]
        if k == "imm":
            values.append(op[2])
            return ("imm", op[1], "V")
        if k == "addr":
            _, t, base, index, scale, disp, abits, seg = op
            for r in (base, index):
                if isinstance(r, int):
                    regs.append(r)
            values.append(disp)
            return ("addr", t, None if base is None else "R", None if index is None else "R", scale, "V", abits, seg)
        return tuple(loc(x) for x in op)

    def loc(x: Any) -> Any:
        if isinstance(x, tuple) and x and x[0] == "r" and len(x) == 4:
            regs.append(x[1])
            return ("r", "R", x[2], x[3])
        if isinstance(x, tuple) and x and x[0] == "x" and len(x) == 2:
            regs.append(1000 + x[1])                     # an XMM register, apart from the general ones
            return ("x", "X")
        return x

    ops = tuple(out(op) for op in ins.ops)
    return (ins.iclass, ops, nzcv_from, live_after, nzcv_out, exit_va), regs + values


def value_class(v: int) -> tuple[Any, ...]:
    """The features of a value that compile_insn's choices read (_folded_addresses, the ADD/SUB immediates of
    _operand_forms and of a computed address, mov_imm's halfwords): zero, sign, the 12-bit and 9-bit ranges, the scaled
    12-bit offset for each access size, which halfwords are nonzero, and a shift count's 1 and 0 (5 and 6 bits)."""
    m = v & (1 << 64) - 1
    sv = m - (1 << 64) if m >> 63 else m
    scaled = tuple(sv % size == 0 and 0 <= sv // size < 4096 for size in (1, 2, 4, 8, 16))
    return (sv == 0, sv < 0, 0 <= sv < 4096, -4096 < sv < 4096, -256 <= sv <= 255, scaled,
            tuple(bool(m >> (16 * h) & 0xFFFF) for h in range(4)),
            m & 31 == 1, m & 63 == 1, m & 31 == 0, m & 63 == 0)   # a shift count: OF defined at 1; nothing at 0


def refined(key: tuple[Any, ...], values: list[int], registers: int) -> tuple[Any, ...]:
    """The form with each value's class, and which of its guest registers are the same register."""
    regs = values[:registers]
    same = tuple(regs.index(r) for r in regs)
    return (*key, same, tuple(value_class(v) for v in values[registers:]))


REGISTER_NUMBERS: dict[str, tuple[str, int]] = {}   # an operand's register name: (its class, its number)
for _cls, _names in (("q", x86ops.GPR64), ("d", x86ops.GPR32), ("w", x86ops.GPR16), ("b", x86ops.GPR8_REX),
                     ("x", x86ops.XMM), ("y", x86ops.YMM)):
    for _n, _name in enumerate(_names):
        REGISTER_NUMBERS[_name] = (_cls, 1000 + _n if _cls in "xy" else _n)
for _n, _name in enumerate(("AH", "CH", "DH", "BH")):
    REGISTER_NUMBERS[_name] = ("h", _n)                 # bits 8-15 of RAX, RCX, RDX, RBX
SPECIAL = {"RSP", "ESP", "SP", "SPL", "RIP"}            # kept by name: the stack pointer and RIP have their own forms


def lifted_registers(ins: Any) -> set[int]:
    """Every guest register the lifted ops name (XMM as 1000 + n)."""
    out: set[int] = set()

    def walk(x: Any) -> None:
        if isinstance(x, tuple):
            if len(x) == 4 and x[0] == "r" and isinstance(x[1], int):
                out.add(x[1])
            elif len(x) == 2 and x[0] == "x" and isinstance(x[1], int):
                out.add(1000 + x[1])
            else:
                for y in x:
                    walk(y)
    walk(ins.ops)
    return out


def implicit_registers(record: dict[str, Any]) -> dict[str, frozenset[int]]:
    """Per spec row, the registers its instructions use beyond their explicit operands (MUL's RAX and RDX, a shift's
    CL): the union over the record of the lifted registers less the decoded ones."""
    out: dict[str, set[int]] = defaultdict(set)
    for ins, encodings in record["bytes"].items():
        for code in encodings:
            d = x86dec.decode(code)
            _, values, nreg = decoded_form((ins, 0, None, 0, False), code)
            out[d.row or ""] |= lifted_registers(ins) - set(values[:nreg])
    return {row: frozenset(r) for row, r in out.items()}


def canonical_flags(ins: Any, nzcv_from: str | None, live_after: int, nzcv_out: bool) -> tuple[str | None, int, bool]:
    """The flag arguments as compile_insn reads them (Sprint 079): `live_after` and `nzcv_out` only through a flags
    op (its flag masks, the dead ops, the last flags op's NZCV); `nzcv_from` only through a Jcc whose condition its
    kind's NZCV_CONDITIONS holds. An argument the instruction cannot read is written as its default, so keys that
    differ only there are one key."""
    has_flags = any(op[0] == "flags" for op in ins.ops)
    table = vm_backend.NZCV_CONDITIONS.get(nzcv_from or "", {})        # as compile_insn reads it
    reads = any(op[0] == "br" and op[1] in table for op in ins.ops)
    return (nzcv_from if reads else None, live_after if has_flags else 0, nzcv_out if has_flags else False)


def decoded_form(call: Call, code: bytes, implicit: frozenset[int] = frozenset()) -> tuple[tuple[Any, ...], list[int], int]:
    """The instruction's key as the decoder gives it (Sprint 074): the spec row, the iclass, the operand and address
    sizes, VEX.L, and the operands (x86ops) with register numbers and values taken out, which registers are equal,
    each value's class, and the flag arguments (canonical_flags). The values, in order: guest registers (XMM and YMM as 1000 + n), then
    immediates (raw and sign-extended), displacements, the next instruction's address and any address the
    instruction names (a branch target, a RIP-relative address). `implicit`: the row's implicit registers
    (implicit_registers); an explicit register equal to one is named in the key. Returns (key, values, the count of
    registers)."""
    ins, exit_va, nzcv_from, live_after, nzcv_out = call
    nzcv_from, live_after, nzcv_out = canonical_flags(ins, nzcv_from, live_after, nzcv_out)
    d = x86dec.decode(code)
    f = d.fields
    nxt = ins.address + d.length
    regs: list[int] = []
    values: list[int] = [nxt]
    shape: list[Any] = []

    def register(name: str) -> Any:
        if name in SPECIAL or name not in REGISTER_NUMBERS:
            return name
        cls, n = REGISTER_NUMBERS[name]
        regs.append(n)
        return cls

    ops = x86ops.operands(f) if f is not None and f.ops else ()
    for o in ops:
        p = o.split(":")
        if p[0] == "R":
            shape.append(("R", register(p[1])))
        elif p[0] in ("M", "A"):
            seg, rest = (p[1], p[2:]) if p[0] == "M" else (None, p[1:])
            base, index, scale, disp = rest[0], rest[1], int(rest[2]), int(rest[3])
            values.append(disp)
            if base == "RIP":
                values.append(nxt + disp)
            shape.append((p[0], seg, register(base) if base != "INVALID" else None,
                          register(index) if index != "INVALID" else None, scale, tuple(rest[4:])))
        elif p[0] == "I":
            width, raw = int(p[1]), int(p[2], 16)
            values += [raw, raw - (1 << 8 * width) if raw >> (8 * width - 1) else raw]
            shape.append(("I", width))
        elif p[0] == "J":
            values += [int(p[1]), nxt + int(p[1])]
            shape.append(("J",))
        else:
            shape.append(o)                              # MM:n, ST:n: kept whole
    same = tuple(regs.index(r) for r in regs)
    named = tuple(r if r in implicit else None for r in regs)   # an explicit register that is also an implicit one
    key = (d.row, d.iclass, f.opsize if f else None, f.asize if f else None, f.vl if f else None, tuple(shape), same, named,
           tuple(value_class(v) for v in values), nzcv_from, live_after, nzcv_out, exit_va)
    return key, regs + values, len(regs)



# The record in a form that needs none of w2m's classes to read (Sprint 080, review 2026-10-15 H3): a header line of
# JSON (the runtime symbols, the exit address), then each compile call in record order as "address bytes nzcv_from
# live_after nzcv_out count" (nzcv_from an index of [None, *NZCV_CONDITIONS]). Reading it lifts each instruction
# again from its bytes, which Sprint 074 showed gives the recorded Lifted.
NZCV_KINDS: list[str | None] = [None, *vm_backend.NZCV_CONDITIONS]


def write_record(record: dict[str, Any], path: Path) -> None:
    import gzip
    import json
    exits = {call[1] for call in record["calls"]}
    assert len(exits) == 1, exits
    lines = [json.dumps({"symbols": record["symbols"], "exit_va": exits.pop()}, sort_keys=True)]
    for call, n in record["calls"].items():
        ins, _, nzcv_from, live_after, nzcv_out = call
        code = next(iter(record["bytes"][ins]))
        lines.append(f"{ins.address:x} {code.hex()} {NZCV_KINDS.index(nzcv_from)} {live_after} {int(nzcv_out)} {n}")
    with open(path, "wb") as f, gzip.GzipFile(fileobj=f, mode="wb", mtime=0, filename="") as g:
        g.write(("\n".join(lines) + "\n").encode())


def read_record(path: Path) -> dict[str, Any]:
    """A record from write_record's file, or a pickled one."""
    import gzip
    import json
    if path.suffix == ".pickle":
        with open(path, "rb") as f:
            out: dict[str, Any] = pickle.load(f)
        return out
    with gzip.open(path, "rt") as f:
        head = json.loads(f.readline())
        calls: dict[Any, int] = {}
        codes: dict[Any, set[bytes]] = {}
        lifted: dict[tuple[int, bytes], Any] = {}
        for line in f:
            address, code_hex, nzcv_from, live_after, nzcv_out, n = line.split()
            a, code = int(address, 16), bytes.fromhex(code_hex)
            ins = lifted.get((a, code))
            if ins is None:
                ins = lifted[(a, code)] = lift(code, a)
                codes.setdefault(ins, set()).add(code)
            calls[(ins, head["exit_va"], NZCV_KINDS[int(nzcv_from)], int(live_after), bool(int(nzcv_out)))] = int(n)
    return {"calls": calls, "symbols": head["symbols"], "bytes": codes}

M64 = (1 << 64) - 1


def _expressions() -> list[tuple[str, str, Any]]:
    """What a hole may equal, as (kind, name, function of one value), tried in this order (Sprint 074): the value; a
    piece of it (a halfword, a 12-bit part, the value masked to 8, 16 or 32 bits); its negation; an affine function
    (an XMM number times 16, a width less a shift count, a shift count less 1); a bit field of up to 4 bits in its low
    byte (a shuffle immediate's selectors); a shift count masked to 5 or 6 bits, less 1, or taken from the width."""
    out: list[tuple[str, str, Any]] = [("value", "v", lambda v: v), ("value", "v mod 2^64", lambda v: v & M64),
                                       ("value", "xmm", lambda v: v - 1000)]
    out += [("piece", f"halfword {h}", lambda v, h=h: v >> 16 * h & 0xFFFF) for h in range(4)]
    out += [("piece", "low 12 bits", lambda v: v & 0xFFF), ("piece", "bits 12-23", lambda v: v >> 12 & 0xFFF)]
    out += [("piece", f"masked to {k} bits", lambda v, k=k: v & (1 << k) - 1) for k in (8, 16, 32)]
    out += [("negated", "-v", lambda v: -v), ("negated", "-v mod 2^64", lambda v: -v & M64),
            ("negated", "-v mod 2^32", lambda v: -v & 0xFFFFFFFF)]
    out += [("affine", "16 x xmm", lambda v: 16 * (v - 1000))]
    for k in (5, 6):                                     # a shift count, masked as x86 masks it (Sprint 076); before
        m = (1 << k) - 1                                 # the unmasked forms, which agree with it below 32 or 64
        out += [("piece", f"low {k} bits", lambda v, m=m: v & m),
                ("affine", f"low {k} bits - 1", lambda v, m=m: (v & m) - 1),
                ("affine", f"{m + 1} - low {k} bits", lambda v, m=m: m + 1 - (v & m))]
    out += [("affine", "v - 1", lambda v: v - 1)]
    out += [("affine", f"{w} - v", lambda v, w=w: w - v) for w in (8, 16, 32, 64)]
    # 2-bit fields at even positions first (a shuffle's selectors): where a 1-bit field fits, the 2-bit field holding
    # it fits too whenever the other bit never varied (Sprint 076)
    out += [("field", f"bits {s}+2", lambda v, s=s: v >> s & 3) for s in (0, 2, 4, 6)]
    out += [("field", f"bits {s}+{w}", lambda v, s=s, w=w: v >> s & (1 << w) - 1)
            for w in (1, 2, 3, 4) for s in range(0, 8) if not (w == 2 and s % 2 == 0)]
    out += [("affine", f"min(v, {w})", lambda v, w=w: min(v, w))                   # PSRAD's and PSRAW's counts
            for w in (8, 16, 32, 64, 7, 15, 31, 63)]                                # (Sprint 076)
    return out


EXPRESSIONS = _expressions()


def hole_expression(pairs: list[tuple[Any, list[int]]]) -> tuple[str, str, int] | None:
    """The first expression of one value that gives the hole's argument in every instruction of a form, as (kind,
    expression, the value's position), or None. `pairs`: each instruction's argument and values."""
    if not all(isinstance(a, int) for a, _ in pairs) or len({len(v) for _, v in pairs}) != 1:
        return None
    for kind, name, fn in EXPRESSIONS:
        for p in range(len(pairs[0][1])):
            if all(fn(values[p]) == arg for arg, values in pairs):
                return kind, name, p
    return None


def _values_only(call: Call) -> list[int]:
    """The immediates and displacements of the call's instruction."""
    out: list[int] = []
    for op in call[0].ops:
        if op[0] == "imm":
            out.append(op[2])
        elif op[0] == "addr":
            out.append(op[5])
    return out


def census(record: dict[str, Any], top: int, refine: bool, decoder_key: bool = False) -> int:
    calls: dict[Call, int] = record["calls"]
    symbols = record["symbols"]
    log = EncoderLog()
    words_of = {name: getattr(a64_encoder, name).__wrapped__ for name in dir(a64_encoder)
                if hasattr(getattr(a64_encoder, name), "__wrapped__")}
    by_form: dict[Any, list[tuple[int, list[int], tuple[Any, ...]]]] = defaultdict(list)
    failures = Counter[str]()
    raw_words = 0
    kinds = Counter[str]()                               # compiled instructions with an item of each kind
    relifted: dict[Any, bool] = {}                       # each Lifted: its bytes lift to it again (decoder_key)
    implicit = implicit_registers(record) if decoder_key else {}
    for call, n in calls.items():
        try:
            words, items = compile_logged(log, call, symbols)
        except Exception as ex:                          # the run compiled it; here a different base can fail it
            failures[type(ex).__name__] += n
            continue
        fixed = [it if it[0] != "branch" else (*it, w) for it, w in zip(items, _item_words(items, words))]
        if rebuild(fixed, words_of) != words:
            failures["rebuild"] += n
            continue
        raw_words += sum(1 for it in items if it[0] == "raw") * n
        kinds.update({it[0]: n for it in items})
        if decoder_key:                                  # the key from the decoder's fields (Sprint 074)
            encodings = record["bytes"].get(call[0], ())
            if len(encodings) != 1:
                failures[f"encodings {min(len(encodings), 2)}"] += n
                continue
            code = next(iter(encodings))
            if relifted.setdefault(call[0], lift(code, call[0].address) == call[0]) is False:
                failures["relift"] += n                  # the bytes do not lift to the recorded instruction
                continue
            key, values, _ = decoded_form(call, code, implicit.get(x86dec.decode(code).row or "", frozenset()))
        else:
            key, values = form(call)
        if refine and not decoder_key:
            registers = len(values) - len(_values_only(call))
            key = refined(key, values, registers)
        sig = tuple((it[0], it[1], len(it[2])) if it[0] != "raw" else ("raw",) for it in items)
        by_form[key].append((n, values, (sig, tuple(items))))
    total = sum(calls.values())
    compiled = sum(n for group in by_form.values() for n, _, _ in group)
    print(f"{len(calls)} distinct compile calls, {total} in all; {compiled} compiled again and rebuilt exactly; "
          f"failures {dict(failures)}")
    if decoder_key:
        print(f"instructions lifted again from their bytes: {sum(relifted.values())} of {len(relifted)} equal the "
              f"recorded Lifted")
    print(f"{len(by_form)} forms; raw words (no encoder) {raw_words}; instructions with an item of each kind "
          f"{dict(kinds)}")
    one = [k for k, g in by_form.items() if len({s for _, _, (s, _) in g}) == 1]
    one_n = sum(n for k in one for n, _, _ in by_form[k])
    print(f"forms with one encoder sequence: {len(one)} of {len(by_form)}, {one_n} of {compiled} compiled "
          f"instructions ({100 * one_n / max(compiled, 1):.1f}%)")
    holes = Counter[str]()
    others: list[Any] = []                               # examples of holes named `other`
    many: list[tuple[int, Any, int]] = []
    for k, g in by_form.items():
        sigs = Counter[Any]()
        for n, _, (s, _) in g:
            sigs[s] += n
        if len(sigs) > 1:
            many.append((sum(sigs.values()), k, len(sigs)))
            continue
        if len(g) < 2:
            continue
        first = g[0][2][1]
        for pos, it in enumerate(first):                  # each varying argument of each item
            if it[0] == "raw":
                if any(items[pos] != it for _, _, (_, items) in g):
                    holes["raw word"] += 1
                continue
            for ai, arg in enumerate(it[2]):
                if any(items[pos][2][ai] != arg for _, _, (_, items) in g):
                    found = hole_expression([(items[pos][2][ai], values) for _, values, (_, items) in g])
                    top_name = found[0] if found else "other"
                    holes[f"{it[0]} {top_name}"] += 1
                    if top_name == "other" and len(others) < top:
                        others.append((k[0], it[1], ai, [(items[pos][2][ai], values) for _, values, (_, items) in g[:3]]))
    single = [k for k, g in by_form.items() if len(g) == 1]
    print(f"forms of one distinct instruction (their holes not shown by variation): {len(single)}, "
          f"{sum(by_form[k][0][0] for k in single)} compiled instructions")
    print(f"holes in one-sequence forms of two or more instructions, by the expression of one value that gives them in all: {dict(holes)}")
    for ic, enc, ai, samples in others:
        print(f"  other: {ic} {enc} argument {ai}: " + "; ".join(f"{arg} in {vals}" for arg, vals in samples))
    structures = Counter[Any]()                          # each distinct encoder sequence, by compiled instructions
    for g in by_form.values():
        for n, _, (sig, _) in g:
            structures[sig] += n
    by_len = Counter[int]()
    for sig in structures:
        by_len[len(sig)] += 1
    print(f"distinct encoder sequences (stencils): {len(structures)}; the commonest 100 cover "
          f"{100 * sum(n for _, n in structures.most_common(100)) / max(compiled, 1):.1f}% of compiled instructions, "
          f"the commonest 1,000 {100 * sum(n for _, n in structures.most_common(1000)) / max(compiled, 1):.1f}%; "
          f"items a stencil: {dict(sorted(by_len.items())[:12])} ...")
    many.sort(key=lambda x: -x[0])
    print(f"forms whose values choose the structure: {len(many)}, {sum(n for n, _, _ in many)} compiled instructions")
    by_iclass = Counter[str]()
    for n, k, _ in many:
        by_iclass[k[1] if decoder_key else k[0]] += n
    print("  by iclass: " + ", ".join(f"{ic} {n}" for ic, n in by_iclass.most_common(top)))
    for n, k, s in many[:top]:
        print(f"  {n:>8} {s:>3} sequences: {k[:2] if decoder_key else k[:5]}")
    return 0


def _item_words(items: list[Any], words: list[int]) -> list[int | None]:
    """The word at each item's start (a branch item's own word)."""
    out: list[int | None] = []
    i = 0
    for it in items:
        out.append(words[i] if i < len(words) else None)
        if it[0] in ("branch", "raw", "enc32"):
            i += 1
        else:
            i += _width(it)
    return out


_widths: dict[tuple[Any, ...], int] = {}


def _width(it: Any) -> int:
    key = (it[1], it[2])
    if key not in _widths:
        pos = tuple(a for a in it[2] if not (isinstance(a, tuple) and len(a) == 2 and isinstance(a[0], str)))
        kw = dict(a for a in it[2] if isinstance(a, tuple) and len(a) == 2 and isinstance(a[0], str))
        w = getattr(a64_encoder, it[1]).__wrapped__(*pos, **kw)
        _widths[key] = len(w) if isinstance(w, list) else 1
    return _widths[key]


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("record", type=Path)
    ap.add_argument("--top", type=int, default=30)
    ap.add_argument("--refine", action="store_true", help="split each form by its values' classes (value_class)")
    ap.add_argument("--decoder-key", action="store_true",
                    help="key each instruction by its decoded fields (decoded_form; needs a record with bytes)")
    a = ap.parse_args(argv)
    with open(a.record, "rb") as f:
        record = pickle.load(f)
    return census(record, a.top, a.refine, a.decoder_key)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
