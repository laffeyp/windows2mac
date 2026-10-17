# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""gen_stencil_table.py — render Sprint 076's stencil table (scripts/stencil_table.py --out) as the C data of the
compiler inside the VM, tools/vm_compiler/stencil_table.h (Sprint 077; BLACKBOARD Decision 2026-10-14).

The key: stencil_census.decoded_form's key written as bytes (`key_bytes`), which tools/vm_compiler/vm_compiler.c
writes the same way from the decoder's output; exit_va is left out (it reaches the words only through the branches
to it, which the emitter computes). Keys are found by their FNV-1a hash and checked byte for byte.

Each item's arguments are mapped to the C encoder's positional parameters (tools/vm_compiler/a64_encoder.h): keyword
arguments placed, defaults filled, a name as its enum index, a system register as its five fields, a boolean as 0
or 1. An argument is a constant, a hole (an expression of stencil_census.EXPRESSIONS over one decoded value), a
runtime symbol's address, or, in a branch to the exit, the exit's offset from the branch.

The table is keyed by the form, the key without its flag arguments (Sprint 078): each form holds the facts the block
compiler reads before an instruction's flags are known, derived from every recorded instruction of the form, and its
stencils by flag arguments.

It also writes tests/data/stencil_samples.txt.gz: for each key and each of its stencils, the first recorded
instruction that reaches it (its bytes, address and flag arguments), and tests/data/block_samples.txt.gz, blocks made
from the record; tests/test_vm_compiler_c.py compiles both with both compilers.

Usage: uv run python scripts/gen_stencil_table.py [--table store/stencil/table.txt.gz] [--record store/stencil/record.txt.gz]
       [--check]
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import inspect
import random
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "scripts"))

import stencil_census as sc  # noqa: E402
import stencil_table as st  # noqa: E402
from w2m import a64_encoder, vm_backend, x86dec, x86ops  # noqa: E402
from w2m.lift import lift  # noqa: E402

OUT = ROOT / "tools" / "vm_compiler" / "stencil_table.h"
SAMPLES = ROOT / "tests" / "data" / "stencil_samples.txt.gz"
BLOCK_SAMPLES = ROOT / "tests" / "data" / "block_samples.txt.gz"
SCAN_SAMPLES = ROOT / "tests" / "data" / "scan_samples.txt.gz"     # successor_scan_judge.scan_samples (Sprint 082)
BLOCKS = 3000
DECODER_TABLES = ROOT / "tools" / "vm_compiler" / "x86_decode_tables.h"
ENCODER_HEADER = ROOT / "tools" / "vm_compiler" / "a64_encoder.h"

# the decoder's register classes (x86_decode.h enum x86_register_class) and their names, as x86ops writes them
CLASS_NAMES: list[list[str]] = [[], list(x86ops.GPR64), list(x86ops.GPR32), list(x86ops.GPR16), list(x86ops.GPR8_REX),
                                list(x86ops.GPR8_LEGACY), list(x86ops.XMM), list(x86ops.YMM), ["RIP"], ["EIP"]]
LETTERS = ["q", "d", "w", "b", "x", "y", "h"]           # a register's class in the key: codes 1-7
KEPT = sorted({n for names in CLASS_NAMES for n in names
               if n in sc.SPECIAL or n not in sc.REGISTER_NUMBERS})   # a register kept by name: codes 16 and up
SEGMENTS = {"INVALID": 0, "FS": 1, "GS": 2}
NZCV_FROM: list[str | None] = [None, *vm_backend.NZCV_CONDITIONS]
OP_TAGS = {"R": 1, "M": 2, "A": 3, "I": 4, "J": 5, "MM": 6, "ST": 7}

# an argument: tag << 28 | payload
T_SMALL, T_POOL, T_HOLE, T_SYMBOL, T_EXIT, T_CHECKED = range(6)
SMALL = 1 << 27
ITEM_KINDS = {"enc": 0, "enc32": 1, "branch": 2}
NEAR = sc.NEAR                                           # compile_logged's base: exit_va - NEAR
FEATURE_NONE = 0xFF
STRING_TABLES: dict[str, dict[str, list[str]]] = {      # a64_encoder.h's enums, in the Python tables' order
    "cset": {"cond": list(a64_encoder.COND)}, "bcond": {"cond": list(a64_encoder.COND)},
    "fcm": {"cond": ["eq", "ge", "gt"]},
    "three_same": {"name": list(a64_encoder.THREE_SAME)}, "three_same_d": {"name": list(a64_encoder.THREE_SAME)},
    "fp_scalar": {"op": list(a64_encoder._FP2)}, "fp_vector": {"op": list(a64_encoder._FPV)},
    "fp_fused": {"op": list(a64_encoder._FP3)},
}


