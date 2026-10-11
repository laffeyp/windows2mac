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

Usage: uv run python scripts/class_census.py <file.exe|file.dll> ... [--json out.json]
       uv run python scripts/class_census.py --jit <capture.bin> [--json out.json]
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from w2m import x86ref  # noqa: E402
from w2m.lift import LiftError, lift  # noqa: E402
from w2m.pe import parse  # noqa: E402
from w2m.x86dec import DecodeError, decode  # noqa: E402


def census(path: Path) -> dict[str, Any]:
    data = path.read_bytes()
    img = parse(data)
    reach = x86ref.walk(img, data)
    instances: Counter[str] = Counter()
    lifted: Counter[str] = Counter()
    refusals: dict[str, Counter[str]] = defaultdict(Counter)
    undecoded = 0
    for ins in reach.insns.values():
        try:
            d = decode(ins.raw + bytes(15))
        except DecodeError:
            undecoded += 1
            continue
        instances[d.iclass] += 1
        try:
            lift(ins.raw + b"\x90" * 15, img.image_base + ins.rva)
            lifted[d.iclass] += 1
        except LiftError as e:
            refusals[d.iclass][str(e).split(" at ")[0]] += 1
    classes: dict[str, dict[str, Any]] = {ic: {"instances": n, "lifted": lifted[ic],
                    "refusals": dict(refusals[ic].most_common(3))} for ic, n in instances.most_common()}
    return {"file": path.name, "reachable_instructions": len(reach.insns), "undecoded": undecoded,
            "classes": len(classes),
            "lifted": sum(1 for c in classes.values() if c["lifted"] == c["instances"]),
            "partly_lifted": sum(1 for c in classes.values() if 0 < c["lifted"] < c["instances"]),
            "missing": sum(1 for c in classes.values() if c["lifted"] == 0),
            "by_class": classes}


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


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="*", type=Path)
    ap.add_argument("--jit", type=Path, action="append", default=[])
    ap.add_argument("--json", type=Path)
    a = ap.parse_args(argv)
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
