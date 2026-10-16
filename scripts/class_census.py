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
    reach = x86ref.walk(img, data)
    raws = {rva: ins.raw for rva, ins in reach.insns.items()}
    raws.update({rva: raw for rva, raw in sweep(img, data).items() if rva not in raws})
    instances: Counter[str] = Counter()
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
        try:
            lifted_ins = lift(raw + b"\x90" * 15, img.image_base + rva)
            lifted[d.iclass] += 1
            if any(op[0] in MODEL_KINDS for op in lifted_ins.ops):
                model[d.iclass] += 1
        except LiftError as e:
            refusals[d.iclass][str(e).split(" at ")[0]] += 1
    classes: dict[str, dict[str, Any]] = {ic: {"instances": n, "lifted": lifted[ic], "model": model[ic],
                    "refusals": dict(refusals[ic].most_common(3))} for ic, n in instances.most_common()}
    return {"file": path.name, "sha256": hashlib.sha256(data).hexdigest(),
            "reachable_instructions": len(raws), "walked": len(reach.insns), "undecoded": undecoded,
            "classes": len(classes),
            "lifted": sum(1 for c in classes.values() if c["lifted"] == c["instances"]),
            "partly_lifted": sum(1 for c in classes.values() if 0 < c["lifted"] < c["instances"]),
            "missing": sum(1 for c in classes.values() if c["lifted"] == 0),
            "by_class": classes}


def sweep(img: Any, data: bytes) -> dict[int, bytes]:
    """Every instruction a linear decode of each `.pdata` function finds, start to end (x64 functions hold no data:
    their jump tables are in .rdata), by RVA; a function's decode stops at a byte w2m's decoder refuses."""
    out: dict[int, bytes] = {}
    for start, end in zip(img.function_starts, img.function_ends, strict=True):
        code = img.read_rva(data, start, end - start)
        at = 0
        while at < len(code):
            try:
                n = decode(code[at:at + 15] + bytes(15)).length
            except DecodeError:
                break
            out[start + at] = code[at:at + n]
            at += n
    return out


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
        raws = set(x86ref.walk(img, data).insns) | set(sweep(img, data))
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


def depots(root: Path) -> int:
    """Every x86-64 image the static census found (store/census/static/<depot>.json), censused one process per image;
    each depot's result to store/census/static/instructions-<depot>.json."""
    from concurrent.futures import ProcessPoolExecutor
    static = ROOT / "store" / "census" / "static"
    for f in sorted(static.glob("*.json")):
        if f.name.startswith("instructions-"):
            continue
        c = json.loads(f.read_text())
        rels = [r for r, e in sorted(c["images"].items())
                if e["machine"] == 0x8664 and not e.get("dotnet") and "refused" not in e]
        if not rels:
            continue
        with ProcessPoolExecutor() as pool:
            results = list(pool.map(census, [root / c["depot"] / r for r in rels]))
        out = {"depot": c["depot"], "images": dict(zip(rels, results, strict=True))}
        (static / f"instructions-{c['depot']}.json").write_text(json.dumps(out, indent=1, sort_keys=True) + "\n")
        print(depot_summary(out))
    return 0


def depot_summary(out: dict[str, Any]) -> str:
    """Instances and classes native, run by the model, not lifted, over the depot's images; the two work lists."""
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
