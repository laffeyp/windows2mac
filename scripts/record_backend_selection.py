"""record_backend_selection.py — the runtime architecture decision on the signal plane (review
2026-10-09 progress K1).

The managed VM was chosen (BLACKBOARD 2026-10-09, runtime architecture; commit 0610f1e) from
measurements already on committed records. This run reads them and makes the choice inside the
topology, so the record holds the chain from evidence to decision:

  evaluator (one run per backend)  BACKEND_EVALUATED, from the measurement and capability records
  selector  (triggered once both evaluations exist)  BACKEND_SELECTED, choosing the backend whose
            evaluation has conformance passes and guest-enforced TSO and 4 KB pages

Inputs: store/runs/capability-1791496383, litmus-1791496387, ordering-1791442786,
pagegeom-1791443163 (Phase 2), and the managed-VM judge records of Sprint 035
(conformance of the two probes).

Usage: uv run python scripts/record_backend_selection.py [--runs runs]
"""

from __future__ import annotations

import argparse
import asyncio
import sys
from collections import Counter
from collections.abc import AsyncIterator
from pathlib import Path
from typing import Any

import msgspec
from substrate.api import PerEvent, Runtime, Subscription, TopologyBuilder, read_record, threshold_count

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
from w2m import VOCAB_VERSION, events  # noqa: E402

STORE = ROOT / "store" / "runs"
EVIDENCE = ("capability-1791496383", "litmus-1791496387", "ordering-1791442786", "pagegeom-1791443163")
APP = "hail-to-the-rainbow-1699230-24247624"
DEST = "mac17-7-macos-25f80"
DECISION = "BLACKBOARD Decisions 2026-10-09, runtime architecture (commit 0610f1e)"


def _evidence() -> tuple[dict[str, str], dict[str, float], list[str]]:
    capabilities: dict[str, str] = {}
    ns: dict[str, float] = {}
    ids: list[str] = []
    for name in EVIDENCE:
        for e in read_record(STORE / name):
            p = e["payload"]
            if e["kind"] == "CAPABILITY_PROBED":
                capabilities[p["capability"]] = p["outcome"]
            elif e["kind"] == "MEASUREMENT_RECORDED":
                ns[p["measurement_id"]] = float(p["value"])
                ids.append(p["measurement_id"])
    return capabilities, ns, ids


def _vm_conformance() -> tuple[int, int]:
    verdicts: Counter[str] = Counter()
    for d in STORE.iterdir():
        if not d.name.startswith("judge-"):
            continue
        evs = list(read_record(d))
        if any(e["kind"] == "EXECUTION_STARTED" and e["payload"].get("profile_kind") == "managed_vm" for e in evs):
            verdicts.update(e["payload"]["verdict"] for e in evs if e["kind"] == "COMPARISON_COMPLETED")
    return verdicts["conforms"], verdicts["discrepancy"]


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", type=Path, default=ROOT / "runs")
    a = ap.parse_args(argv)
    capabilities, ns, ids = _evidence()
    vm_pass, vm_fail = _vm_conformance()
    evaluations: dict[str, dict[str, Any]] = {}

    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile=DEST, contract_version=VOCAB_VERSION,          # no contract is judged here
            harness_version="0.0.1", purpose="measurement"))

    async def evaluator(inp: dict[str, Any]) -> AsyncIterator[msgspec.Struct]:
        backend = inp["backend"]
        if backend == "managed_vm":
            mids = [m for m in ids if "entso" in m or "guest" in m or m.startswith("pagegeom")]
            ratio = ns.get("ordering-l1-entso_on", 0) / ns["ordering-l1-plain"]
            objective = (f"guest TSO {capabilities.get('guest_tsoen')}, 4 KB guest pages "
                         f"{capabilities.get('ipa_granule_4k')}; TSO access {ratio:.2f}x plain (L1)")
            passes, fails = vm_pass, vm_fail
        else:
            mids = [m for m in ids if any(k in m for k in ("ldapr", "ldar", "dmb", "native"))]
            ratio = ns["ordering-l1-ldapr_stlr"] / ns["ordering-l1-plain"]
            objective = (f"process TSO control {capabilities.get('native_process_tso')}, mprotect at 4 KB "
                         f"{capabilities.get('native_mprotect_4k', 'absent')}; cheapest ordered lowering "
                         f"{ratio:.2f}x plain (L1); no conformance run (no native back end built)")
            passes, fails = 0, 0
        evaluations[backend] = {"pass": passes, "tso": backend == "managed_vm"}
        yield events.validate(events.BACKEND_EVALUATED(
            evaluation_id=f"eval-{backend}", backend=backend, application_profile_id=APP,
            destination_profile_id=DEST, conformance_pass_count=passes, conformance_fail_count=fails,
            objective_ref=objective, measurement_ids=mids))

    async def selector(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        chosen = max(evaluations, key=lambda b: (evaluations[b]["tso"], evaluations[b]["pass"]))
        yield events.validate(events.BACKEND_SELECTED(
            backend=chosen, application_profile_id=APP, destination_profile_id=DEST,
            evaluation_ids=[f"eval-{b}" for b in sorted(evaluations)], decision_ref=DECISION))
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed", discrepancy_count=0, accepted_count=0, elapsed_seconds=0.0))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(evidence=list(EVIDENCE), decision=DECISION)
        b.producer_kind("harness", schemas=[events.SESSION_INIT], schema_version=1, factory=lambda: harness)
        b.producer_kind("evaluator", schemas=[events.BACKEND_EVALUATED], schema_version=1, factory=lambda: evaluator)
        b.producer_kind("selector", schemas=[events.BACKEND_SELECTED, events.SESSION_COMPLETED],
                        schema_version=1, factory=lambda: selector)
        b.initial("harness", input=None)
        b.trigger("evaluate-vm", subscription=Subscription(kinds=frozenset({"SESSION_INIT"})),
                  predicate=lambda c: True, starts="evaluator", policy=PerEvent(),
                  input_builder=lambda c: {"backend": "managed_vm"})
        b.trigger("evaluate-native", subscription=Subscription(kinds=frozenset({"SESSION_INIT"})),
                  predicate=lambda c: True, starts="evaluator", policy=PerEvent(),
                  input_builder=lambda c: {"backend": "native_process"})
        b.trigger("select", subscription=Subscription(kinds=frozenset({"BACKEND_EVALUATED"})),
                  predicate=lambda c: len(evaluations) == 2, starts="selector", policy=PerEvent(),
                  input_builder=lambda c: None)
        b.termination(threshold_count("SESSION_COMPLETED", 1))

    result = asyncio.run(Runtime(a.runs / "backend-selection").run(topology))
    for e in read_record(result.record_root):
        if e["kind"] in ("BACKEND_EVALUATED", "BACKEND_SELECTED"):
            print(e["kind"], {k: e["payload"][k] for k in e["payload"] if k in ("backend", "conformance_pass_count", "objective_ref", "evaluation_ids")})
    print(f"record {result.record_root}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
