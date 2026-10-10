# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""approximation_tables.py — specs/approximations.json from one approximations probe run (Sprint 046).

The probe (probes/approximations) tabulated RCPPS and RSQRTPS on one CPU: 4,096 results per table (the
mantissa's top 12 bits decide each result: every run's `varies` is 0), and exponent scaling exact (every
run's `mismatches` is 0). This script refuses a run where either does not hold, and writes the tables
with the CPU's name and the run they came from.

Usage: uv run python scripts/approximation_tables.py <store/probe-runs/<run> | build/probe-runs/.../traces-*>
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "specs" / "approximations.json"


def main(argv: list[str]) -> int:
    where = Path(argv[0])
    rows = {r["s"]: r for r in (json.loads(line) for line in (where / "body-1.jsonl").read_text().splitlines())}
    for name in ("rcp.table", "rsqrt.table.even", "rsqrt.table.odd"):
        if rows[name]["varies"]:
            raise SystemExit(f"{name}: {rows[name]['varies']} inputs not decided by 12 bits")
    for name in ("rcp.scaling", "rsqrt.scaling.even", "rsqrt.scaling.odd"):
        if rows[name]["mismatches"]:
            raise SystemExit(f"{name}: {rows[name]['mismatches']} scaling mismatches")
    spec = {"spec_id": "w2m.approximations", "version": "0.1.0", "cpu": (where / "cpu.txt").read_text().strip(),
            "source": str(where.relative_to(ROOT)) if where.is_relative_to(ROOT) else str(where),
            "rule": ("a normal input's result is the table entry for its mantissa's top 12 bits, the exponent moved by "
                     "the input's (RCP: -(e - 127); RSQRT: -(e - e0) / 2 from the table of e's parity, e0 127 or 128); "
                     "a result below the normal range is zero; specials as the rows"),
            "rcp": rows["rcp.table"]["out"], "rsqrt_even": rows["rsqrt.table.even"]["out"],
            "rsqrt_odd": rows["rsqrt.table.odd"]["out"],
            "specials": {k.split(".", 1)[1]: [v["rcp"], v["rsqrt"]] for k, v in rows.items() if k.startswith("special.")}}
    SPEC.write_text(json.dumps(spec) + "\n")
    print(f"{SPEC.relative_to(ROOT)}: {spec['cpu']}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
