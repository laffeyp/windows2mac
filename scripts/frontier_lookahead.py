# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""frontier_lookahead.py — list the imports a program calls past its frontier (Sprint 047).

The frontier (scripts/run_frontier.py) stops at the first import w2m lacks. This run instead has
each such call return 0 and goes on, so it lists the calls that follow: which DLL and function,
how often, from where, in first-call order. Returning 0 is not Windows's behaviour, so everything
after the first stop is a guess about the program's path, not a judgment; it plans families and
records nothing.

Usage: uv run python scripts/frontier_lookahead.py <exe> [--max-steps N] [--limit calls]
Prints one line per import in first-call order: count, name, first return address; then the stop.
"""

from __future__ import annotations

import argparse
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from w2m.file_bridge import FileBridge  # noqa: E402
from w2m.loader import load  # noqa: E402
from w2m.lift import RAX  # noqa: E402
from w2m.vm_backend import USER_ENVIRONMENT, VM  # noqa: E402


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("exe", type=Path)
    ap.add_argument("--max-steps", type=int, default=50_000_000)
    ap.add_argument("--limit", type=int, default=5_000, help="stop after this many returned calls")
    a = ap.parse_args(argv)
    here = a.exe.resolve().parent
    files = FileBridge(here, read_only=True)          # C:\\w2m\\app, read-only (Sprint 048)
    vm = VM(load(a.exe.read_bytes(), lambda n: (here / n).read_bytes() if (here / n).is_file() else None,
                 a.exe.name), image_name=a.exe.name, gui=True, files=files, environment=USER_ENVIRONMENT)
    counts: Counter[str] = Counter()
    first: dict[str, int] = {}

    def returned(v: VM, x: list[int], name: str) -> bool:
        if sum(counts.values()) >= a.limit:
            return False
        counts[name] += 1
        first.setdefault(name, int.from_bytes(v.mem.read(x[4], 8), "little"))
        x[RAX] = 0
        return True

    try:
        r = vm.run(max_steps=a.max_steps, timeout_ms=120_000, on_unimplemented=returned)
    finally:
        vm.close()
        files.close_all()
    for name, ret in first.items():
        print(f"{counts[name]:>7}  {name}  0x{ret:x}")
    print(f"stop after {r.instructions_executed:,} instructions: {r.stop}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
