# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""A run's numbers from its record alone (vocabulary 0.9, P20; Sprint 108): every MEASUREMENT_RECORDED read through
substrate, printed by metric, then what does not add up (run_frontier.measurement_problems). Exit 1 when something
does not.

Usage: uv run python scripts/run_numbers.py store/runs/<record>
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from substrate.api import read_record  # noqa: E402

import run_frontier  # noqa: E402


def main(argv: list[str]) -> int:
    record = Path(argv[0])
    recorded = [e["payload"] for e in read_record(record) if e["kind"] == "MEASUREMENT_RECORDED"]
    table = run_frontier.measurement_table(recorded)
    units = {(m["metric"], m.get("variant", "")): m["unit"] for m in recorded}
    for (metric, variant), value in sorted(table.items()):
        print(f"{metric:26} {variant:28} {value:>22,.3f} {units[(metric, variant)]}")
    problems = run_frontier.measurement_problems(table)
    for p in problems:
        print(f"does not add up: {p}")
    print(f"{record.name}: {len(table)} numbers, {len(problems)} that do not add up")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