def names_of(symbol: str) -> list[str]:
    """A name array of x86_decode_tables.h (under X86_DECODE_NAMES), in index order."""
    text = DECODER_TABLES.read_text()
    start = text.index(f"const char *const {symbol}[")
    body = text[text.index("{", start) + 1:text.index("};", start)]
    return re.findall(r'"([^"]*)"', body)


_ROWS = {n: i for i, n in enumerate(names_of("x86_row_name"))}
_ICLASSES = {n: i for i, n in enumerate(names_of("x86_iclass_name"))}


def register_code(r: Any) -> int:
    """A register in the key's shape: 0 none, 1-7 a class letter, 16 and up a register kept by name."""
    if r is None:
        return 0
    if r in LETTERS:
        return 1 + LETTERS.index(r)
    return 16 + KEPT.index(r)


def value_class_bits(c: tuple[Any, ...]) -> int:
    zero, neg, r4096, pm4096, r9, scaled, halves, s1, s2, s3, s4 = c
    bits = [zero, neg, r4096, pm4096, r9, *scaled, *halves, s1, s2, s3, s4]
    return sum(int(b) << i for i, b in enumerate(bits))


def key_bytes(key: tuple[Any, ...]) -> bytes:
    """decoded_form's key as the C writes it (vm_compiler.c vmc_key)."""
    row, iclass, opsize, asize, vl, shape, same, named, classes, nzcv_from, live_after, nzcv_out, _exit = key
    out = bytearray()
    out += _ROWS[row].to_bytes(2, "little") + _ICLASSES[iclass].to_bytes(2, "little")
    out += bytes([opsize, asize, vl, len(shape)])
    for s in shape:
        if isinstance(s, str):                           # MM:n, ST:n
            kind, n = s.split(":")
            out += bytes([OP_TAGS[kind], int(n), 0, 0, 0, 0])
        elif s[0] == "R":
            out += bytes([OP_TAGS["R"], register_code(s[1]), 0, 0, 0, 0])
        elif s[0] in ("M", "A"):
            _, seg, base, index, scale, rest = s
            out += bytes([OP_TAGS[s[0]], SEGMENTS[seg or "INVALID"], register_code(base), register_code(index), scale,
                          int(rest[0]) if rest else 0])
        elif s[0] == "I":
            out += bytes([OP_TAGS["I"], s[1], 0, 0, 0, 0])
        else:
            out += bytes([OP_TAGS["J"], 0, 0, 0, 0, 0])
    out.append(len(same))
    out += bytes(same)
    for n in named:
        out += (0xFFFF if n is None else n).to_bytes(2, "little")
    out.append(len(classes))
    for c in classes:
        out += value_class_bits(c).to_bytes(3, "little")
    out += bytes([NZCV_FROM.index(nzcv_from), live_after, int(nzcv_out)])
    return bytes(out)


def fnv1a(b: bytes) -> int:
    h = 0xCBF29CE484222325
    for x in b:
        h = (h ^ x) * 0x100000001B3 & (1 << 64) - 1
    return h


# ---------------------------------------------------------------------------------------------
# the C equal of stencil_census.EXPRESSIONS and stencil_table.FEATURES, from their names

def c_expression(name: str) -> str:
    """The C of one hole expression over u64 u (the value's bits) and s64 v (the same bits, signed)."""
    fixed = {"v": "u", "v mod 2^64": "u", "xmm": "u - 1000", "low 12 bits": "u & 0xFFF",
             "bits 12-23": "u >> 12 & 0xFFF", "-v": "-u", "-v mod 2^64": "-u", "-v mod 2^32": "-u & 0xFFFFFFFFu",
             "16 x xmm": "16 * (u - 1000)", "v - 1": "u - 1"}
    if name in fixed:
        return fixed[name]
    if m := re.fullmatch(r"halfword (\d)", name):
        return f"u >> {16 * int(m[1])} & 0xFFFF"
    if m := re.fullmatch(r"masked to (\d+) bits", name):
        return f"u & 0x{(1 << int(m[1])) - 1:X}u"
    if m := re.fullmatch(r"low (\d) bits", name):
        return f"u & {(1 << int(m[1])) - 1}"
    if m := re.fullmatch(r"low (\d) bits - 1", name):
        return f"(u & {(1 << int(m[1])) - 1}) - 1"
    if m := re.fullmatch(r"(\d+) - low (\d) bits", name):
        return f"{m[1]} - (u & {(1 << int(m[2])) - 1})"
    if m := re.fullmatch(r"(\d+) - v", name):
        return f"{m[1]} - u"
    if m := re.fullmatch(r"bits (\d)\+(\d)", name):
        return f"u >> {m[1]} & {(1 << int(m[2])) - 1}"
    if m := re.fullmatch(r"min\(v, (\d+)\)", name):
        return f"(u64)(v < {m[1]} ? v : {m[1]})"
    raise SystemExit(f"no C for the hole expression {name!r}")


