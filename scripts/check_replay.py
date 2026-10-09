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
    print(f"{len(records)} records under {STORE}; {len(records) - len(failed)} replay at level 2")
    for f in failed:
        print(f"  {f}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
