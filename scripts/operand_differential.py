"""operand_differential.py — w2m operands vs Intel XED operands over a corpus (Sprint 022).

Positions come from the Capstone recursive traversal, as in decoder_differential.py. At each
position where XED and w2m agree on length and iclass and the w2m row carries an operand template,
the two operand lists are compared after x86ops.normalize.

Usage: uv run python scripts/operand_differential.py <dir> [--json out.json]
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from collections import Counter
from pathlib import Path

from w2m.pe import PeError, parse
from w2m.x86dec import DecodeError, decode
from w2m.x86ops import OperandError, normalize, operands
from w2m.x86ref import walk
from w2m.xedref import decode_many

SUFFIXES = (".exe", ".dll", ".pyd")


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("root", type=Path)
    ap.add_argument("--json", type=Path)
    a = ap.parse_args(argv)
    t0 = time.monotonic()
    out: Counter[str] = Counter()
    by_class: Counter[str] = Counter()
    diffs: Counter[tuple[str, str]] = Counter()
    example: dict[tuple[str, str], str] = {}
    for f in sorted(a.root.iterdir()):
        if f.suffix.lower() not in SUFFIXES:
            continue
        data = f.read_bytes()
        try:
            img = parse(data)
        except PeError:
            continue
        insns = list(walk(img, data).insns.values())
        raws = [img.read_rva(data, i.rva, 15) for i in insns]
        for raw, x in zip(raws, decode_many(raws)):
            out["positions"] += 1
            if not x.ok:
                continue
            try:
                g = decode(raw)
            except DecodeError:
                continue
            if (g.length, g.iclass) != (x.length, x.iclass) or g.fields is None or not g.fields.ops:
                out["no_template"] += 1
                continue
            try:
                ours = tuple(map(normalize, operands(g.fields)))
            except OperandError:
                out["no_template"] += 1
                continue
            theirs = tuple(map(normalize, x.operands))
            out["compared"] += 1
            if ours == theirs:
                out["agree"] += 1
            else:
                k = (x.iform, f"{';'.join(theirs)} | {';'.join(ours)}")
                by_class[x.iform] += 1
                diffs[k] += 1
                example.setdefault(k, raw[:x.length].hex())
    n, c = out["positions"], out["compared"]
    print(f"positions {n}; compared {c} ({c / n:.2%}); agree {out['agree']} "
          f"({out['agree'] / max(c, 1):.2%}); differ {c - out['agree']}; "
          f"{time.monotonic() - t0:.0f} s")
    for iform, m in by_class.most_common(15):
        k = next(k for k in diffs if k[0] == iform)
        print(f"  {m:7} {iform}  xed | ours: {k[1]}  {example[k]}")
    if a.json:
        a.json.write_text(json.dumps({
            "positions": n, "compared": c, "agree": out["agree"],
            "differ": [{"iform": k[0], "xed_vs_ours": k[1], "count": m, "example": example[k]}
                       for k, m in diffs.most_common()]}, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