def c_feature(name: str) -> str:
    if m := re.fullmatch(r"at least (\d+)", name):
        return f"(u & 0xFF) >= {m[1]}"
    if m := re.fullmatch(r"low (\d) bits", name):
        return f"u & {(1 << int(m[1])) - 1}"
    raise SystemExit(f"no C for the feature {name!r}")


EXPRESSION_ID = {name: i for i, (_, name, _) in enumerate(sc.EXPRESSIONS)}
FEATURE_ID = {name: i for i, (name, _) in enumerate(st.FEATURES)}


# ---------------------------------------------------------------------------------------------
# the encoders: Python parameters to C parameters

def c_prototypes() -> dict[str, tuple[str, int]]:
    """Each C encoder: (its return type, its parameter count)."""
    out = {}
    for ret, name, params in re.findall(r"^(u32|int) a64_(\w+)\((.*?)\);", ENCODER_HEADER.read_text(), re.M):
        out[name] = (ret, 0 if params.strip() == "void" else len(params.split(",")))
    return out


def python_encoder(name: str) -> Any:
    f = getattr(a64_encoder, name)
    return getattr(f, "__wrapped__", f)


def c_name(name: str) -> str:
    return name.rstrip("_")


def c_arguments(name: str, specs: tuple[Any, ...]) -> list[Any]:
    """The item's argument specs in the C encoder's parameter order."""
    params = list(inspect.signature(python_encoder(name)).parameters.values())
    positional = [s for s in specs if s[0] != "kw"]
    by_name = {s[1]: s[2] for s in specs if s[0] == "kw"}
    out: list[Any] = []
    for i, p in enumerate(params):
        spec = positional[i] if i < len(positional) else by_name.get(p.name)
        if spec is None:
            assert p.default is not inspect.Parameter.empty, (name, p.name)
            spec = ("c", p.default)
        if name in STRING_TABLES and p.name in STRING_TABLES[name]:
            assert spec[0] == "c" and isinstance(spec[1], str), (name, spec)
            table = STRING_TABLES[name][p.name]
            out.append(("c", a64_encoder.COND[spec[1]] if p.name == "cond" and name != "fcm" else table.index(spec[1])))
        elif p.name == "reg":
            assert spec[0] == "c" and isinstance(spec[1], tuple), (name, spec)
            out += [("c", f) for f in spec[1]]
        elif spec[0] == "c" and isinstance(spec[1], bool):
            out.append(("c", int(spec[1])))
        else:
            out.append(spec)
    return out


class Render:
    """The table's pools, filled as stencils are added."""

    def __init__(self) -> None:
        self.args: list[int] = []
        self.pool: dict[int, int] = {}
        self.items: dict[tuple[Any, ...], int] = {}
        self.item_rows: list[tuple[int, int, int, int]] = []          # encoder, kind, count, first argument
        self.stencils: dict[tuple[int, ...], int] = {}
        self.stencil_rows: list[tuple[int, int]] = []                 # first item reference, count
        self.stencil_items: list[int] = []
        self.encoders: dict[str, int] = {}
        self.symbols: dict[str, int] = {}
        self.kinds: dict[str, int] = {}
        self.checked: dict[tuple[Any, ...], int] = {}             # (expression, position, alternatives): row
        self.alts: list[int] = []

    def arg(self, spec: Any, kind: str) -> int:
        tag = spec[0]
        if kind == "branch" and tag == "c" and NEAR - 4 * 4096 < spec[1] <= NEAR:   # an offset to the exit
            self.kinds["exit"] = self.kinds.get("exit", 0) + 1
            return T_EXIT << 28
        self.kinds[tag if tag != "h" else "h " + spec[1]] = self.kinds.get(tag if tag != "h" else "h " + spec[1], 0) + 1
        if tag == "c":
            v = spec[1]
            assert isinstance(v, int), spec
            if kind == "branch":
                assert -4096 < v < 4096, spec                # a branch inside the instruction
            if -SMALL <= v < SMALL:
                return T_SMALL << 28 | v + SMALL
            bits = v & (1 << 64) - 1
            return T_POOL << 28 | self.pool.setdefault(bits, len(self.pool))
        if tag == "h" and len(spec) == 4:                 # alternatives that must agree (stencil_table._hole)
            k = (EXPRESSION_ID[spec[1]], int(spec[2]), tuple(EXPRESSION_ID[a] for a in spec[3]))
            if k not in self.checked:
                self.checked[k] = len(self.checked)
                self.alts.extend(k[2])
            return T_CHECKED << 28 | self.checked[k]
        if tag == "h":
            return T_HOLE << 28 | EXPRESSION_ID[spec[1]] << 8 | int(spec[2])
        if tag == "sym":
            return T_SYMBOL << 28 | self.symbols.setdefault(spec[1], len(self.symbols))
        raise SystemExit(f"an argument with no C form: {spec}")

    def item(self, kind: str, name: str, specs: tuple[Any, ...]) -> int:
        cargs = c_arguments(name, specs)
        codes = tuple(self.arg(s, kind) for s in cargs)
        enc = self.encoders.setdefault(name, len(self.encoders))
        k = (ITEM_KINDS[kind], enc, codes)
        if k not in self.items:
            self.items[k] = len(self.item_rows)
            self.item_rows.append((enc, ITEM_KINDS[kind], len(codes), len(self.args)))
            self.args.extend(codes)
        return self.items[k]

    def stencil(self, stencil: Any) -> int:
        ids = tuple(self.item(kind, name, specs) for kind, name, specs in stencil)
        if ids not in self.stencils:
            self.stencils[ids] = len(self.stencil_rows)
            self.stencil_rows.append((len(self.stencil_items), len(ids)))
            self.stencil_items.extend(ids)
        return self.stencils[ids]


