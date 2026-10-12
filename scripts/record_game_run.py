# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""record_game_run.py — a game run on a Windows runner as a reference execution on the record (Sprint 060).

The frontier's call stream is compared with the game's calls on Windows (scripts/call_compare.py), and a comparison
cites its reference by execution id (COMPARISON_COMPLETED.reference_ref; Drift watchlist: an EXECUTION_STARTED on a
windows_reference profile). The game runs under store/game-runs/ had no execution on the record. This writes one per
run: EXECUTION_STARTED with the game's executable (the digest games/<game>/files.json gives it; the workflow checks
every file against that list before the start) and the runner's profile, and EXECUTION_COMPLETED with the run's end
and the call trace's digest and line count. The trace is call_trace.h's text, not a vocabulary guest trace: the
capture does not record where threads start, so `unobserved` says so.

Usage: uv run python scripts/record_game_run.py store/game-runs/<run> [--game hail-to-the-rainbow]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

import msgspec

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
from w2m import VOCAB_VERSION, events  # noqa: E402
from w2m.record import one_step_run, uncommitted  # noqa: E402

EMPTY = "sha256:" + hashlib.sha256(b"").hexdigest()


def execution_id(run: Path) -> str:
    return f"game-run-{run.name}"


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("run", type=Path)
    ap.add_argument("--game", default="hail-to-the-rainbow")
    ap.add_argument("--exe", default="Hail to the Rainbow.exe")
    a = ap.parse_args(argv)
    changed = uncommitted()
    if changed:
        print("refused: store/runs/ records name the commit that ran; uncommitted: " + ", ".join(changed[:10]))
        return 2
    run = a.run.resolve()
    listing = json.loads((ROOT / "games" / a.game / "files.json").read_text())["listing"]
    exe = next(f for f in listing if f["path"] == a.exe)
    trace = run / "jit_capture.bin.trace.tsv"
    lines = len([ln for ln in trace.read_text().splitlines() if ln])
    ended = json.loads((run / "exec.json").read_text())
    profile = (run / "profile.txt").read_text().split()[0]
    xid = execution_id(run)

    def work() -> list[msgspec.Struct]:
        return [
            events.validate(events.EXECUTION_STARTED(
                execution_id=xid, subject_kind="application", subject_sha256="sha256:" + exe["sha256"],
                profile_kind="windows_reference", profile_id=profile, initial_state_sha256=EMPTY,
                injected_inputs_sha256=EMPTY, timeout_seconds=float(round(ended["elapsed_seconds"])),
                unobserved="initial state and injected inputs: the capture records neither")),
            events.validate(events.EXECUTION_COMPLETED(
                execution_id=xid, exit_status="exited" if ended["exited_by_itself"] else "timed_out",
                trace_sha256="sha256:" + hashlib.sha256(trace.read_bytes()).hexdigest(), guest_line_count=lines,
                elapsed_seconds=ended["elapsed_seconds"],
                **({"exit_code": ended["exit_code"]} if ended["exit_code"] is not None else {}),
                unobserved=f"the trace is {trace.relative_to(ROOT)} in tools/jit_capture/call_trace.h's columns, not "
                           "a vocabulary guest trace: the capture does not record where threads start"))]

    root, kinds = one_step_run(ROOT / "store" / "runs" / xid, profile="windows_reference",
                               contract_version=VOCAB_VERSION, purpose="application", work=work,
                               baseline={"capture": str(run.relative_to(ROOT)), "game": a.game})
    print(root, dict(kinds))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
