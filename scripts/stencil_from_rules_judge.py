# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""stencil_from_rules_judge.py — stencils made by running compile_insn on symbolic templates (src/w2m/symbolic_compile.py)
against compile_insn itself (Sprint 090).

For every instruction of a source at flag contexts drawn from the record's commonest and the two defaults, at a base
and exit drawn at random in the monitor: the stencil's words, evaluated, against compile_insn's. Counts: equal,
differing (a defect), refused (a path that raises; the host compiles it), lift refuses; then the stencils made, by
template and in all, and the time.

Usage: uv run python scripts/stencil_from_rules_judge.py [--source record|spec|cases] [--limit N]
"""

from __future__ import annotations

import argparse
import random
import sys
import time
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path[:0] = [str(ROOT / "src"), str(ROOT / "scripts"), str(ROOT / "tests")]

from w2m import vm_backend as vb  # noqa: E402
from w2m.lift import LiftError, lift  # noqa: E402
from w2m.symbolic_compile import NZCV_FROM, Table, compile_words, words_by_stencil  # noqa: E402

SYMBOLS = {"w2m_cpuid": vb.MONITOR_VA + vb.RUNTIME_IMAGE + 0x2000, "w2m_approximate": vb.MONITOR_VA + vb.RUNTIME_IMAGE + 0x3000,
           "w2m_pcmpistri": vb.MONITOR_VA + vb.RUNTIME_IMAGE + 0x4000}


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--source", default="record", choices=("record", "spec", "cases"))
    ap.add_argument("--limit", type=int, default=0)
    a = ap.parse_args(argv)
    import template_judge
    items = template_judge.sources(10)[a.source]
    rng = random.Random(9001)
    if a.limit:
        items = rng.sample(items, min(a.limit, len(items)))
    table = Table()
    c: Counter[str] = Counter()
    examples: list[str] = []
    t0 = time.monotonic()
    for code, address in items:
        try:
            lift(code, address)
        except LiftError:
            c["lift refuses"] += 1
            continue
        nzcv_from = rng.choice(NZCV_FROM)
        live = rng.choice([vb.ALL_FLAGS, 0, rng.randrange(64)])
        nzcv_out = rng.random() < 0.3
        base = vb.MONITOR_VA + vb.CODE_CACHE + rng.randrange(1 << 22) * 4
        exit_va = vb.MONITOR_VA + vb.DISPATCHER
        want = compile_words(code, address, base, exit_va, SYMBOLS, nzcv_from, live, nzcv_out)
        got, why = words_by_stencil(table, code + bytes(15), address, base, exit_va, SYMBOLS, nzcv_from, live, nzcv_out)
        if got is None:
            c[f"refused: {why.split(':')[0]}"] += 1
            if len(examples) < 10:
                examples.append(f"refused {code[:15].hex()} {nzcv_from} {live} {nzcv_out}: {why[:80]}")
        elif got == want:
            c["equal"] += 1
        else:
            c["differ"] += 1
            if len(examples) < 10:
                examples.append(f"differs {code[:15].hex()} {nzcv_from} {live} {nzcv_out}")
    n = sum(len(v) for v in table.stencils.values())
    print(f"{a.source}: {len(items)} instructions, {dict(c)}; {len(table.templates)} template keys, "
          f"{len(table.stencils)} (template, NZCV kind) pairs, {n} stencils, {time.monotonic() - t0:.0f} s")
    for e in examples:
        print("  " + e)
    return 1 if c["differ"] else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
