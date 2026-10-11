# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""check_replay.py — every committed substrate record replays (review 2026-10-09 progress S5).

Runs substrate's assert_replayable at level 2 (event hashes re-derived) over each record under
store/runs/; a record that does not replay honestly fails the gate. Exit 0 when all replay.

Usage: uv run python scripts/check_replay.py
"""

from __future__ import annotations

import sys
from pathlib import Path

from substrate.api import assert_replayable

STORE = Path(__file__).resolve().parent.parent / "store" / "runs"


def main() -> int:
    failed = []
    records = sorted(d for d in STORE.iterdir() if d.is_dir())
    for d in records:
        try:
            assert_replayable(d, "2")
        except Exception as e:                               # noqa: BLE001 — every failure is reported
            failed.append(f"{d.name}: {type(e).__name__}: {e}")
    # level 2 checks each record's integrity (the stream, its counts, every trigger's input hash; substrate docs/api.md);
    # it re-runs nothing, so it says nothing about whether w2m still behaves as recorded (review 2026-10-10 F3)
    print(f"{len(records)} records under {STORE}; {len(records) - len(failed)} intact (substrate replay level 2: "
          f"stream, counts and trigger inputs; nothing re-run)")
    for f in failed:
        print(f"  {f}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
