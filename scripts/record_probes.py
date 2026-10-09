"""record_probes.py — PROBE_AUTHORED for every probe binary in the content store (Sprint 038).

A probe program enters the store when its binary is kept: store/binaries/<sha256>.exe (built on the
runner and copied back) and store/probe-runs/<run>/probe.exe (each runner build; clang on the
runner does not produce identical bytes for identical source, so each build is its own entry).
Each is authored by hand (origin hand) from its probes/<dir> source; the contracts it is judged
against are listed by probe.

Usage: uv run python scripts/record_probes.py [--runs runs]
"""

from __future__ import annotations

import argparse
import hashlib
import sys
import time
from collections.abc import Iterator
from pathlib import Path

import msgspec

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
from w2m import VOCAB_VERSION, events  # noqa: E402
from w2m.record import one_step_run  # noqa: E402

LAST_ERROR, EVENT = "windows.thread.last_error.set_get", "windows.event.auto_reset.single_signal"
STORED = {  # store/binaries prefix -> (probe directory, contracts); identified by each binary's usage string
    "9708e648": ("hello", []), "98e74d00": ("event_auto_reset", [EVENT]),
    "18f721e6": ("skeleton", [LAST_ERROR]), "d210df33": ("last_error_values", [LAST_ERROR]),
}
RUNS = {"37989842529": ("guest_state", ["windows.guest_state.layout"])}   # other probe runs: semantics


def probes() -> Iterator[tuple[str, str, list[str]]]:
    for f in sorted((ROOT / "store" / "binaries").glob("*.exe")):
        name, contracts = STORED[f.name[:8]]
        yield name, "sha256:" + hashlib.sha256(f.read_bytes()).hexdigest(), contracts
    for d in sorted((ROOT / "store" / "probe-runs").iterdir()):
        exe = d / "probe.exe"
        if exe.exists():
            name, contracts = RUNS.get(d.name, ("semantics", ["x86.semantics"]))
            yield f"{name}-run-{d.name}", "sha256:" + hashlib.sha256(exe.read_bytes()).hexdigest(), contracts


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", type=Path, default=ROOT / "runs")
    a = ap.parse_args(argv)
    found = list(probes())

    def work() -> Iterator[msgspec.Struct]:
        for name, sha, contracts in found:
            yield events.PROBE_AUTHORED(probe_id=name, probe_sha256=sha, contract_ids=contracts, origin="hand")

    root, kinds = one_step_run(a.runs / f"probes-{time.time_ns()}",
                               profile="store", contract_version=VOCAB_VERSION, purpose="contract_construction",
                               work=work, baseline={"probes": len(found)})
    print(dict(kinds))
    print(f"record {root}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
