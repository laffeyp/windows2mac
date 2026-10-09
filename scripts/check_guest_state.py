"""check_guest_state.py — judge specs/guest_state.json against probes/guest_state runs (Sprint 034).

Three comparisons, each field by field:
  layout   — every CONTEXT, XMM_SAVE_AREA32, EXCEPTION_RECORD, TEB and PEB offset the spec states
             from a header equals the runner's offsetof/sizeof, in every run;
  observed — every TEB/PEB field the spec takes from observation names a check that was equal in
             every run, at the offset the spec states;
  defaults — MXCSR, the x87 control word and IF at thread start equal the runner's fault CONTEXT.
Also: the layout lines are identical across runs, and every run's summary has 0 failures.

--record writes one substrate run: SESSION_INIT, COMPARISON_COMPLETED (contract
windows.guest_state.layout at the spec version, relation exact, reference the runner profile),
DISCREPANCY_REPORTED and DISCREPANCY_ROUTED (contract_revision: the spec is wrong, Windows is the
reference) per differing field, SESSION_COMPLETED.

Usage: uv run python scripts/check_guest_state.py store/probe-runs/<run id> [--record] [--runs runs]
"""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
import sys
import time
from collections import Counter
from collections.abc import AsyncIterator
from pathlib import Path
from typing import Any

import msgspec
from substrate.api import Runtime, TopologyBuilder, read_record, threshold_count

from w2m import VOCAB_VERSION, events

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "specs" / "guest_state.json"
EMPTY = "sha256:" + hashlib.sha256(b"").hexdigest()


def runs_of(d: Path) -> list[list[dict[str, Any]]]:
    return [[json.loads(line) for line in p.read_text().splitlines() if line.strip()]
            for p in sorted(d.glob("body-*.jsonl"))]


def compare(spec: dict[str, Any], runs: list[list[dict[str, Any]]]) -> tuple[int, list[str]]:
    """Number of fields compared, and one line per difference."""
    diffs: list[str] = []
    n = 0
    layouts = [{r["name"]: r["value"] for r in run if r["kind"] == "layout"} for run in runs]
    if any(lay != layouts[0] for lay in layouts[1:]):
        diffs.append("layout lines differ between runs")
    checks = [{r["name"]: r for r in run if r["kind"] == "check"} for run in runs]
    for i, run in enumerate(runs, 1):
        summary = [r for r in run if r["kind"] == "summary"]
        if not summary or summary[0]["failures"] != 0:
            diffs.append(f"run {i}: summary {summary[0] if summary else 'missing'}")
    w = spec["windows"]
    for struct in ("CONTEXT", "XMM_SAVE_AREA32", "EXCEPTION_RECORD"):
        for field, want in w[struct]["fields"].items():
            n += 1
            got = layouts[0].get(f"{struct}.{field}")
            if got != want:
                diffs.append(f"{struct}.{field}: spec {want:#x}, runner {got if got is None else hex(got)}")
    for struct in ("TEB", "PEB"):
        for f in w[struct]["fields"]:
            n += 1
            name = f["name"]
            if f["source"] == "winternl.h":
                got = layouts[0].get(f"{struct}.{name}")
                if got != f["offset"]:
                    diffs.append(f"{struct}.{name}: spec {f['offset']:#x}, runner {got if got is None else hex(got)}")
            if "check" in f:
                for i, c in enumerate(checks, 1):
                    r = c.get(f["check"])
                    if r is None or not r["equal"]:
                        diffs.append(f"{struct}.{name}: run {i} check {f['check']} {r}")
                    elif r["offset"] >= 0 and r["offset"] != f["offset"] and f["check"] != "teb.tls_slot":
                        diffs.append(f"{struct}.{name}: spec offset {f['offset']:#x}, check read {r['offset']:#x}")
                    elif f["check"] == "teb.tls_slot" and not 0x1480 <= r["offset"] < 0x1480 + 512:
                        diffs.append(f"{struct}.{name}: slot read at {r['offset']:#x}, outside TlsSlots")
            elif f["source"] == "observation":
                diffs.append(f"{struct}.{name}: source observation but no check named")
    d = w["defaults_at_thread_start"]
    for key, check, want in (("mxcsr", "fault.mxcsr", d["mxcsr"]),
                             ("x87_control_word", "fault.x87_control_word", d["x87_control_word"]),
                             ("rflags_if", "fault.eflags_if", d["rflags_if"])):
        n += 1
        for i, c in enumerate(checks, 1):
            r = c.get(check)
            if r is None or r["observed"] != want:
                diffs.append(f"defaults.{key}: spec {want:#x}, run {i} {r and hex(r['observed'])}")
    return n, diffs


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("run_dir", type=Path)
    ap.add_argument("--record", action="store_true")
    ap.add_argument("--runs", type=Path, default=Path("runs"))
    a = ap.parse_args(argv)
    t0 = time.monotonic()
    spec = json.loads(SPEC.read_text())
    runs = runs_of(a.run_dir)
    profile = (a.run_dir / "profile.txt").read_text().strip()
    n, diffs = compare(spec, runs)
    print(f"spec {spec['version']}; runs {len(runs)} on {profile}; fields compared {n}; differences {len(diffs)}")
    for d in diffs:
        print(f"  {d}")
    if a.record:
        record(a, spec["version"], profile, n, diffs, time.monotonic() - t0)
    return 1 if diffs else 0


def record(a: argparse.Namespace, version: str, profile: str, n: int, diffs: list[str],
           elapsed: float) -> None:
    run_id = f"gueststate-{time.time_ns()}"

    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile=profile, contract_version=version,
            harness_version="0.0.1", purpose="conformance"))
        cmp_id = f"cmp-{run_id}"
        yield events.validate(events.COMPARISON_COMPLETED(
            comparison_id=cmp_id, contract_id="windows.guest_state.layout", contract_version=version,
            relation="exact", reference_ref=profile, target_execution_id=a.run_dir.name,
            verdict="discrepancy" if diffs else "conforms",
            rule_ref=f"specs/guest_state.json {version}: {n} fields against {a.run_dir}"))
        for i, d in enumerate(diffs, 1):
            did = f"{run_id}-d{i}"
            yield events.validate(events.DISCREPANCY_REPORTED(
                discrepancy_id=did, contract_id="windows.guest_state.layout", contract_version=version,
                mismatch_class="exact", minimized_program_sha256=EMPTY, initial_state_sha256=EMPTY,
                first_divergence=d, reference_profile_id=profile,
                destination_profile_id=f"w2m.guest_state-{version}", comparison_id=cmp_id))
            yield events.validate(events.DISCREPANCY_ROUTED(
                discrepancy_id=did, repair_path="contract_revision", reference_conforms=True,
                grounds="Windows on the runner is the reference; the spec states the wrong value"))
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed_with_discrepancies" if diffs else "completed",
            discrepancy_count=len(diffs), accepted_count=0, elapsed_seconds=round(elapsed, 3)))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(spec_version=version, reference=profile, run_dir=str(a.run_dir))
        b.producer_kind("comparator", schemas=list(events.HARNESS_EVENTS), schema_version=1,
                        factory=lambda: harness)
        b.initial("comparator", input=None)
        b.termination(threshold_count("substrate.ProducerCompleted", 1))

    result = asyncio.run(Runtime(a.runs / run_id).run(topology))
    kinds = Counter(e["kind"] for e in read_record(result.record_root)
                    if not e["kind"].startswith("substrate."))
    print(f"record {result.record_root}: {dict(kinds)}")


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
