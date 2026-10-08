"""generated_differential.py — w2m decoder vs Intel XED on generated encodings (Sprint 020).

Outcome classes per case:
  both_reject      XED and ours both refuse the bytes
  agree            same length and iclass
  ours_accepts     XED refuses (the CPU would raise #UD); ours decodes — a validity gap
  ours_gap         XED decodes; ours has no entry — a coverage gap outside the corpus
  length_differs   both decode, different length — a structural bug
  iclass_differs   both decode, same length, different iclass

Usage: uv run python scripts/generated_differential.py [--per-row 48] [--chip ALL] [--json out]
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from collections import Counter
from pathlib import Path

from w2m.encgen import cases
from w2m.x86dec import DecodeError, decode
from w2m.xedref import decode_many


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--per-row", type=int, default=48)
    ap.add_argument("--chip", default="ALL")
    ap.add_argument("--json", type=Path)
    a = ap.parse_args(argv)
    t0 = time.monotonic()
    cs = list(cases(per_row=a.per_row))
    xs = decode_many([c.raw for c in cs], a.chip)
    outcome: Counter[str] = Counter()
    kinds: Counter[tuple[str, str, str]] = Counter()
    example: dict[tuple[str, str, str], str] = {}
    for c, x in zip(cs, xs):
        try:
            g = decode(c.raw + b"\0" * 15)
            ours: tuple[int, str] | None = (g.length, g.iclass)
        except (DecodeError, IndexError, KeyError):
            ours = None
        if not x.ok and ours is None:
            o, k = "both_reject", ("", "", "")
        elif x.ok and ours is not None and (x.length, x.iclass) == ours:
            o, k = "agree", ("", "", "")
        elif not x.ok:
            assert ours is not None
            o, k = "ours_accepts", (x.error, ours[1], "")
        elif ours is None:
            o, k = "ours_gap", (x.iclass, "", "")
        elif ours[0] != x.length:
            o, k = "length_differs", (x.iclass, ours[1], f"{x.length}/{ours[0]}")
        else:
            o, k = "iclass_differs", (x.iclass, ours[1], "")
        outcome[o] += 1
        if o not in ("both_reject", "agree"):
            kinds[(o, *k[:2], k[2])] += 1  # type: ignore[arg-type]
            example.setdefault((o, *k[:2], k[2]), f"{c.raw[:x.length or 15].hex()} [{c.note}]")  # type: ignore[arg-type]
    total = len(cs)
    print(f"cases {total} ({a.per_row} per row, chip {a.chip}); {time.monotonic() - t0:.0f} s")
    for o in ("agree", "both_reject", "ours_accepts", "ours_gap", "length_differs", "iclass_differs"):
        print(f"  {o:15} {outcome[o]:7} ({outcome[o] / total:.2%})")
    for o in ("length_differs", "iclass_differs", "ours_gap", "ours_accepts"):
        top = [(k, n) for k, n in kinds.most_common() if k[0] == o][:12]
        if top:
            print(f"{o} (xed, ours, lengths): count, example")
            for k, n in top:
                print(f"  {n:6} {k[1:]} {example[k]}")
    if a.json:
        a.json.write_text(json.dumps({"cases": total, "chip": a.chip, "outcomes": dict(outcome),
                                      "kinds": [{"outcome": k[0], "xed": k[1], "ours": k[2],
                                                 "lengths": k[3], "count": n, "example": example[k]}
                                                for k, n in kinds.most_common()]}, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
