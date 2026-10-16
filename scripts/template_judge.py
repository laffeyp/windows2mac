# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""template_judge.py — symbolic templates (src/w2m/symbolic_lift.py) against lift() (Sprint 089).

For every instruction of three sources, the template for its class, shape and mode, filled with its values, against
lift()'s ops: the batch run's compiled instructions (store/stencil/record.txt.gz), random encodings of every decoder
spec row (tests/test_x86_decode_c.row_samples), and the class case sets (semantics_differential.CASE_SETS). Counts:
equal, differing (a defect), lift() refuses (no rule: not the templates' to judge), no template applies, and the
templates made, by class.

Usage: uv run python scripts/template_judge.py [--per-row 10]
"""

from __future__ import annotations

import argparse
import gzip
import random
import sys
from collections import Counter
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path[:0] = [str(ROOT / "src"), str(ROOT / "scripts"), str(ROOT / "tests")]

from w2m.lift import LiftError, lift  # noqa: E402
from w2m.symbolic_lift import lifted_by_template  # noqa: E402


def sources(per_row: int) -> dict[str, list[tuple[bytes, int]]]:
    out: dict[str, list[tuple[bytes, int]]] = {"record": [], "spec": [], "cases": []}
    with gzip.open(ROOT / "store" / "stencil" / "record.txt.gz", "rt") as f:
        f.readline()
        seen = set()
        for line in f:
            a, code = line.split()[:2]
            if (a, code) not in seen:
                seen.add((a, code))
                out["record"].append((bytes.fromhex(code), int(a, 16)))
    import test_x86_decode_c
    rng = random.Random(8901)
    out["spec"] = [(raw, 0x1_4000_0000 + rng.randrange(1 << 24)) for raw in test_x86_decode_c.row_samples(rng, per_row)]
    import class_run
    import semantics_differential
    out["cases"] = [(case.code, 0x1_4000_0000 + 16 * i) for k in semantics_differential.CASE_SETS
                    if k in class_run.REFERENCES for i, case in enumerate(class_run._built(k))]
    return out


def judge(items: list[tuple[bytes, int]], made: dict[Any, Any], examples: list[str]) -> Counter[str]:
    c: Counter[str] = Counter()
    for code, address in items:
        try:
            want = lift(code, address).ops
        except LiftError:
            c["lift refuses"] += 1
            continue
        except Exception:
            c["lift raises"] += 1
            continue
        try:
            got = lifted_by_template(code + bytes(15), address, made)
        except Exception as e:
            c[f"template raises {type(e).__name__}"] += 1
            if len(examples) < 20:
                examples.append(f"{code.hex()} {type(e).__name__}: {e}")
            continue
        if got is None:
            c["no template applies"] += 1
        elif got == want:
            c["equal"] += 1
        else:
            c["differ"] += 1
            if len(examples) < 20:
                examples.append(f"{code.hex()} differs")
    return c


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--per-row", type=int, default=10)
    a = ap.parse_args(argv)
    made: dict[Any, Any] = {}
    examples: list[str] = []
    bad = 0
    for name, items in sources(a.per_row).items():
        c = judge(items, made, examples)
        bad += c["differ"]
        print(f"{name}: {len(items)} instructions, {dict(c)}")
    n = sum(len(v) for v in made.values())
    keys = len(made)
    classes = len({k[0] for k in made})
    print(f"templates: {n} over {keys} (class, shape, mode) keys in {classes} classes; "
          f"keys with more than one template {sum(1 for v in made.values() if len(v) > 1)}")
    for e in examples:
        print("  " + e)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