def _checked_rows(r: Render) -> list[str]:
    rows, at = [], 0
    for (expr, pos, alts), _ in sorted(r.checked.items(), key=lambda x: x[1]):
        rows.append(f"    {{{expr}, {pos}, {at}, {len(alts)}}},")
        at += len(alts)
    return rows


def _array(ctype: str, name: str, values: list[Any], per_line: int = 16) -> list[str]:
    lines = [f"static const {ctype} {name}[{max(len(values), 1)}] = {{"]
    for i in range(0, len(values), per_line):
        lines.append("    " + ", ".join(str(v) for v in values[i:i + per_line]) + ",")
    if not values:
        lines.append("    0,")
    return lines + ["};"]


FLAGS_BYTES = 3                                          # key_bytes ends with nzcv_from, live_after, nzcv_out


def form_bytes(key: tuple[Any, ...]) -> bytes:
    """The key without its flag arguments: the form, which the block reads its facts from (Sprint 078)."""
    return key_bytes(key)[:-FLAGS_BYTES]


BRANCH_NONE, BRANCH_ALWAYS, BRANCH_OTHER = 0, 1, 2     # the first br op's kind (vm_compiler.c VMC_BRANCH_*)
TARGET_NONE, TARGET_NOT_A_VALUE = 255, 254              # no constant target; a constant no decoded value holds


def branch_facts(ins: Any, values: list[int]) -> tuple[int, int]:
    """What the successor scan reads of an instruction's branch (VM._flags_live_out, _flags_live_in; Sprint 082): the
    first br op's kind, and the position among the decoded values (the last that holds it) of its target when the
    target is a constant, as both read it (`constant[br[2]]`)."""
    br = next((op for op in ins.ops if op[0] == "br"), None)
    if br is None:
        return BRANCH_NONE, TARGET_NONE
    kind = BRANCH_ALWAYS if br[1] == "always" else BRANCH_OTHER
    constant = {op[1]: op[2] for op in ins.ops if op[0] == "imm"}
    if br[2] not in constant:
        return kind, TARGET_NONE
    target = constant[br[2]] & (1 << 64) - 1
    held = [i for i, v in enumerate(values) if v & (1 << 64) - 1 == target]
    return kind, held[-1] if held else TARGET_NOT_A_VALUE


def facts_of(ins: Any, values: list[int]) -> tuple[int, ...]:
    """What the block compiler reads of an instruction before its flags are known (Monitor._compile, assemble_block):
    ends the block (a branch), falls through (no unconditional branch), the kind of NZCV it leaves (nzcv_setter, as
    NZCV_FROM's index), the kinds its Jcc branches on (nzcv_branch: bit i for NZCV_FROM[i]), whether it has a flags op
    (stencil_census.canonical_flags), its branch for the successor scan (branch_facts), then the flags read and
    written for each nzcv_from kind (insn_flags)."""
    brs = [op for op in ins.ops if op[0] == "br"]
    ok = sum(1 << i for i, kind in enumerate(NZCV_FROM)
             if kind is not None and any(op[1] in vm_backend.NZCV_CONDITIONS[kind] for op in brs))
    flags = [vm_backend.insn_flags(ins, kind) for kind in NZCV_FROM]
    return (int(bool(brs)), int(not any(op[1] == "always" for op in brs)),
            NZCV_FROM.index(vm_backend.nzcv_setter(ins)), ok, int(any(op[0] == "flags" for op in ins.ops)),
            *branch_facts(ins, values), *(r for r, _ in flags), *(w for _, w in flags))


