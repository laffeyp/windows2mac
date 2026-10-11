# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""measure_exit.py — the cost of a VM exit, raw and through the monitor's pipe (Sprint 048; review 2026-10-10 F1).

Decision 97 measured "about 27 µs" per exit in lockstep, which crosses the monitor's text pipe and Python on every
exit. This measures the two apart on one guest loop (`hvc #0x7e; b . - 4`): the monitor resuming each exit in C,
and Python resuming each over the pipe. Records nothing; prints nanoseconds per exit and a frame's budget.

Usage: uv run python scripts/measure_exit.py [--exits 200000] [--pipe 20000] [--repeat 3]
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from w2m.loader import load  # noqa: E402
from w2m.vm_backend import VM  # noqa: E402

PROBE = ROOT / "store" / "binaries" / "2bc372b71143f8a74323f8ecac7f5a73ec622d6a329e9000647f5e779d6d8e04.exe"   # loader.exe


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--exits", type=int, default=200_000)
    ap.add_argument("--pipe", type=int, default=20_000)
    ap.add_argument("--repeat", type=int, default=3)
    a = ap.parse_args(argv)
    for i in range(a.repeat):
        vm = VM(load(PROBE.read_bytes(), lambda n: None, "loader.exe"))
        try:
            r = vm.measure_exits(a.exits, a.pipe)
        finally:
            vm.close()
        frame = 16_666_667
        print(f"repeat {i + 1}: raw {r['raw_ns']:.0f} ns per exit ({frame / r['raw_ns']:,.0f} in a 60 Hz frame); "
              f"through the pipe {r['pipe_ns']:.0f} ns ({frame / r['pipe_ns']:,.0f} in a frame)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
