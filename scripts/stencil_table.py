# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The stencil table from instances made per key (Sprint 076; BLACKBOARD Decision 2026-10-14, the stencil key).

For each key of a record (stencil_census.decoded_form over each recorded instruction's bytes) the script makes
instances of the key: the recorded instruction with its decoded fields changed in its bytes (ModRM reg and rm, REX.R,
X and B, SIB, the displacement and immediate bytes, VEX.vvvv and VEX's R, X and B) and its address changed, kept when
the bytes decode (to their whole length) to the same key and lift; a REX byte may be added. Each instance is compiled with its encoder calls logged
(stencil_census.compile_logged). The key's stencil is its encoder sequence, and each argument is a constant (the
same in every instance; a runtime symbol's address is named), or the first expression of hole_expression that gives
it in every instance, or `other`.

The judge: a second set of instances per key, from another seed, and the record's own instructions, each rebuilt from
the table (each argument from its constant or expression over the instruction's decoded values) and compared word
for word with the compiler's output.

Usage: uv run python scripts/stencil_table.py <record.pickle> [--out table.pickle] [--workers N]
"""

from __future__ import annotations

import argparse
import multiprocessing
import pickle
import random
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "scripts"))

import stencil_census as sc  # noqa: E402
from w2m import a64_encoder, x86dec  # noqa: E402
from w2m.lift import lift  # noqa: E402

EXPRESSION = {name: fn for _, name, fn in sc.EXPRESSIONS}
TRIES, KEEP = 320, 24                                     # instances tried and kept per key, for the table and the judge

Instance = tuple[Any, bytes, list[int]]                  # (the compile call, its bytes, its decoded values)


def layout(code: bytes, d: x86dec.Decoded) -> dict[str, Any]:
    """Where the decoded fields sit in the bytes."""
    f = d.fields
    assert f is not None
    p = 0
    while p < len(code) and (code[p] in x86dec.LEGACY or 0x40 <= code[p] <= 0x4F):
        p += 1
    imm = d.length - len(f.imm)
    disp = imm - f.disp_size
    sib = disp - (f.sib is not None)
    modrm = sib - (f.modrm is not None)
    payload = {"legacy": 0, "vex": 2 if code[p] == 0xC4 else 1, "evex": 3, "xop": 2}[f.enc]
    return {"enc": f.enc, "rex": p - 1 if f.enc == "legacy" and f.rex and p else None,
            "payload": list(range(p + 1, p + 1 + payload)), "modrm": modrm if f.modrm is not None else None,
            "sib": sib if f.sib is not None else None, "disp": (disp, f.disp_size), "imm": (imm, len(f.imm)),
            "opcode": modrm - 1, "prefix_end": p}


def register_fields(b: bytearray, lay: dict[str, Any]) -> list[tuple[Any, Any]]:
    """Each place in the bytes that names a register, as (read its number, write a number): ModRM reg, ModRM rm in
    its register form, SIB base and index, an opcode's low bits where there is no ModRM, VEX.vvvv. A number's high bit
    sits in REX (R, X, B) or, inverted, in VEX's payload; where neither can hold it, only numbers 0-7 are written."""
    enc, rex, pay = lay["enc"], lay["rex"], lay["payload"]

    def high(bit: int) -> tuple[Any, Any] | None:      # REX bit 2 R, 1 X, 0 B; VEX inverted at 7 R, 6 X, 5 B
        if enc == "legacy" and rex is not None:
            return (lambda: b[rex] >> bit & 1), (lambda h: b.__setitem__(rex, b[rex] & ~(1 << bit) | h << bit))
        if enc in ("vex", "xop") and (len(pay) == 2 or bit == 2):
            vb = 7 - (2 - bit)
            return (lambda: ~b[pay[0]] >> vb & 1), (lambda h: b.__setitem__(pay[0], b[pay[0]] & ~(1 << vb) | (h ^ 1) << vb))
        return None

    def field(at: int, shift: int, bit: int) -> tuple[Any, Any]:
        h = high(bit)

        def read() -> int:
            return (b[at] >> shift & 7) | (h[0]() << 3 if h else 0)

        def write(n: int) -> None:
            b[at] = b[at] & ~(7 << shift) | (n & 7) << shift
            if h:
                h[1](n >> 3 & 1)
        return read, write

    out = []
    m, x = lay["modrm"], lay["sib"]
    if m is not None:
        out.append(field(m, 3, 2))
        if b[m] >> 6 == 3:
            out.append(field(m, 0, 0))
    else:
        out.append(field(lay["opcode"], 0, 0))
    if x is not None:
        out += [field(x, 0, 0), field(x, 3, 1)]
    if enc in ("vex", "xop"):
        v = pay[-1]
        out.append(((lambda: ~b[v] >> 3 & 15), (lambda n: b.__setitem__(v, b[v] & 0x87 | (~n & 15) << 3))))
    return out


def _value_like(old: bytes, rng: random.Random, field: str) -> bytes:
    """A new value of the same width. A displacement mostly moves by a multiple of 16 (its divisibility by the access
    sizes, and so its class, kept); an immediate mostly has bits of its low byte changed (a shuffle's selectors, a
    count). Sometimes a higher byte changes, or the value is small or anything."""
    n = len(old)
    v = int.from_bytes(old, "little", signed=True)
    pick = rng.randrange(10)
    if pick < 5:
        # a displacement by a multiple of 16, 32 or 64 (every value's class holds its divisibility by the access sizes
        # and its low 5 and 6 bits' 0 and 1); an immediate by its low byte's bits, or by 32 or 64 (a count's class)
        new = (v + rng.choice((16, 32, 64)) * rng.randint(-6, 6) if field == "disp" else v ^ rng.randrange(1, 256)
               if pick < 2 else v + rng.choice((32, 64)) * rng.randint(-3, 3) if pick < 4
               else v ^ rng.randrange(1, 256) << 8 * rng.randrange(n))     # a higher byte: a wide mask's bits
    else:
        new = (v + 16 * rng.randint(-1 << 12, 1 << 12) if pick == 5 else
               v ^ rng.randrange(1, 256) << 8 * rng.randrange(n) if pick == 6 else rng.randint(-300, 300)
               if pick == 7 else rng.randrange(1 << 8 * n) if pick == 8 else 0)
    return (new & (1 << 8 * n) - 1).to_bytes(n, "little")


def mutate(code: bytes, lay: dict[str, Any], rng: random.Random, turn: int,
           values_only: bool = False) -> tuple[bytes | None, int, int]:
    """The bytes with one decoded field changed: the field whose turn it is, in a fixed cycle; None on the address's
    turn (the caller changes the address). Returns (bytes, the change's index, the number of changes)."""
    b = bytearray(code)
    changes: list[Any] = []
    value_changes: list[Any] = []
    insert_rex: list[Any] = []
    rename_change: list[Any] = []
    if lay["rex"] is not None:
        changes.append(lambda: b.__setitem__(lay["rex"], b[lay["rex"]] & 0xF8 | rng.randrange(8)))
    if lay["modrm"] is not None:
        m = lay["modrm"]
        changes.append(lambda: b.__setitem__(m, b[m] & 0xC7 | rng.randrange(8) << 3))
        changes.append(lambda: b.__setitem__(m, b[m] & 0xF8 | rng.randrange(8)))
    if lay["sib"] is not None:
        x = lay["sib"]
        changes += [lambda: b.__setitem__(x, b[x] & 0xF8 | rng.randrange(8)),
                    lambda: b.__setitem__(x, b[x] & 0xC7 | rng.randrange(8) << 3),
                    lambda: b.__setitem__(x, b[x] & 0x3F | rng.randrange(4) << 6)]
    for field in ("disp", "imm"):
        off, n = lay[field]
        if n:
            changes.append(lambda off=off, n=n, field=field: b.__setitem__(
                slice(off, off + n), _value_like(bytes(b[off:off + n]), rng, field)))
            value_changes.append(changes[-1])
    m, (d_off, d_n) = lay["modrm"], lay["disp"]
    if m is not None and b[m] >> 6 in (1, 2) and d_n in (1, 4):
        def widen_or_narrow() -> None:                   # disp8 to disp32 and back (the key does not hold the width)
            v = int.from_bytes(b[d_off:d_off + d_n], "little", signed=True) + rng.choice((0, 0, 64, -64, 128, 192, 256))
            if d_n == 1:
                b[m] = b[m] & 0x3F | 0x80
                b[d_off:d_off + 1] = (v & 0xFFFFFFFF).to_bytes(4, "little")
            elif -128 <= v < 128:
                b[m] = b[m] & 0x3F | 0x40
                b[d_off:d_off + 4] = (v & 0xFF).to_bytes(1, "little")
        changes.append(widen_or_narrow)
        value_changes.append(widen_or_narrow)
    if lay["enc"] == "legacy" and lay["rex"] is None:   # a REX byte added: registers 8-15 (the length grows by 1)
        changes.append(lambda: b.insert(lay["prefix_end"], 0x40 | rng.randrange(16)))
        insert_rex.append(changes[-1])
    regs = register_fields(b, lay)
    if regs:
        def rename() -> None:                            # every place naming the chosen place's register, at once
            old = rng.choice(regs)[0]()
            new = rng.randrange(16)
            for read, write in [(r, w) for r, w in regs if r() == old]:
                write(new)
        changes.append(rename)
        rename_change.append(rename)
    pay = lay["payload"]
    if pay and lay["enc"] == "vex" and len(pay) == 1:            # C5: R, vvvv
        changes += [lambda: b.__setitem__(pay[0], b[pay[0]] ^ 0x80),
                    lambda: b.__setitem__(pay[0], b[pay[0]] & 0x87 | rng.randrange(16) << 3)]
    elif pay and lay["enc"] == "evex":                           # R X B R', vvvv
        changes += [lambda: b.__setitem__(pay[0], b[pay[0]] & 0x0F | rng.randrange(16) << 4),
                    lambda: b.__setitem__(pay[1], b[pay[1]] & 0x87 | rng.randrange(16) << 3)]
    elif pay:                                                    # C4 and XOP: R X B, vvvv
        changes += [lambda: b.__setitem__(pay[0], b[pay[0]] & 0x1F | rng.randrange(8) << 5),
                    lambda: b.__setitem__(pay[1], b[pay[1]] & 0x87 | rng.randrange(16) << 3)]
    if values_only:                                      # the displacement's and immediate's changes and the width
        changes = value_changes                          # switch only (a value that never varied)
    k = turn % (len(changes) + 1)
    if k == len(changes):
        return None, k, k + 1
    if not values_only and insert_rex and rename_change and rng.random() < 0.5:
        rename_change[0]()                               # a rename, then a REX byte whose R, X and B agree (the rename
        b.insert(lay["prefix_end"], 0x40 | rng.randrange(2) << 3 | rng.choice((0, 7)))   # first: the byte moves every
                                                         # later field): equal registers 8-15
        return bytes(b), k, len(changes) + 1
    changes[k]()
    return bytes(b), k, len(changes) + 1


def _address_like(address: int, rng: random.Random) -> int:
    """A new address, moved by a multiple of 16 (its and its targets' divisibility by the access sizes kept)."""
    pick = rng.randrange(4)
    return max(0x1000, address + 16 * (rng.randint(-256, 256) if pick < 2 else rng.randint(-1 << 16, 1 << 16)
                                       if pick == 2 else rng.randint(-1 << 28, 1 << 28)))


def instances(call: Any, code: bytes, key: Any, implicit: frozenset[int], rng: random.Random,
              tries: int = TRIES, keep: int = KEEP, values_only: bool = False) -> list[Instance]:
    """Instances of the key: the recorded instruction first, then its kept mutations, distinct, at most `keep`."""
    ins = call[0]
    lay = layout(code, x86dec.decode(code))
    out: list[Instance] = [(call, code, sc.decoded_form(call, code, implicit)[1])]
    seen = {(code, ins.address)}
    kept: Counter[int] = Counter()                       # kept instances per kind of change: a quota each, so every
    for turn in range(tries):                            # field varies in the kept set where its key lets it
        if len(out) >= keep:
            break
        changed, k, kinds = mutate(code, lay, rng, turn, values_only)
        if kept[k] >= max(2, (keep - 1) // kinds) and turn < tries // 2:
            continue
        b = code if changed is None else changed
        # the address's own turn; an added byte moves the start back by one, so the next address (and any target
        # relative to it) stays where it was
        address = (_address_like(ins.address, rng) if changed is None else ins.address - (len(b) - len(code)))
        if (b, address) in seen:
            continue
        seen.add((b, address))
        try:
            if x86dec.decode(b).length != len(b):
                continue
            new = (lift(b, address), *call[1:])
            k2, values, _ = sc.decoded_form(new, b, implicit)
        except Exception:                                # not an instruction, or not one the lifter takes
            continue
        if k2 == key:
            kept[k] += 1
            out.append((new, b, values))
    return out


def _signature(items: list[Any]) -> tuple[Any, ...]:
    """The encoder sequence, with its string arguments (a condition, an operation's name): a string that changes
    between instances is a choice of structure, not a hole."""
    return tuple((it[0], it[1], len(it[2]), tuple(a for a in it[2] if isinstance(a, str))) for it in items)


def _arg_spec(values: list[Any], valueses: list[list[int]], symbol_of: dict[int, str]) -> Any:
    """One argument's spec over the instances: ("c", value), ("sym", name), ("h", expression, position),
    ("kw", name, spec) for a keyword argument, or ("other",)."""
    first = values[0]
    if isinstance(first, tuple) and len(first) == 2 and isinstance(first[0], str):
        if all(isinstance(v, tuple) and v[0] == first[0] for v in values):
            return ("kw", first[0], _arg_spec([v[1] for v in values], valueses, symbol_of))
        return ("other",)
    if all(v == first for v in values):
        if isinstance(first, int) and not isinstance(first, bool) and first in symbol_of and first > 0xFFFF:
            return ("sym", symbol_of[first])
        return ("c", first)
    found = sc.hole_expression(list(zip(values, valueses)))
    return ("h", found[1], found[2]) if found else ("other",)


def _guards(stencil: Any, samples: list[tuple[list[Any], list[int]]]) -> tuple[tuple[int, int], ...]:
    """The values the samples never varied that a constant argument may stand for (Sprint 079): each such value's
    position and its one value. A constant equal to an expression of a value that never varied may be a hole the
    instances could not show (a refined group's register, a target, a return address); an instruction whose value
    there differs has no stencil from this table."""
    first = samples[0][1]
    fixed = [p for p in range(len(first)) if all(v[p] == first[p] for _, v in samples)]
    constants = {s[1] for _, _, specs in stencil for s in specs
                 if s[0] == "c" and isinstance(s[1], int) and not isinstance(s[1], bool)}
    constants |= {s[2][1] for _, _, specs in stencil for s in specs
                  if s[0] == "kw" and s[2][0] == "c" and isinstance(s[2][1], int) and not isinstance(s[2][1], bool)}
    out = []
    for p in fixed:
        try:
            images = {fn(first[p]) for _, _, fn in sc.EXPRESSIONS}
        except (ValueError, OverflowError):
            images = set()
        if images & constants:
            out.append((p, first[p]))
    return tuple(out)


def derive(samples: list[tuple[list[Any], list[int]]], symbol_of: dict[int, str]) -> Any:
    """The stencil of a key from its instances' items and values, with its guards (_guards), or ("split",
    signatures)."""
    sigs = {_signature(items) for items, _ in samples}
    if len(sigs) > 1:
        return ("split", sigs)
    first = samples[0][0]
    stencil = []
    for pos, (kind, name, args) in enumerate(first):
        specs = [_arg_spec([items[pos][2][ai] for items, _ in samples], [v for _, v in samples], symbol_of)
                 for ai in range(len(args))]
        stencil.append((kind, name, tuple(specs)))
    return ("stencil", tuple(stencil), _guards(stencil, samples))


# A key whose instances give more than one encoder sequence is refined by one feature of one decoded value: an SSE
# shift's count at or past the lane width (a zeroing MOVI, not a shift), CMPPS's predicate in the immediate's low bits.
FEATURES: list[tuple[str, Any]] = (
    [(f"at least {w}", lambda v, w=w: (v & 0xFF) >= w) for w in (8, 16, 32, 64, 128)]
    + [(f"low {k} bits", lambda v, k=k: v & (1 << k) - 1) for k in (1, 2, 3, 4, 5)])
FEATURE = dict(FEATURES)


def refine(samples: list[tuple[list[Any], list[int]]], symbol_of: dict[int, str]) -> Any:
    """("by", value position, feature, {feature value: stencil}) for the first feature and position under which every
    group of instances gives one sequence, or None."""
    for fname, fn in FEATURES:
        for p in range(len(samples[0][1])):
            groups: dict[Any, list[tuple[list[Any], list[int]]]] = defaultdict(list)
            for items, values in samples:
                groups[fn(values[p])].append((items, values))
            if len(groups) < 2 or any(len({_signature(i) for i, _ in g}) > 1 for g in groups.values()):
                continue
            return ("by", p, fname, {fv: derive(g, symbol_of)[1:] for fv, g in groups.items()})
    return None


def stencil_for(entry: Any, values: list[int]) -> Any:
    """The stencil a table entry gives for an instruction's decoded values, or None (a feature value with none, or a
    guard the values fail)."""
    if entry[0] == "one":
        chosen = entry[1:]
    else:
        _, p, fname, by = entry
        chosen = by.get(FEATURE[fname](values[p]))
    if chosen is None or any(values[q] != v for q, v in chosen[1]):
        return None
    return chosen[0]


def _value(spec: Any, values: list[int], symbols: dict[str, int]) -> Any:
    tag = spec[0]
    if tag == "c":
        return spec[1]
    if tag == "sym":
        return symbols[spec[1]]
    if tag == "h":
        return EXPRESSION[spec[1]](values[spec[2]])
    if tag == "kw":
        return (spec[1], _value(spec[2], values, symbols))
    raise ValueError("an argument named other has no value")


def rebuild(stencil: Any, values: list[int], symbols: dict[str, int], words_of: dict[str, Any]) -> list[int]:
    """The words a stencil gives for an instruction's decoded values."""
    out: list[int] = []
    for kind, name, specs in stencil:
        args = [_value(s, values, symbols) for s in specs]
        pos = [a for a in args if not (isinstance(a, tuple) and len(a) == 2 and isinstance(a[0], str))]
        kw = dict(a for a in args if isinstance(a, tuple) and len(a) == 2 and isinstance(a[0], str))
        w = words_of[name](*pos, **kw)
        ws = w if isinstance(w, list) else [w]
        out.extend(ws if kind != "enc32" else [ws[0] & ~(1 << 31)])
    return out


_STATE: dict[str, Any] = {}


def _init(symbols: dict[str, int]) -> None:
    _STATE["log"] = sc.EncoderLog()
    _STATE["symbols"] = symbols
    _STATE["symbol_of"] = {v: k for k, v in symbols.items()}
    _STATE["words_of"] = {name: getattr(a64_encoder, name).__wrapped__ for name in dir(a64_encoder)
                          if hasattr(getattr(a64_encoder, name), "__wrapped__")}


def _compiled(inst: Instance) -> tuple[list[int], list[Any]] | None:
    try:
        return sc.compile_logged(_STATE["log"], inst[0], _STATE["symbols"])
    except Exception:
        return None


def gather(recorded: list[tuple[Any, bytes]], key: Any, implicit: frozenset[int], seed: int,
           counts: Counter[str]) -> list[tuple[list[Any], list[int]]]:
    """A key's samples (each instance's encoder items and decoded values): the recorded instruction's mutations, the
    key's recorded calls at even positions, and, where a value never varied or the key has an immediate, a longer
    round of value changes."""
    call, code = recorded[0]
    made = instances(call, code, key, implicit, random.Random(seed))
    counts["kept"] += len(made)
    made += [(c, b, sc.decoded_form(c, b, implicit)[1]) for c, b in recorded[2:2 * KEEP:2]]
    samples = []
    for inst in made:
        c = _compiled(inst)
        if c is None:
            counts["compile failed"] += 1
            continue
        samples.append((c[1], inst[2]))
    if not samples:
        return samples
    # a value that never varied (not a register, not the next address), or an immediate: a longer round of value
    # changes only
    nregs = len(key[6])
    unvaried = [p for p in range(nregs + 1, len(samples[0][1])) if len({v[p] for _, v in samples}) == 1]
    immediate = any(o[0] == "I" for o in key[5] if isinstance(o, tuple))
    if unvaried or immediate:                            # an immediate's bits (a shuffle's selectors) need many
        counts["keys with an unvaried value"] += bool(unvaried)
        for inst in instances(call, code, key, implicit, random.Random(seed + 3_000_003), 16 * TRIES, 2 * KEEP,
                              values_only=True)[1:]:
            c = _compiled(inst)
            if c is not None:
                samples.append((c[1], inst[2]))
        if any(len({v[p] for _, v in samples}) == 1 for p in unvaried):
            counts["keys with a value still unvaried"] += 1
    return samples


def build_one(job: tuple[Any, list[tuple[Any, bytes]], frozenset[int], int]) -> tuple[Any, Any, dict[str, int]]:
    """One key's stencil and its judge: (key, stencil or split, counts). The key's recorded calls at even positions
    join the made instances in the derivation; those at odd positions are judged, with fresh instances."""
    key, recorded, implicit, seed = job
    counts: Counter[str] = Counter()
    call, code = recorded[0]
    samples = gather(recorded, key, implicit, seed, counts)
    if not samples:
        return key, None, dict(counts)
    derived = derive(samples, _STATE["symbol_of"])
    entry: Any = ("one", derived[1], derived[2]) if derived[0] == "stencil" else None
    if entry is None:                                    # more instances, then a feature that separates them
        for inst in instances(call, code, key, implicit, random.Random(seed + 2_000_003), 8 * TRIES, 8 * KEEP)[1:]:
            c = _compiled(inst)
            if c is not None:
                samples.append((c[1], inst[2]))
        entry = refine(samples, _STATE["symbol_of"])
        counts["refined" if entry else "split"] += 1
    if entry is not None:
        stencils = [entry[1]] if entry[0] == "one" else [st for st, _ in entry[3].values()]
        counts["other arguments"] += sum(1 for st in stencils for _, _, specs in st for s in specs if s[0] == "other")
        judged = instances(call, code, key, implicit, random.Random(seed + 1_000_003))[1:]
        judged += [(c, b, sc.decoded_form(c, b, implicit)[1]) for c, b in recorded[1:2 * KEEP:2]]
        for inst in judged:
            c = _compiled(inst)
            if c is None:
                continue
            counts["judged"] += 1
            chosen = stencil_for(entry, inst[2])
            if chosen is None:
                counts["judged with no stencil (a feature value or a guard)"] += 1
                continue
            try:
                ok = rebuild(chosen, inst[2], _STATE["symbols"], _STATE["words_of"]) == c[0]
            except Exception:
                ok = False
            counts["judged equal" if ok else "judged differ"] += 1
    return key, entry if entry is not None else ("split", derived[1]), dict(counts)


def keys_of(record: dict[str, Any]) -> tuple[dict[Any, list[Any]], dict[str, frozenset[int]]]:
    """Each key's recorded calls, in record order."""
    implicit = sc.implicit_registers(record)
    by_key: dict[Any, list[Any]] = defaultdict(list)
    for call in record["calls"]:
        code = next(iter(record["bytes"][call[0]]))
        row = x86dec.decode(code).row or ""
        by_key[sc.decoded_form(call, code, implicit.get(row, frozenset()))[0]].append(call)
    return by_key, implicit


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("record", type=Path)
    ap.add_argument("--out", type=Path)
    ap.add_argument("--workers", type=int, default=max(1, (multiprocessing.cpu_count() or 2) - 1))
    a = ap.parse_args(argv)
    with open(a.record, "rb") as f:
        record = pickle.load(f)
    by_key, implicit = keys_of(record)
    jobs = []
    for n, (key, calls) in enumerate(by_key.items()):
        recorded = [(c, next(iter(record["bytes"][c[0]]))) for c in calls[:2 * KEEP]]
        jobs.append((key, recorded, implicit.get(key[0] or "", frozenset()), 76_000 + n))
    table: dict[Any, Any] = {}
    totals: Counter[str] = Counter()
    kept = Counter[int]()
    splits = []
    with multiprocessing.get_context("fork").Pool(a.workers, _init, (record["symbols"],)) as pool:
        for key, stencil, counts in pool.imap_unordered(build_one, jobs, chunksize=64):
            totals.update(counts)
            kept[min(counts.get("kept", 0), KEEP)] += 1
            if stencil is None:
                totals["keys with no compiled instance"] += 1
            elif stencil[0] == "split":
                splits.append(key)
            else:
                table[key] = stencil
    print(f"keys {len(by_key)}; stencils {len(table)}; keys split by their instances {len(splits)}; "
          f"instances kept per key (with the recorded one) {dict(sorted(kept.items()))}; totals {dict(totals)}")
    every = [e[1] for e in table.values() if e[0] == "one"] + [st for e in table.values() if e[0] == "by"
                                                              for st, _ in e[3].values()]
    guards = [e[2] for e in table.values() if e[0] == "one"] + [gs for e in table.values() if e[0] == "by"
                                                               for _, gs in e[3].values()]
    print(f"stencils with guards {sum(1 for gs in guards if gs)} of {len(guards)}; guards {sum(map(len, guards))}")
    args = Counter(s[0] if s[0] != "h" else "h " + s[1].split()[0] for st in every for _, _, specs in st
                   for s in specs)
    print(f"keys refined by a feature: {sum(1 for e in table.values() if e[0] == 'by')} "
          f"{dict(Counter(e[2] for e in table.values() if e[0] == 'by'))}")
    print(f"arguments by kind: {dict(args)}")
    _init(record["symbols"])                             # the record's own instructions, rebuilt from the table
    equal = differ = missing = guarded = 0
    for key, calls in by_key.items():
        if key not in table:
            missing += len(calls)
            continue
        for call in calls:
            code = next(iter(record["bytes"][call[0]]))
            values = sc.decoded_form(call, code, implicit.get(key[0] or "", frozenset()))[1]
            c = _compiled((call, code, values))
            chosen = stencil_for(table[key], values)
            if chosen is None:
                guarded += 1                             # a feature value with no stencil, or a guard failed
                continue
            try:
                ok = c is not None and chosen is not None and rebuild(chosen, values, record["symbols"],
                                                                       _STATE["words_of"]) == c[0]
            except Exception:
                ok = False
            equal += ok
            differ += not ok
    print(f"record calls rebuilt from the table: {equal} equal, {differ} differ, {missing} with no stencil, "
          f"{guarded} with no stencil for their values (a guard or a feature value)")
    if a.out:
        with open(a.out, "wb") as f:
            pickle.dump({"table": table, "implicit": implicit, "splits": splits}, f)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
