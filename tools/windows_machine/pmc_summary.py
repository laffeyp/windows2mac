# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""pmc_summary.py — xperf's dump of a W2mPmc trace (pmc.wprp) reduced to two small tables (Sprint 068 item I).

Usage: python pmc_summary.py <dump.txt> <out dir> <image name part>

A `Pmc` row (TimeStamp, ThreadID, InstructionRetired) shares its timestamp with the `CSwitch` row that follows it and
holds the switching CPU's raw counter; the CSwitch row names that CPU (field 16) and the thread switched out (fields 8
and 9). The thread switched out ran the counter's rise since the same CPU's previous switch. Timestamps are
microseconds from the trace's start.

instructions.tsv: second, pid, process, tid, instructions, for every process whose name holds the image name part.
presents.tsv: second, pid, process, event, count: DXGI events whose name holds "Present", for the same processes.
"""

from __future__ import annotations

import sys
from collections import Counter
from pathlib import Path


def process_of(field: str) -> tuple[str, int]:
    name, _, pid = field.strip().rpartition("(")
    return name.strip(), int(pid.rstrip(") ").strip())


def main(argv: list[str]) -> int:
    dump, out, part = Path(argv[0]), Path(argv[1]), argv[2].lower()
    last: dict[int, int] = {}                       # CPU -> its counter at its last switch
    pending: tuple[int, int] | None = None          # the Pmc row waiting for its CSwitch: (timestamp, value)
    instructions: Counter[tuple[int, int, str, int]] = Counter()
    presents: Counter[tuple[int, int, str, str]] = Counter()
    unmatched = 0
    with dump.open(encoding="utf-8", errors="replace") as f:
        for line in f:
            head = line[:24].lstrip()
            if head.startswith("Pmc,"):
                p = [x.strip() for x in line.split(",")]
                if p[1].isdigit():
                    pending = (int(p[1]), int(p[3]))
            elif head.startswith("CSwitch,"):
                p = [x.strip() for x in line.split(",")]
                if not p[1].isdigit():
                    continue
                ts, cpu = int(p[1]), int(p[16])
                if pending is None or pending[0] != ts:
                    unmatched += 1
                    continue
                value = pending[1]
                pending = None
                before = last.get(cpu)
                last[cpu] = value
                if before is None or value < before:
                    continue
                name, pid = process_of(p[8])
                if part in name.lower():
                    instructions[(ts // 1_000_000, pid, name, int(p[9]))] += value - before
            elif "Present" in head or ("DXGI" in head and "Present" in line[:80]):
                p = [x.strip() for x in line.split(",")]
                if len(p) > 2 and p[1].isdigit() and "(" in p[2]:
                    name, pid = process_of(p[2])
                    if part in name.lower():
                        presents[(int(p[1]) // 1_000_000, pid, name, p[0])] += 1
    out.mkdir(parents=True, exist_ok=True)
    with (out / "instructions.tsv").open("w") as w:
        w.write("second\tpid\tprocess\ttid\tinstructions\n")
        for (s, pid, name, tid), n in sorted(instructions.items()):
            w.write(f"{s}\t{pid}\t{name}\t{tid}\t{n}\n")
    with (out / "presents.tsv").open("w") as w:
        w.write("second\tpid\tprocess\tevent\tcount\n")
        for (s, pid, name, ev), n in sorted(presents.items()):
            w.write(f"{s}\t{pid}\t{name}\t{ev}\t{n}\n")
    print(f"{sum(instructions.values())} instructions over {len({k[3] for k in instructions})} threads; "
          f"{sum(presents.values())} Present events; {unmatched} switches without their counter row")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
