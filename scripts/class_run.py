"""class_run.py — one instruction class judged on its own record (Sprint 041; docs/plan.md Phase 4).

An item run is one substrate run whose events cause one another:

  SESSION_INIT
    -> CANDIDATE_GENERATED      the class's rule (src/w2m/rules/<module>.py and the shared modules it
                                uses), identified by their sha256; generator_ref names who wrote it
    -> COMPARISON_COMPLETED x2  the class's oracle cases run in the model (runtime.py) and in the
                                managed VM, each against the CPU's recorded results
    -> CANDIDATE_ACCEPTED       when both conform
       or DISCREPANCY_REPORTED + DISCREPANCY_ROUTED per difference class (implementation_repair:
                                the CPU is the reference)
    -> SESSION_COMPLETED

The reference is the case set that covers the class and an Intel CPU's results for it (REFERENCES:
Xeon Platinum 8573C runs of case sets 039 and 041). `--pool N` judges every class the case file
covers in N processes, one run each.

Usage: uv run python scripts/class_run.py <ICLASS> [--runs store/runs]
       uv run python scripts/class_run.py --all [--pool 8] [--runs store/runs]
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from collections import Counter
from collections.abc import AsyncIterator
from concurrent.futures import ProcessPoolExecutor
from functools import cache
from pathlib import Path
from typing import Any

import msgspec

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "src"))

import semantics_differential as sd  # noqa: E402
import vm_oracle  # noqa: E402

from w2m import VOCAB_VERSION, events  # noqa: E402
from w2m.record import producer, run_with_provenance  # noqa: E402
from w2m.rules import registry  # noqa: E402
from w2m.x86dec import decode  # noqa: E402

# Each case set's reference: an Intel Xeon Platinum 8573C run of its case file
REFERENCES = {"039": "38011862284", "041": "38029980267", "042": "38031723882", "044": "38039360955"}
CASE_FILE_VERSIONS = {"039": "0.39.0", "041": "0.41.0", "042": "0.42.0", "044": "0.44.0"}     # the case set (its class list and seed)


def contract_id(iclass: str) -> str:
    return "x86.semantics." + iclass.lower()


def iclass_of(c: sd.Case) -> str:
    """The case's iclass: its label, except the control-flow cases labelled by condition code
    (Jcc_0 ... Jcc_f), which decode to JO ... JNLE."""
    return decode(c.code).iclass if c.label.startswith("Jcc_") else c.label


@cache
def _built(case_set: str) -> list[sd.Case]:
    seed, class_set, locked, control = sd.CASE_SETS[case_set]
    return sd.build_cases(seed, class_set, locked, control)[0]


def case_set_of(iclass: str) -> str:
    """The case set that covers `iclass` (each class is in exactly one)."""
    found = [k for k in sd.CASE_SETS if iclass in {iclass_of(c) for c in _built(k)}]
    if len(found) != 1:
        raise KeyError(f"{iclass}: covered by case sets {found}")
    return found[0]


def cases_for(iclass: str) -> tuple[str, list[sd.Case], list[sd.Result]]:
    case_set = case_set_of(iclass)
    built = _built(case_set)
    results = sd.read_results(vm_oracle.result_file(REFERENCES[case_set], case_set))
    keep = [i for i, c in enumerate(built) if iclass_of(c) == iclass]
    return case_set, [built[i] for i in keep], [results[i] for i in keep]


def judge_model(cases: list[sd.Case], results: list[sd.Result]) -> Counter[str]:
    out: Counter[str] = Counter()
    for c, r in zip(cases, results, strict=True):
        for k, _detail in sd.judge(c, r, c.label):
            out[k] += 1
    return out


def item_run(iclass: str, runs: Path, generator_ref: str = "hand (agent-authored)") -> dict[str, Any]:
    """Judge one class in one substrate run; return its verdict summary."""
    from substrate.api import Once, PerEvent, Subscription, TopologyBuilder, read_record, threshold_count

    rule = registry()[iclass]
    case_set, cases, results = cases_for(iclass)
    reference_run, version = REFERENCES[case_set], CASE_FILE_VERSIONS[case_set]
    store = (ROOT / "store" / "probe-runs" / reference_run)
    reference = f"{(store / 'cpu.txt').read_text().strip()}; {(store / 'profile.txt').read_text().strip()}"
    cid = contract_id(iclass)
    run_id = f"class-{iclass.lower()}-{time.time_ns()}"
    cand = f"rule-{rule.module}-{rule.sha256[7:19]}"
    t0 = time.monotonic()
    state: dict[str, Any] = {"verdicts": {}, "diffs": {}}

    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile=reference, contract_version=version,
            harness_version="0.0.1", purpose="conformance"))

    async def generator(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.CANDIDATE_GENERATED(
            candidate_id=cand, contract_id=cid, contract_version=version,
            generator_ref=f"{generator_ref}: src/w2m/rules/{rule.module}.py", attempt=1,
            candidate_sha256=rule.sha256))

    def judge(executor: str) -> Any:
        async def run(inp: dict[str, Any]) -> AsyncIterator[msgspec.Struct]:
            if executor == "model":
                diffs = judge_model(cases, results)
                example: dict[str, str] = {}
            else:
                diffs, example, skipped, judged, _faults = vm_oracle.run_cases(cases, results)
                if skipped:
                    diffs.update({f"not compiled: {k}": n for k, n in skipped.items()})
            state["diffs"][executor] = (diffs, example)
            verdict: events.EnumVerdict = "discrepancy" if diffs else "conforms"
            state["verdicts"][executor] = verdict
            first = next(iter(diffs.most_common(1)), None)
            yield events.validate(events.COMPARISON_COMPLETED(
                comparison_id=f"cmp-{cand}-{executor}", contract_id=cid, contract_version=version,
                relation="exact", reference_ref=f"semantics-{case_set} run {reference_run}",
                target_execution_id=f"{executor}-{cand}", verdict=verdict,
                rule_ref=f"{len(cases)} cases of {iclass}; candidate {inp['candidate_id']}",
                first_divergence=f"{first[0]} ({first[1]} cases)" if first else None))
        return run

    async def decide(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        if all(v == "conforms" for v in state["verdicts"].values()):
            yield events.validate(events.CANDIDATE_ACCEPTED(
                candidate_id=cand, contract_id=cid, contract_version=version,
                comparison_ids=tuple(f"cmp-{cand}-{e}" for e in ("model", "managed_vm"))))
            return
        empty = "sha256:" + "0" * 64
        n = 0
        for executor, (diffs, example) in state["diffs"].items():
            for k, count in diffs.most_common():
                n += 1
                did = f"{run_id}-d{n}"
                yield events.validate(events.DISCREPANCY_REPORTED(
                    discrepancy_id=did, contract_id=cid, contract_version=version,
                    mismatch_class="exact", minimized_program_sha256=empty, initial_state_sha256=empty,
                    first_divergence=f"{executor}: {k} ({count} cases) {example.get(k, '')}"[:500],
                    reference_profile_id=reference, destination_profile_id=f"w2m-{executor}",
                    comparison_id=f"cmp-{cand}-{executor}"))
                yield events.validate(events.DISCREPANCY_ROUTED(
                    discrepancy_id=did, repair_path="implementation_repair", reference_conforms=True,
                    grounds="the CPU's recorded result is the reference; the rule or its lowering differs"))

    async def close(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        bad = sum(1 for v in state["verdicts"].values() if v != "conforms")
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed_with_discrepancies" if bad else "completed",
            discrepancy_count=sum(sum(d.values()) > 0 for d, _ in state["diffs"].values()),
            accepted_count=0 if bad else 1, elapsed_seconds=round(time.monotonic() - t0, 3)))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(iclass=iclass, rule_sha256=rule.sha256, case_set=case_set, reference_run=reference_run,
                   cases=len(cases))
        b.producer_kind("harness", schemas=[events.SESSION_INIT], schema_version=1, factory=lambda: producer(harness))
        b.producer_kind("rule_source", schemas=[events.CANDIDATE_GENERATED], schema_version=1,
                        factory=lambda: producer(generator))
        b.producer_kind("model_judge", schemas=[events.COMPARISON_COMPLETED], schema_version=1,
                        factory=lambda: producer(judge("model")))
        b.producer_kind("vm_judge", schemas=[events.COMPARISON_COMPLETED], schema_version=1,
                        factory=lambda: producer(judge("managed_vm")))
        b.producer_kind("acceptance", schemas=[events.CANDIDATE_ACCEPTED, events.DISCREPANCY_REPORTED,
                        events.DISCREPANCY_ROUTED], schema_version=1, factory=lambda: producer(decide))
        b.producer_kind("harness_close", schemas=[events.SESSION_COMPLETED], schema_version=1,
                        factory=lambda: producer(close))
        b.initial("harness", input=None)

        def on(kinds: set[str]) -> Subscription:
            return Subscription(kinds=frozenset(kinds))

        b.trigger("generate", subscription=on({"SESSION_INIT"}), predicate=lambda c: True,
                  starts="rule_source", input_builder=lambda c: {}, policy=PerEvent())
        for name in ("model_judge", "vm_judge"):
            b.trigger(f"judge-{name}", subscription=on({"CANDIDATE_GENERATED"}), predicate=lambda c: True,
                      starts=name, policy=PerEvent(),
                      input_builder=lambda c: {"candidate_id": c.event.payload["candidate_id"]})
        b.trigger("decide", subscription=on({"COMPARISON_COMPLETED"}),
                  predicate=lambda c: len(state["verdicts"]) == 2, starts="acceptance", policy=Once(),
                  input_builder=lambda c: {})
        b.trigger("close", subscription=on({"substrate.ProducerCompleted"}),       # once the decision is written
                  predicate=lambda c: c.event.payload["producer"]["kind"] == "acceptance",
                  starts="harness_close", policy=PerEvent(), input_builder=lambda c: None)
        b.termination(threshold_count("SESSION_COMPLETED", 1))

    result = run_with_provenance(runs / run_id, topology)
    kinds = Counter(e["kind"] for e in read_record(result.record_root) if not e["kind"].startswith("substrate."))
    return {"iclass": iclass, "record": str(result.record_root), "rule_sha256": rule.sha256, "cases": len(cases),
            "verdicts": dict(state["verdicts"]), "events": dict(kinds)}


def classes_in_case_file() -> list[str]:
    """Every iclass with a rule that some case set covers."""
    return sorted({iclass_of(c) for k in sd.CASE_SETS for c in _built(k)} & set(registry()))


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("iclass", nargs="?")
    ap.add_argument("--all", action="store_true")
    ap.add_argument("--only", help="comma-separated iclasses, with --pool")
    ap.add_argument("--pool", type=int, default=8)
    ap.add_argument("--runs", type=Path, default=ROOT / "runs")
    a = ap.parse_args(argv)
    targets = classes_in_case_file() if a.all else a.only.split(",") if a.only else [a.iclass]
    t0 = time.monotonic()
    if len(targets) == 1:
        summaries = [item_run(targets[0], a.runs)]
    else:
        with ProcessPoolExecutor(a.pool) as pool:
            summaries = list(pool.map(item_run, targets, [a.runs] * len(targets)))
    accepted = [s for s in summaries if all(v == "conforms" for v in s["verdicts"].values())]
    for s in summaries:
        mark = "accepted" if s in accepted else "discrepancy"
        print(f"{s['iclass']:12} {mark:11} {s['cases']:5} cases  {s['verdicts']}  {s['record']}")
    print(f"{len(accepted)} of {len(summaries)} classes accepted; {time.monotonic() - t0:.0f} s")
    (a.runs / "class-runs.json").write_text(json.dumps(summaries, indent=1) + "\n")
    return 0 if len(accepted) == len(summaries) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