def record_facts(record: dict[str, Any], implicit: dict[str, frozenset[int]]) -> tuple[dict[bytes, tuple[int, ...]],
                                                                                        list[Any]]:
    """Each form's facts over every recorded instruction, and the forms whose instructions disagree."""
    facts: dict[bytes, tuple[int, ...]] = {}
    disagree = []
    for ins, codes in record["bytes"].items():
        for code in codes:
            if lift(code, ins.address) != ins:
                continue                                 # Sprint 074's relift check: the bytes give another Lifted
            key, values, _ = sc.decoded_form((ins, 0, None, 0, False), code,
                                             implicit.get(x86dec.decode(code).row or "", frozenset()))
            fb, f = form_bytes(key), facts_of(ins, values)
            if facts.setdefault(fb, f) != f:
                disagree.append((key[1], code.hex(), facts[fb], f))
    return facts, disagree


def render(data: dict[str, Any], digest: str, facts: dict[bytes, tuple[int, ...]]) -> tuple[str, dict[str, Any]]:
    table, implicit = data["table"], data["implicit"]
    r = Render()
    variants: dict[bytes, list[tuple[Any, ...]]] = {}    # form: (nzcv_from, live_after, nzcv_out, feature, position,
    for key, entry in table.items():                     # choices)
        if key[1] in vm_backend.HOST_LOWERED:            # lowered anew since the table (Sprints 096, 098): the host's
            continue
        if entry[0] == "one":
            choices = [(0, r.stencil(entry[1]), entry[2])]
            feature, pos = FEATURE_NONE, 0
        else:
            _, pos, fname, by = entry
            feature = FEATURE_ID[fname]
            choices = sorted((int(fv), r.stencil(st_), gs) for fv, (st_, gs) in by.items())
        flags = key_bytes(key)[-FLAGS_BYTES:]
        variants.setdefault(form_bytes(key), []).append((*flags, feature, pos, choices))
    for fb, vs in variants.items():                     # a form's variants are unique by flag arguments
        assert len({v[:FLAGS_BYTES] for v in vs}) == len(vs), fb.hex()
    forms = sorted(((fnv1a(fb), fb, sorted(v)) for fb, v in variants.items()), key=lambda e: e[0])
    hashes = [e[0] for e in forms]
    assert len(set(hashes)) == len(hashes), "two forms with one hash"
    keys_blob: list[int] = []
    choice_rows: list[str] = []
    guard_rows: list[str] = []
    variant_rows: list[str] = []
    form_rows = []
    for h, fb, vs in forms:
        ends, falls, setter, ok, has_flags, branch, target, *rw = facts[fb]
        form_rows.append(f"    {{0x{h:016x}u, {len(keys_blob)}, {len(fb)}, {ends}, {falls}, {setter}, {ok}, {has_flags}, "
                         f"{branch}, {target}, "
                         f"{{{', '.join(map(str, rw[:len(NZCV_FROM)]))}}}, {{{', '.join(map(str, rw[len(NZCV_FROM):]))}}}, "
                         f"{len(variant_rows)}, {len(vs)}}},")
        keys_blob.extend(fb)
        for nzcv_from, live_after, nzcv_out, feature, pos, choices in vs:
            variant_rows.append(f"    {{{nzcv_from}, {live_after}, {nzcv_out}, {feature}, {pos}, {len(choice_rows)}, "
                                f"{len(choices)}}},")
            for fv, stencil_id, gs in choices:
                choice_rows.append(f"    {{{fv}, {stencil_id}, {len(guard_rows)}, {len(gs)}}},")
                guard_rows.extend(f"    {{{p}, 0x{v & (1 << 64) - 1:x}u}}," for p, v in gs)
    protos = c_prototypes()
    dispatch = []
    for name, e in sorted(r.encoders.items(), key=lambda x: x[1]):
        ret, count = protos[c_name(name)]
        if ret == "int":                                 # mov_imm: its words and their count
            assert count == 3
            dispatch.append(f"    case {e}: return a64_mov_imm(out, (int)a[0], a[1]);")
        else:
            call = ", ".join(f"(int)a[{i}]" for i in range(count))
            dispatch.append(f"    case {e}: out[0] = a64_{c_name(name)}({call}); return out[0] ? 1 : 0;   /* {name} */")
    for (kind, e, codes) in r.items:                     # every item's argument count is its encoder's
        name = next(n for n, i in r.encoders.items() if i == e)
        assert len(codes) == protos[c_name(name)][1] - (protos[c_name(name)][0] == "int"), name
    implicit_masks = [0] * len(_ROWS)
    for row, regs in implicit.items():
        if row in _ROWS:
            m = 0
            for n in regs:
                assert 0 <= n < 16 or 1000 <= n < 1016, (row, n)
                m |= 1 << (n if n < 16 else 16 + n - 1000)
            implicit_masks[_ROWS[row]] = m
    reg_code = [[0] * 32 for _ in CLASS_NAMES]
    reg_number = [[0] * 32 for _ in CLASS_NAMES]
    for c, names in enumerate(CLASS_NAMES):
        for i, n in enumerate(names):
            if n in sc.SPECIAL or n not in sc.REGISTER_NUMBERS:
                reg_code[c][i] = register_code(n)
            else:
                letter, number = sc.REGISTER_NUMBERS[n]
                reg_code[c][i], reg_number[c][i] = register_code(letter), number
    symbols = [n for n, _ in sorted(r.symbols.items(), key=lambda x: x[1])]
    pool = [f"0x{v:x}u" for v, _ in sorted(r.pool.items(), key=lambda x: x[1])]
    out = [
        "// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0",
        "// Copyright (C) 2026 Peter Laffey",
        f"/* GENERATED by scripts/gen_stencil_table.py from store/stencil ({digest}).",
        " * Do not edit; regenerate. Included once, by tools/vm_compiler/vm_compiler.c. */",
        "",
        f"#define VMC_FORMS {len(forms)}",
        f"#define VMC_KEYS {len(table)}",
        f"#define VMC_STENCILS {len(r.stencil_rows)}",
        f"#define VMC_ITEMS {len(r.item_rows)}",
        f"#define VMC_SYMBOLS {len(symbols)}",
        f"#define VMC_EXPRESSIONS {len(sc.EXPRESSIONS)}",
        f"#define VMC_FEATURES {len(st.FEATURES)}",
        f"#define VMC_FEATURE_NONE {FEATURE_NONE}",
        f"#define VMC_ROWS {len(_ROWS)}",
        f"#define VMC_NZCV_FROM {len(NZCV_FROM)}",
        f"#define VMC_MAX_BLOCK_INSNS {vm_backend.MAX_BLOCK_INSNS}",
        f"#define VMC_LIVENESS_SCAN {vm_backend.LIVENESS_SCAN}",
        f"#define VMC_PROCESS_VA 0x{vm_backend.PROCESS_VA:x}ull",
        f"#define VMC_PAGE 0x{vm_backend.PAGE:x}ull",
        f"#define VMC_ALL_FLAGS {vm_backend.ALL_FLAGS}",
        f"#define VMC_REG_BUDGET {vm_backend.REG_BUDGET}",
        "",
        "/* a register's code in the key (0 none, 1-7 the class letters q d w b x y h, 16 and up a register kept by name:",
        f" * {' '.join(KEPT)}) and its number in the values, by decoder class and number */",
        "static const u8 VMC_REG_CODE[10][32] = {",
        *("    {" + ", ".join(map(str, row)) + "}," for row in reg_code),
        "};",
        "static const u16 VMC_REG_NUMBER[10][32] = {",
        *("    {" + ", ".join(map(str, row)) + "}," for row in reg_number),
        "};",
        "",
        "/* each spec row's implicit registers (stencil_census.implicit_registers): bit n for general register n,",
        " * bit 16 + n for XMM register n */",
        *_array("u32", "VMC_IMPLICIT", [f"0x{m:x}" for m in implicit_masks]),
        "",
        "/* a hole's expression of one value (stencil_census.EXPRESSIONS, in order) */",
        "static u64 vmc_expression(int e, u64 u) {",
        "    s64 v = (s64)u;",
        "    (void)v;",
        "    switch (e) {",
        *(f"    case {i}: return {c_expression(name)};   /* {name} */" for i, (_, name, _) in enumerate(sc.EXPRESSIONS)),
        "    }",
        "    return 0;",
        "}",
        "",
        "/* a refinement's feature of one value (stencil_table.FEATURES, in order) */",
        "static u64 vmc_feature(int f, u64 u) {",
        "    switch (f) {",
        *(f"    case {i}: return {c_feature(name)};   /* {name} */" for i, (name, _) in enumerate(st.FEATURES)),
        "    }",
        "    return 0;",
        "}",
        "",
        "/* an item's encoder called on its arguments: the word count, 0 where the encoder refuses them */",
        "static int vmc_encode(int e, const u64 *a, u32 *out) {",
        "    switch (e) {",
        *dispatch,
        "    }",
        "    return 0;",
        "}",
        "",
        "/* the runtime symbols the table names, in index order */",
        "#ifdef VMC_NAMES",
        f"const char *const vmc_symbol_name[{max(len(symbols), 1)}] = {{",
        *(f'    "{s}",' for s in symbols or [""]),
        "};",
        "#endif",
        "",
        *_array("u64", "VMC_POOL", pool, 8),
        "",
        "/* each argument: tag << 28 | payload (0 a constant + 2^27, 1 a pool index, 2 a hole: expression << 8 | value,",
        " * 3 a symbol, 4 the exit's offset from the branch, 5 a checked hole's row) */",
        *_array("u32", "VMC_ARG", [f"0x{a:x}" for a in r.args], 12),
        "",
        "/* a checked hole: its expression, the value it reads, its first alternative and their count; an instruction on",
        " * which an alternative gives another argument has no stencil (stencil_table._hole) */",
        f"static const struct vmc_checked VMC_CHECKED[{max(len(r.checked), 1)}] = {{",
        *(_checked_rows(r) or ["    {0, 0, 0, 0},"]),
        "};",
        *_array("u8", "VMC_ALT", r.alts, 32),
        "",
        "/* encoder, kind (0 its words, 1 bit 31 cleared, 2 a branch), argument count, first argument */",
        "static const struct vmc_item VMC_ITEM[VMC_ITEMS] = {",
        *(f"    {{{e}, {k}, {n}, {f}}}," for e, k, n, f in r.item_rows),
        "};",
        "",
        *_array("u32", "VMC_STENCIL_ITEM", r.stencil_items, 16),
        "/* first item reference, count */",
        "static const struct vmc_stencil VMC_STENCIL[VMC_STENCILS] = {",
        *(f"    {{{f}, {n}}}," for f, n in r.stencil_rows),
        "};",
        "",
        "/* a key's choices: the feature's value, the stencil, its first guard and their count */",
        f"static const struct vmc_choice VMC_CHOICE[{len(choice_rows)}] = {{",
        *choice_rows,
        "};",
        "",
        "/* a guard: the value at this position must be this one (stencil_table._guards: a value the derivation never",
        " * varied that a constant argument may stand for) */",
        f"static const struct vmc_guard VMC_GUARD[{max(len(guard_rows), 1)}] = {{",
        *(guard_rows or ["    {0, 0},"]),
        "};",
        "",
        *_array("u8", "VMC_FORM_BYTES", keys_blob, 32),
        "",
        "/* a form's stencils by its flag arguments: nzcv_from, live_after, nzcv_out, the feature (VMC_FEATURE_NONE: one",
        " * stencil) and the value it reads, the first choice and their count */",
        f"static const struct vmc_variant VMC_VARIANT[{len(variant_rows)}] = {{",
        *variant_rows,
        "};",
        "",
        "/* the forms by hash: hash, the form's bytes' start and length; its facts (ends the block, falls through, the",
        " * NZCV kind it leaves, the kinds its Jcc branches on, has a flags op, its first branch's kind and the position",
        " * of its constant target among the values, the flags read and written per nzcv_from kind); its first",
        " * variant and their count */",
        "static const struct vmc_form VMC_FORM[VMC_FORMS] = {",
        *form_rows,
        "};",
        "",
    ]
    stats = {"keys": len(table), "forms": len(forms), "variants": len(variant_rows), "choices": len(choice_rows),
             "guards": len(guard_rows), "checked holes": len(r.checked), "choices with guards": sum(1 for c in choice_rows if not c.endswith(", 0},")), "stencils": len(r.stencil_rows), "items": len(r.item_rows),
             "arguments": len(r.args), "pool": len(pool), "key bytes": len(keys_blob), "encoders": len(r.encoders),
             "symbols": symbols, "argument kinds": r.kinds,
             "branch forms": sum(1 for _, fb, _ in forms if facts[fb][5] != BRANCH_NONE),
             "constant targets": sum(1 for _, fb, _ in forms if facts[fb][6] < TARGET_NOT_A_VALUE),
             "constant targets no value holds": sum(1 for _, fb, _ in forms if facts[fb][6] == TARGET_NOT_A_VALUE)}
    return "\n".join(out), stats


