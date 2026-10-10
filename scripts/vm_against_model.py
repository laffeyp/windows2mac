# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""vm_against_model.py — a case set run in the VM and judged against the model, with no runner (Sprint 047).

scripts/vm_oracle.py judges the VM against a Windows result file. This script makes the result file from
the model instead (runtime.Runtime, one step per case, as semantics_differential.model runs it) and hands
it to the same judge, so a new class's lowering is checked against its model the hour it is written. It
judges w2m against itself: the CPU's verdict still comes from a runner, and this records nothing.

Usage: uv run python scripts/vm_against_model.py <set> [--limit N]
Exit 0 when the VM equals the model on every case it compiles.
"""

from __future__ import annotations

import argparse
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "src"))

import semantics_differential as sd  # noqa: E402
import vm_oracle  # noqa: E402


def model_result(c: sd.Case) -> sd.Result | None:
    """The model's state after `c` as a Windows result would carry it; None when the model stopped for a
    reason that is not a guest fault."""
    m = sd.model(c)
    if isinstance(m, str):
        return None
    th, mem = m.rt.th, m.rt.mem
    rflags = 0x2 | sum(1 << b for f, b in sd.FLAG_BITS.items() if th.flags.get(f)) | int(th.df) << sd.DF_BIT
    data = bytes(mem.read(sd.DATA_VA + sd.DATA_OFF, sd.WINDOW))
    stack = bytes(mem.read(sd.STACK_VA + sd.STACK_OFF, sd.WINDOW))
    edge = bytes(mem.read(sd.DATA_VA + sd.EDGE_OFF, sd.EDGE))
    if m.fault:
        # a fault's CONTEXT.EFlags has IF (bit 9) set, as Windows gives it to a vectored handler
        return sd.Result(list(th.regs), rflags | 0x200, 2, m.fault, data, stack, sd.CODE_VA, th.mxcsr, list(th.xmm), edge)
    paths = {sd.CODE_VA + len(c.code): 0, sd.CODE_VA + len(c.code) + 14: 1, sd.RET_TARGET: 1}
    return sd.Result(list(th.regs), rflags, paths.get(th.rip, 3), 0, data, stack, 0, th.mxcsr, list(th.xmm), edge)


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("set", choices=sorted(sd.CASE_SETS))
    ap.add_argument("--limit", type=int)
    a = ap.parse_args(argv)
    cases = sd.cached_cases(a.set)
    pairs = [(c, model_result(c)) for c in cases]
    stopped = Counter(c.label for c, r in pairs if r is None)
    kept = [(c, r) for c, r in pairs if r is not None]
    classes, example, skipped, judged, faults = vm_oracle.run_cases([c for c, _ in kept], [r for _, r in kept], a.limit)
    print(f"set {a.set}: {len(cases)} cases; model stopped on {sum(stopped.values())} {dict(stopped)}; "
          f"VM judged {judged} ({faults} faults) against the model; difference classes {len(classes)}; "
          f"not compiled {sum(skipped.values())}")
    for k, n in classes.most_common(30):
        print(f"  {n:6} {k}: {example[k]}")
    return 1 if classes or stopped else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
