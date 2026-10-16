# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""class_census.py — the instruction classes a game's native code reaches, and which w2m lifts (Sprint 044).

The method of the 2026-10-09 progress review's count (323 of UnityPlayer's 345 classes without a
rule): Capstone's recursive traversal of each PE file (x86ref.walk) gives the reachable instructions;
w2m's own decoder names each one's iclass; w2m's lifter then lifts it or refuses it. A class is
lifted when every reachable instance lifts, partly lifted when some do (a form the rule refuses, as
RET imm16 was), and missing when none do. The census is the count, by file, with instances.

Sprint 049: `--jit capture.bin` counts the code mono's JIT generated in a captured run (tools/jit_capture: every
compiled method and code buffer), decoded linearly from each region's start; a region stops at the first byte w2m's
decoder refuses (data after the code), and a specific trampoline at its first call (src/w2m/jit_capture.py).

Sprint 086 (docs/recentering-2026-10-16-knowable-first.md): a lifted instruction whose operations include one the VM
hands to the Python model (MODEL_KINDS: compile_insn's hvc #11) is counted as `model` too: it runs, at the host's speed.
Sprint 086: the reachable set is Capstone's walk joined with a linear decode of every function `.pdata` lists, start
to end (`sweep`): the walk misses code reached only through jump tables and pointers (32% of the mono code the batch
run compiled, 2.7% of UnityPlayer's), the sweep misses leaf functions with no `.pdata` entry; together they hold 99.7%
and 97.9% (judge_record).
`--depots` takes every x86-64 image of every depot in store/census/static (no .NET assembly) and writes
store/census/static/instructions-<depot>.json, one process per image.

Usage: uv run python scripts/class_census.py <file.exe|file.dll> ... [--json out.json]
       uv run python scripts/class_census.py --jit <capture.bin> [--json out.json]
       uv run python scripts/class_census.py --depots [~/w2m-depots]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from w2m import x86ref  # noqa: E402
from w2m.lift import LiftError, lift  # noqa: E402
from w2m.pe import PeError, parse  # noqa: E402
from w2m.x86dec import DecodeError, decode  # noqa: E402


MODEL_KINDS = frozenset({"x86", "x87", "ud", "fcw", "xgetbv"})   # vm_backend.compile_insn: hvc #11, the model


def census(path: Path) -> dict[str, Any]:
    data = path.read_bytes()
    img = parse(data)
    raws, only, tables = reachable(img, data)
    instances: Counter[str] = Counter()
    only_by_class: Counter[str] = Counter()
    lifted: Counter[str] = Counter()
    model: Counter[str] = Counter()
    refusals: dict[str, Counter[str]] = defaultdict(Counter)
    undecoded = 0
    for rva, raw in sorted(raws.items()):
        try:
            d = decode(raw + bytes(15))
        except DecodeError:
            undecoded += 1
            continue
        instances[d.iclass] += 1
        only_by_class[d.iclass] += rva in only
        try:
            lifted_ins = lift(raw + b"\x90" * 15, img.image_base + rva)
            lifted[d.iclass] += 1
            if any(op[0] in MODEL_KINDS for op in lifted_ins.ops):
                model[d.iclass] += 1
        except LiftError as e:
            refusals[d.iclass][str(e).split(" at ")[0]] += 1
    classes: dict[str, dict[str, Any]] = {ic: {"instances": n, "lifted": lifted[ic], "model": model[ic],
                    "sweep_only": only_by_class[ic], "refusals": dict(refusals[ic].most_common(3))}
                                          for ic, n in instances.most_common()}
    return {"file": path.name, "sha256": hashlib.sha256(data).hexdigest(),
            "reachable_instructions": len(raws), "walked": len(raws) - len(only), "undecoded": undecoded,
            "jump_tables": len(tables), "jump_table_entries": sum(len(e) for e in tables.values()),
            "precision": precision(img, data, raws, only),
            "classes": len(classes),
            "lifted": sum(1 for c in classes.values() if c["lifted"] == c["instances"]),
            "partly_lifted": sum(1 for c in classes.values() if 0 < c["lifted"] < c["instances"]),
            "missing": sum(1 for c in classes.values() if c["lifted"] == 0),
            "by_class": classes}


STOPS = frozenset({"RET_NEAR", "JMP", "INT3", "UD2", "HLT"})   # no fall-through: what follows is reached otherwise
# MSVC's x64 switch (Sprint 087; large review of 082-086, H2): `mov r32, dword ptr [base + index*4 + table]` with base
# the image base (an earlier `lea base, [rip - ...]`), so `table` is an RVA and each entry an RVA in the function
TABLE_LOAD = re.compile(r"dword ptr \[(\w+) \+ (\w+)\*4 \+ (0x[0-9a-f]+)\]")


def _functions(img: Any) -> list[tuple[int, int]]:
    return sorted(zip(img.function_starts, img.function_ends, strict=True))


def _function_at(functions: list[tuple[int, int]], rva: int) -> tuple[int, int] | None:
    import bisect
    i = bisect.bisect_right(functions, (rva, 1 << 32)) - 1
    return functions[i] if i >= 0 and functions[i][0] <= rva < functions[i][1] else None


def jump_tables(img: Any, data: bytes, insns: dict[int, Any]) -> dict[int, list[int]]:
    """Each switch table a dispatch load names: its RVA and its entries, read while each is an RVA inside the load's
    own `.pdata` function (the table ends where a word is not)."""
    functions = _functions(img)
    tables: dict[int, list[int]] = {}
    for ins in insns.values():
        m = TABLE_LOAD.search(ins.op_str) if ins.mnemonic in ("mov", "movsxd") else None
        fn = _function_at(functions, ins.rva) if m else None
        if m is None or fn is None:
            continue
        table, entries = int(m.group(3), 16), list[int]()
        while len(entries) < 4096:
            try:
                word = int.from_bytes(img.read_rva(data, table + 4 * len(entries), 4), "little")
            except PeError:
                break
            if not fn[0] <= word < fn[1]:
                break
            entries.append(word)
        if entries:
            tables[table] = entries
    return tables


def reachable(img: Any, data: bytes) -> tuple[dict[int, bytes], set[int], dict[int, list[int]]]:
    """The walk, with the targets of the jump tables it finds walked too (until no new table appears), joined with the
    sweep: (raw bytes by RVA, the RVAs only the sweep found, the tables)."""
    reach = x86ref.walk(img, data)
    tables = jump_tables(img, data, reach.insns)
    for _ in range(4):
        targets = {t for es in tables.values() for t in es} - set(reach.insns)
        if not targets:
            break
        reach = x86ref.walk(img, data, extra_roots=tuple(sorted({t for es in tables.values() for t in es})))
        tables = jump_tables(img, data, reach.insns)
    raws = {rva: ins.raw for rva, ins in reach.insns.items()}
    swept = sweep(img, data, set(raws) | {t for es in tables.values() for t in es},
                  [(t, t + 4 * len(es)) for t, es in tables.items()])
    only = {rva for rva in swept if rva not in raws}
    raws.update({rva: swept[rva] for rva in only})
    return raws, only, tables


def sweep(img: Any, data: bytes, known: set[int] | None = None,
          tables: list[tuple[int, int]] | None = None) -> dict[int, bytes]:
    """A linear decode of each `.pdata` function, by RVA. After an instruction with no fall-through (STOPS) it
    resumes only at the next address the walk or a jump table reaches inside the function (`known`), and it never
    decodes a jump table's words: MSVC places tables inside functions (UnityPlayer.dll 0x130260: RET, NOP, then RVAs).
    A function's decode stops at a byte w2m's decoder refuses."""
    import bisect
    known_sorted = sorted(known or ())
    spans = sorted(tables or ())
    out: dict[int, bytes] = {}

    def next_known(after: int, end: int) -> int | None:
        i = bisect.bisect_right(known_sorted, after)
        return known_sorted[i] if i < len(known_sorted) and known_sorted[i] < end else None

    def in_table(rva: int) -> int | None:
        i = bisect.bisect_right(spans, (rva, 1 << 32)) - 1
        return spans[i][1] if i >= 0 and spans[i][0] <= rva < spans[i][1] else None

    for start, end in zip(img.function_starts, img.function_ends, strict=True):
        code = img.read_rva(data, start, end - start)
        at: int | None = start
        while at is not None and at < end:
            table_end = in_table(at)
            if table_end is not None:
                at = next_known(table_end - 1, end) if known is not None else table_end
                continue
            try:
                d = decode(code[at - start:at - start + 15] + bytes(15))
            except DecodeError:
                break
            out[at] = code[at - start:at - start + d.length]
            at += d.length
            if known is not None and d.iclass in STOPS:
                at = next_known(at - 1, end)
    return out


def precision(img: Any, data: bytes, raws: dict[int, bytes], only: set[int]) -> dict[str, int]:
    """The large review's measure (H2): sweep-only instructions that hold a 4-byte word that is an RVA inside their own
    function (a jump-table entry read as code), and sweep-only instructions that overlap a walked one's bytes."""
    functions = _functions(img)
    walked = sorted(r for r in raws if r not in only)
    import bisect
    table_like = overlap = 0
    for rva in only:
        raw = raws[rva]
        fn = _function_at(functions, rva)
        if fn and any(fn[0] <= int.from_bytes(raw[k:k + 4], "little") < fn[1] for k in range(0, len(raw) - 3)):
            table_like += 1
        i = bisect.bisect_right(walked, rva) - 1
        if i >= 0 and walked[i] + len(raws[walked[i]]) > rva:
            overlap += 1
    return {"sweep-only": len(only), "holding an RVA into their function": table_like,
            "overlapping a walked instruction": overlap}


def judge_record(depot: Path, census_json: Path, record: Path) -> dict[str, tuple[int, int, int]]:
    """Sprint 086's judge, from outside the census's sources: the instructions the batch run compiled
    (store/stencil/record.txt.gz) against each image's reachable set. An image's base in that run is found from the
    record itself: the 64 KB-aligned difference most `.pdata` starts' bytes agree on. (image: recorded, reachable,
    bytes equal) for each image the run loaded."""
    import gzip
    recs: dict[int, bytes] = {}
    with gzip.open(record, "rt") as f:
        f.readline()
        for line in f:
            address, code = line.split()[:2]
            recs[int(address, 16)] = bytes.fromhex(code)
    by_bytes: dict[bytes, list[int]] = defaultdict(list)
    for at, b in recs.items():
        by_bytes[b].append(at)
    out = {}
    c = json.loads(census_json.read_text())
    for rel, e in sorted(c["images"].items()):
        if e["machine"] != 0x8664 or e.get("dotnet") or "refused" in e:
            continue
        data = (depot / rel).read_bytes()
        img = parse(data)
        votes: Counter[int] = Counter()
        for rva in img.function_starts[:5000]:
            for n in (8, 7, 6, 5):
                b = img.read_rva(data, rva, n)
                votes.update(a - rva for a in by_bytes.get(b, ()) if (a - rva) & 0xFFFF == 0)
        if not votes:
            continue
        base = votes.most_common(1)[0][0]
        mine = [at for at in recs if base <= at < base + img.size_of_image]
        equal = 0
        for at in mine:
            try:
                equal += img.read_rva(data, at - base, len(recs[at])) == recs[at]
            except PeError:
                pass
        if equal < len(mine) // 2:                       # not this image's code: the run did not load it here
            continue
        raws = set(reachable(img, data)[0])
        out[rel] = (len(mine), sum(1 for at in mine if at - base in raws), equal)
    return out


def jit_census(path: Path) -> dict[str, Any]:
    from w2m import jit_capture
    cap = jit_capture.parse(path.read_bytes())
    instances: Counter[str] = Counter()
    lifted: Counter[str] = Counter()
    refusals: dict[str, Counter[str]] = defaultdict(Counter)
    stopped = total = 0
    regions = cap.regions()
    for r in regions:
        at = 0
        while at < len(r.code):
            window = r.code[at:at + 15]
            try:
                d = decode(window + bytes(15))
            except DecodeError:
                stopped += 1
                break
            if at + d.length > len(r.code):
                stopped += 1
                break
            total += 1
            instances[d.iclass] += 1
            try:
                lift(r.code[at:at + d.length] + b"\x90" * 15, r.start + at)
                lifted[d.iclass] += 1
            except LiftError as e:
                refusals[d.iclass][str(e).split(" at ")[0]] += 1
            at += d.length
            if r.code_ends_at_first_call and d.iclass.startswith("CALL"):
                break
    classes: dict[str, dict[str, Any]] = {ic: {"instances": n, "lifted": lifted[ic],
                    "refusals": dict(refusals[ic].most_common(3))} for ic, n in instances.most_common()}
    return {"file": f"{path.name} (JIT code: {len(regions)} regions, {stopped} stopped at an undecoded byte)",
            "reachable_instructions": total, "undecoded": stopped, "classes": len(classes),
            "lifted": sum(1 for c in classes.values() if c["lifted"] == c["instances"]),
            "partly_lifted": sum(1 for c in classes.values() if 0 < c["lifted"] < c["instances"]),
            "missing": sum(1 for c in classes.values() if c["lifted"] == 0),
            "by_class": classes}


TABLE32 = re.compile(r"dword ptr \[(\w+)\*4 \+ (0x[0-9a-f]+)\]")


def census32(path: Path) -> dict[str, Any]:
    """A 32-bit image (PE32, x86) sized by Capstone (w2m's decoder takes 64-bit mode only; its 32-bit mode is the
    first item of the 32-bit work list): the walk from the entry, exports and TLS callbacks, with MSVC's 32-bit switch
    tables (`jmp dword ptr [index*4 + table]`, entries VAs) walked too; instances by Capstone mnemonic. A PE32 image
    has no `.pdata`, so there is no sweep."""
    data = path.read_bytes()
    img = parse(data, x86=True)
    reach = x86ref.walk(img, data, x86=True)
    for _ in range(4):
        targets: set[int] = set()
        for ins in reach.insns.values():
            m = TABLE32.search(ins.op_str) if ins.mnemonic == "jmp" else None
            if not m:
                continue
            at = int(m.group(2), 16) - img.image_base
            for k in range(4096):
                try:
                    va = int.from_bytes(img.read_rva(data, at + 4 * k, 4), "little")
                except PeError:
                    break
                rva = va - img.image_base
                if not any(sec.virtual_address <= rva < sec.virtual_address + sec.virtual_size
                           and sec.characteristics & 0x20000000 for sec in img.sections):
                    break
                targets.add(rva)
        if not targets - set(reach.insns):
            break
        reach = x86ref.walk(img, data, extra_roots=tuple(sorted(targets)), x86=True)
    mnemonics = Counter(i.mnemonic for i in reach.insns.values())
    return {"file": path.name, "sha256": hashlib.sha256(data).hexdigest(), "machine": "x86",
            "reachable_instructions": len(reach.insns), "unresolved_indirect": len(reach.unresolved_indirect),
            "by_mnemonic": dict(mnemonics.most_common())}


def depots(root: Path) -> int:
    """Every x86-64 image the static census found (store/census/static/<depot>.json), censused one process per image;
    each depot's result to store/census/static/instructions-<depot>.json."""
    from concurrent.futures import ProcessPoolExecutor
    static = ROOT / "store" / "census" / "static"
    seen: dict[str, str] = {}                            # sha256 -> the depot/path it was censused under
    # a depot that copies another's images (hail-to-the-rainbow-native) comes after it, so the images count under the
    # game's own depot; it keeps a file naming where each of its images was counted
    files = sorted((f for f in static.glob("*.json") if not f.name.startswith("instructions-")),
                   key=lambda f: (f.stem.endswith("-native"), f.stem))
    for f in files:
        c = json.loads(f.read_text())
        rels, rels32, same = list[str](), list[str](), dict[str, str]()
        for r, e in sorted(c["images"].items()):
            if e.get("dotnet") or "refused" in e:
                continue
            if e["sha256"] in seen:                      # one image under two depots is counted once (large review M2)
                same[r] = seen[e["sha256"]]
                continue
            seen[e["sha256"]] = f"{c['depot']}/{r}"
            (rels if e["machine"] == 0x8664 else rels32).append(r)
        if not rels and not rels32 and not same:
            continue
        with ProcessPoolExecutor() as pool:
            results = list(pool.map(census, [root / c["depot"] / r for r in rels]))
            results32 = list(pool.map(census32, [root / c["depot"] / r for r in rels32]))
        out = {"depot": c["depot"], "images": dict(zip(rels, results, strict=True)),
               "images_x86": dict(zip(rels32, results32, strict=True)), "same_as": same}
        (static / f"instructions-{c['depot']}.json").write_text(json.dumps(out, indent=1, sort_keys=True) + "\n")
        print(depot_summary(out))
    return 0


def depot_summary(out: dict[str, Any]) -> str:
    """Instances and classes native, run by the model, not lifted, over the depot's images; the two work lists."""
    if not out["images"] and not out["images_x86"]:
        return f"{out['depot']}: every image counted under another depot {sorted(set(out['same_as'].values()))[:3]}"
    if not out["images"]:
        n = sum(r["reachable_instructions"] for r in out["images_x86"].values())
        top = Counter[str]()
        for r in out["images_x86"].values():
            top.update(r["by_mnemonic"])
        return (f"{out['depot']}: {len(out['images_x86'])} x86 images, {n} reachable instructions, "
                f"{len(top)} mnemonics; most {top.most_common(12)}")
    by: dict[str, Counter[str]] = defaultdict(Counter)
    for r in out["images"].values():
        for ic, v in r["by_class"].items():
            by[ic].update({"instances": v["instances"], "lifted": v["lifted"], "model": v.get("model", 0)})
    total = sum(v["instances"] for v in by.values())
    missing = sorted(((ic, v["instances"] - v["lifted"]) for ic, v in by.items() if v["lifted"] < v["instances"]),
                     key=lambda e: -e[1])
    model = sorted(((ic, v["model"]) for ic, v in by.items() if v["model"]), key=lambda e: -e[1])
    return (f"{out['depot']}: {len(out['images'])} images, {total} reachable instructions in {len(by)} classes; "
            f"not lifted {sum(n for _, n in missing)} in {len(missing)} classes {missing[:20]}; "
            f"run by the model {sum(n for _, n in model)} in {len(model)} classes {model[:20]}")


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="*", type=Path)
    ap.add_argument("--jit", type=Path, action="append", default=[])
    ap.add_argument("--json", type=Path)
    ap.add_argument("--depots", type=Path, nargs="?", const=Path.home() / "w2m-depots")
    a = ap.parse_args(argv)
    if a.depots:
        return depots(a.depots)
    out = [census(p) for p in a.files] + [jit_census(p) for p in a.jit]
    for c in out:
        missing = [(ic, v["instances"]) for ic, v in c["by_class"].items() if v["lifted"] == 0]
        partly = [(ic, v["lifted"], v["instances"]) for ic, v in c["by_class"].items() if 0 < v["lifted"] < v["instances"]]
        print(f"{c['file']}: {c['reachable_instructions']} reachable instructions ({c['undecoded']} undecoded), "
              f"{c['classes']} classes: {c['lifted']} lifted, {c['partly_lifted']} partly, {c['missing']} missing")
        print(f"  missing, most instances first: {missing[:25]}")
        print(f"  partly lifted: {partly[:15]}")
    if a.json:
        a.json.parent.mkdir(parents=True, exist_ok=True)
        a.json.write_text(json.dumps(out, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