def samples(record: dict[str, Any], data: dict[str, Any]) -> list[str]:
    """For each key and each of its stencils, the first recorded call that reaches it and passes its guards, else an
    instance of the key that does: "bytes address nzcv_from live_after nzcv_out" (nzcv_from as NZCV_FROM's index)."""
    table, implicit = data["table"], data["implicit"]
    seen: set[tuple[Any, Any]] = set()
    first: dict[Any, tuple[Any, bytes]] = {}
    lines = []
    for call in record["calls"]:
        code = next(iter(record["bytes"][call[0]]))
        key, values, _ = sc.decoded_form(call, code, implicit.get(x86dec.decode(code).row or "", frozenset()))
        entry = table.get(key)
        if entry is None:
            continue
        choice = None if entry[0] == "one" else st.FEATURE[entry[2]](values[entry[1]])
        if (key, choice) in seen or st.stencil_for(entry, values) is None:
            continue                                     # (a guard the values fail: the host compiles it)
        seen.add((key, choice))
        first.setdefault(key, (call, code))
        lines.append(_sample(key, call, code))
    # a refined key's stencils the record never reached: from instances of its first recorded call
    for key, (call, code) in first.items():
        entry = table[key]
        if entry[0] == "one":
            continue
        row = x86dec.decode(code).row or ""
        missing = set(entry[3]) - {c for k, c in seen if k == key}
        for seed in range(8):
            for c, b, values in st.instances(call, code, key, implicit.get(row, frozenset()), random.Random(seed),
                                             8 * st.TRIES, 8 * st.KEEP)[1:]:
                choice = st.FEATURE[entry[2]](values[entry[1]])
                if choice in missing and st.stencil_for(entry, values) is not None:
                    missing.discard(choice)
                    seen.add((key, choice))
                    lines.append(_sample(key, c, b))
            if not missing:
                break
    return lines


def block_samples(record: dict[str, Any]) -> list[str]:
    """Blocks made from the record (Sprint 078): BLOCKS recorded addresses drawn at random, each with its recorded
    successors on, up to and including a branch and at most MAX_BLOCK_INSNS; the flags live at its end drawn from
    those its last instruction was recorded with where a branch or the count ends it, else every flag:
    "start live_out bytes"."""
    by_addr: dict[int, tuple[Any, bytes]] = {}
    for ins, codes in record["bytes"].items():
        for code in codes:
            if lift(code, ins.address) == ins:
                by_addr.setdefault(ins.address, (ins, code))
    lives: dict[int, set[int]] = {}
    for call in record["calls"]:
        lives.setdefault(call[0].address, set()).add(call[3])
    rng = random.Random(78)
    lines = []
    for start in rng.sample(sorted(by_addr), BLOCKS):
        addr, codes, last, ended = start, [], None, False
        while len(codes) < vm_backend.MAX_BLOCK_INSNS and addr in by_addr:
            ins, code = by_addr[addr]
            codes.append(code)
            last, addr = ins, addr + ins.length
            if any(op[0] == "br" for op in ins.ops):
                ended = True
                break
        else:
            ended = len(codes) == vm_backend.MAX_BLOCK_INSNS
        assert last is not None
        live = rng.choice(sorted(lives[last.address])) if ended else vm_backend.ALL_FLAGS
        lines.append(f"{start:x} {live} {b''.join(codes).hex()}")
    return lines


def _sample(key: Any, call: Any, code: bytes) -> str:
    nzcv_from, live_after, nzcv_out = key[9:12]          # the key's flag arguments: canonical (canonical_flags)
    return f"{code.hex()} {call[0].address:x} {NZCV_FROM.index(nzcv_from)} {live_after} {int(nzcv_out)}"


STORE = ROOT / "store" / "stencil"
TABLE = STORE / "table.txt.gz"                          # scripts/stencil_table.py --out, from RECORD
RECORD = STORE / "record.txt.gz"                        # stencil_census.write_record of the batch run's record


def gzip_bytes(lines: list[str]) -> bytes:
    """The lines gzipped with no timestamp, so the same lines give the same bytes."""
    import io
    buf = io.BytesIO()
    with gzip.GzipFile(fileobj=buf, mode="wb", mtime=0, filename="") as g:
        g.write(("\n".join(lines) + "\n").encode())
    return buf.getvalue()


def outputs(table_path: Path, record_path: Path) -> dict[Path, bytes]:
    """Every file this script writes, from the committed table and record."""
    data = st.read_table(table_path)
    record = sc.read_record(record_path)
    facts, disagree = record_facts(record, data["implicit"])
    if disagree:
        raise SystemExit(f"recorded instructions whose facts disagree with their form's: {len(disagree)} {disagree[:5]}")
    digest = (f"table sha256:{hashlib.sha256(table_path.read_bytes()).hexdigest()}, "
              f"record sha256:{hashlib.sha256(record_path.read_bytes()).hexdigest()}")
    text, stats = render(data, digest, facts)
    print(f"{OUT.name}: {stats}")
    import successor_scan_judge
    return {OUT: text.encode(), SAMPLES: gzip_bytes(samples(record, data)), BLOCK_SAMPLES: gzip_bytes(block_samples(record)),
            SCAN_SAMPLES: gzip_bytes(successor_scan_judge.scan_samples(record, 4000, 8201))}


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--table", type=Path, default=TABLE)
    ap.add_argument("--record", type=Path, default=RECORD)
    ap.add_argument("--check", action="store_true", help="exit 1 when a committed output differs")
    a = ap.parse_args(argv)
    out = outputs(a.table, a.record)
    if a.check:
        stale = [p for p, b in out.items() if not p.exists() or p.read_bytes() != b]
        for p in stale:
            print(f"stale: {p}; run scripts/gen_stencil_table.py", file=sys.stderr)
        return 1 if stale else 0
    for p, b in out.items():
        p.parent.mkdir(exist_ok=True)
        p.write_bytes(b)
        print(f"wrote {p} ({len(b)} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
