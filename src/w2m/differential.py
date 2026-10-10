"""differential.py — one causal topology for every differential (Sprint 043; review 2026-10-09 progress S1).

A differential judges an implementation (the decoder, the lifter's rules, the guest-state spec, the
VM) against a reference (Intel XED, a CPU's recorded results, Windows on the runner), unit by unit.
One run per differential:

  SESSION_INIT
    -> CANDIDATE_GENERATED per unit       the implementation as judged on that unit (its sha256);
                                          discrepancy_id set when the ledger holds one open on the
                                          unit: the run is then a repair attempt
    -> COMPARISON_COMPLETED               caused by its unit's candidate
    -> CANDIDATE_ACCEPTED, or CANDIDATE_REJECTED and DISCREPANCY_REPORTED, caused by the comparison
    -> DISCREPANCY_ROUTED                 caused by its report
    -> DISCREPANCY_RESOLVED               caused by an acceptance, one per discrepancy the ledger
                                          holds open on that unit from an earlier run
    -> SESSION_COMPLETED                  once every unit is decided and every route and resolution
                                          is written

The ledger (`store/runs/discrepancy-ledger.json`) is a function of the records: a REPORTED on a
differential's record opens an entry, a RESOLVED closes it. `rebuild_ledger` derives it; the file
is that derivation, written after each run.
"""

from __future__ import annotations

import asyncio
import hashlib
import json
import time
from collections import Counter
from collections.abc import AsyncIterator, Callable, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import msgspec
from substrate.api import Once, PerEvent, Runtime, Subscription, TopologyBuilder, read_record, threshold_count

from w2m import VOCAB_VERSION, events
from w2m.record import Purpose, producer

EMPTY = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
LEDGER = "discrepancy-ledger.json"


@dataclass(frozen=True)
class Verdict:
    """One unit's comparison. `differences` are the lines behind a discrepancy (the first is the
    first divergence); routing follows the vocabulary: the reference conforms and the target does
    not -> implementation_repair; the reference itself is what the contract states wrongly ->
    contract_revision."""

    verdict: events.EnumVerdict
    differences: tuple[str, ...] = ()
    rule_ref: str | None = None
    repair_path: events.EnumRepairPath = "implementation_repair"
    grounds: str = ""
    program: bytes | None = None               # the first difference's input bytes, stored by sha256


@dataclass(frozen=True)
class Differential:
    name: str                                  # record prefix and ledger key: "guest-state", "decoder-xed"
    profile: str                               # the reference, as SESSION_INIT and reports name it
    purpose: Purpose
    contract_id: Callable[[str], str]          # unit -> contract id
    contract_version: str
    relation: events.EnumRelation
    implementation: str                        # what is judged, e.g. "specs/guest_state.json"
    implementation_sha256: str
    generator_ref: str                         # who wrote the implementation
    reference_ref: str                         # an execution or run id
    target_execution_id: str
    units: Sequence[str]
    compare: Callable[[str], Verdict]
    baseline: dict[str, Any] = field(default_factory=dict)
    unit_sha256: Callable[[str], str] | None = None   # a unit's own implementation hash (a class's rule)
    programs: Path | None = None               # where a report's example bytes go (store/programs)


def by_unit(units: Sequence[str], keyed: Counter[str], example: dict[str, str]) -> dict[str, tuple[str, ...]]:
    """Difference keys of the form '<unit> <what>' grouped per unit, most frequent first, each as
    '<key>: <n> cases; <example>'."""
    out: dict[str, list[str]] = {u: [] for u in units}
    for k, n in keyed.most_common():
        out.setdefault(k.split(" ", 1)[0], []).append(f"{k}: {n} cases; {example.get(k, '')}")
    return {u: tuple(v) for u, v in out.items()}


def _safe(unit: str) -> str:
    return "".join(c if c.isalnum() or c in "._-" else "-" for c in unit)


def candidate_id(d: Differential, unit: str) -> str:
    return f"{d.name}:{_safe(unit)}@{d.implementation_sha256[7:19]}"


def rebuild_ledger(runs: Path) -> dict[str, dict[str, Any]]:
    """Every discrepancy a differential record reported, with the record that resolved it (or None).
    Records are read in run-start order, so a resolution always follows its report."""
    started: list[tuple[float, Path, str]] = []
    for root in runs.iterdir():
        if not root.is_dir():
            continue
        try:
            first = next(read_record(root))
        except (StopIteration, Exception):  # noqa: BLE001 — not a record
            continue
        diff = (first.get("payload") or {}).get("baseline", {}).get("differential")
        if diff:
            started.append((first["t"], root, diff))
    ledger: dict[str, dict[str, Any]] = {}
    for _, root, diff in sorted(started):
        for e in read_record(root):
            p = e["payload"]
            if e["kind"] == "DISCREPANCY_REPORTED":
                ledger[p["discrepancy_id"]] = {"differential": diff, "unit": p["first_divergence"].split(":", 1)[0],
                                               "contract_id": p["contract_id"], "reported_in": root.name,
                                               "first_divergence": p["first_divergence"], "repair_path": None,
                                               "resolved_in": None, "resolving_ref": None}
            elif e["kind"] == "DISCREPANCY_ROUTED" and p["discrepancy_id"] in ledger:
                ledger[p["discrepancy_id"]]["repair_path"] = p["repair_path"]
            elif e["kind"] == "DISCREPANCY_RESOLVED" and p["discrepancy_id"] in ledger:
                ledger[p["discrepancy_id"]].update(resolved_in=root.name, resolving_ref=p["resolving_ref"])
    return ledger


def open_on(ledger: dict[str, dict[str, Any]], differential: str, unit: str) -> list[str]:
    return sorted(k for k, v in ledger.items()
                  if v["differential"] == differential and v["unit"] == unit and v["resolved_in"] is None)


def run(d: Differential, runs: Path) -> dict[str, Any]:
    """One causal run of `d` under `runs`; update the ledger; return a summary."""
    ledger = rebuild_ledger(runs) if runs.exists() else {}
    run_id = f"{d.name}-{time.time_ns()}"
    units = list(d.units)
    t0 = time.monotonic()
    state: dict[str, Any] = {"verdicts": {}, "decided": 0, "reported": 0, "routed": 0,
                             "to_resolve": 0, "resolved": 0}

    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile=d.profile, contract_version=d.contract_version,
            harness_version="0.0.1", purpose=d.purpose))

    async def generate(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        for unit in units:
            repairing = open_on(ledger, d.name, unit)
            yield events.validate(events.CANDIDATE_GENERATED(
                candidate_id=candidate_id(d, unit), contract_id=d.contract_id(unit),
                contract_version=d.contract_version, generator_ref=f"{d.generator_ref}: {d.implementation}",
                attempt=1, candidate_sha256=d.unit_sha256(unit) if d.unit_sha256 else d.implementation_sha256,
                discrepancy_id=repairing[0] if repairing else None))

    def unit_of(cid: str) -> str:
        return next(u for u in units if candidate_id(d, u) == cid)

    async def compare(inp: dict[str, Any]) -> AsyncIterator[msgspec.Struct]:
        unit = unit_of(inp["candidate_id"])
        v = d.compare(unit)
        state["verdicts"][unit] = v
        first = f"{unit}: {v.differences[0]}" + (f" (+{len(v.differences) - 1} more)" if len(v.differences) > 1 else "") \
            if v.differences else None
        yield events.validate(events.COMPARISON_COMPLETED(
            comparison_id=f"cmp-{inp['candidate_id']}", contract_id=d.contract_id(unit),
            contract_version=d.contract_version, relation=d.relation, reference_ref=d.reference_ref,
            target_execution_id=d.target_execution_id, verdict=v.verdict, rule_ref=v.rule_ref,
            first_divergence=first[:500] if first else None))

    async def decide(inp: dict[str, Any]) -> AsyncIterator[msgspec.Struct]:
        cid = inp["comparison_id"][4:]
        unit = unit_of(cid)
        v = state["verdicts"][unit]
        if v.verdict == "conforms":
            state["to_resolve"] += len(open_on(ledger, d.name, unit))
            yield events.validate(events.CANDIDATE_ACCEPTED(
                candidate_id=cid, contract_id=d.contract_id(unit), contract_version=d.contract_version,
                comparison_ids=(inp["comparison_id"],)))
        elif v.verdict == "discrepancy":
            did = f"{run_id}-{_safe(unit)}"
            yield events.validate(events.CANDIDATE_REJECTED(
                candidate_id=cid, contract_id=d.contract_id(unit), contract_version=d.contract_version,
                reason=f"{len(v.differences)} differences on {unit}", discrepancy_id=did))
            state["reported"] += 1
            program = EMPTY
            if v.program is not None and d.programs is not None:
                program = "sha256:" + hashlib.sha256(v.program).hexdigest()
                d.programs.mkdir(parents=True, exist_ok=True)
                (d.programs / f"{program[7:]}.bin").write_bytes(v.program)
            yield events.validate(events.DISCREPANCY_REPORTED(
                discrepancy_id=did, contract_id=d.contract_id(unit), contract_version=d.contract_version,
                mismatch_class=d.relation, minimized_program_sha256=program, initial_state_sha256=EMPTY,
                first_divergence=f"{unit}: {v.differences[0]}"[:500], reference_profile_id=d.profile,
                destination_profile_id=f"w2m {d.implementation}", comparison_id=inp["comparison_id"]))
        else:
            yield events.validate(events.CANDIDATE_REJECTED(
                candidate_id=cid, contract_id=d.contract_id(unit), contract_version=d.contract_version,
                reason=f"inconclusive on {unit}: {v.differences[0] if v.differences else 'no comparison'}"[:500]))
        state["decided"] += 1

    async def route(inp: dict[str, Any]) -> AsyncIterator[msgspec.Struct]:
        unit = inp["first_divergence"].split(":", 1)[0]
        v = state["verdicts"][unit]
        yield events.validate(events.DISCREPANCY_ROUTED(
            discrepancy_id=inp["discrepancy_id"], repair_path=v.repair_path,
            reference_conforms=v.repair_path == "implementation_repair", grounds=v.grounds))
        state["routed"] += 1

    async def resolve(inp: dict[str, Any]) -> AsyncIterator[msgspec.Struct]:
        unit = unit_of(inp["candidate_id"])
        for did in open_on(ledger, d.name, unit):
            revised = ledger[did]["repair_path"] == "contract_revision"
            yield events.validate(events.DISCREPANCY_RESOLVED(
                discrepancy_id=did, resolution="contract_revised" if revised else "implementation_repaired",
                resolving_ref=inp["candidate_id"]))
            state["resolved"] += 1

    def done(_c: Any) -> bool:
        s = state
        return bool(s["decided"] == len(units) and s["routed"] == s["reported"] and s["resolved"] == s["to_resolve"])

    async def close(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        verdicts = Counter(v.verdict for v in state["verdicts"].values())
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed_with_discrepancies" if verdicts["discrepancy"] else "completed",
            discrepancy_count=verdicts["discrepancy"], accepted_count=verdicts["conforms"],
            elapsed_seconds=round(time.monotonic() - t0, 3)))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(differential=d.name, implementation=d.implementation,
                   implementation_sha256=d.implementation_sha256, reference=d.reference_ref,
                   units=len(units), **d.baseline)
        kinds: list[tuple[str, list[type[msgspec.Struct]], Callable[[Any], AsyncIterator[msgspec.Struct]]]] = [
            ("harness", [events.SESSION_INIT], harness),
            ("implementation", [events.CANDIDATE_GENERATED], generate),
            ("comparator", [events.COMPARISON_COMPLETED], compare),
            ("acceptance", [events.CANDIDATE_ACCEPTED, events.CANDIDATE_REJECTED, events.DISCREPANCY_REPORTED], decide),
            ("router", [events.DISCREPANCY_ROUTED], route),
            ("resolver", [events.DISCREPANCY_RESOLVED], resolve),
            ("harness_close", [events.SESSION_COMPLETED], close)]
        for kind, schemas, fn in kinds:
            b.producer_kind(kind, schemas=list(schemas), schema_version=1,
                            factory=(lambda fn=fn: lambda: producer(fn))())
        b.initial("harness", input=None)

        def on(kind: str) -> Subscription:
            return Subscription(kinds=frozenset({kind}))

        b.trigger("implementation-on-session-init", subscription=on("SESSION_INIT"), predicate=lambda c: True,
                  starts="implementation", input_builder=lambda c: {}, policy=PerEvent())
        b.trigger("comparator-on-candidate", subscription=on("CANDIDATE_GENERATED"), predicate=lambda c: True,
                  starts="comparator", input_builder=lambda c: {"candidate_id": c.event.payload["candidate_id"]},
                  policy=PerEvent())
        b.trigger("acceptance-on-comparison", subscription=on("COMPARISON_COMPLETED"), predicate=lambda c: True,
                  starts="acceptance", input_builder=lambda c: {"comparison_id": c.event.payload["comparison_id"]},
                  policy=PerEvent())
        b.trigger("router-on-report", subscription=on("DISCREPANCY_REPORTED"), predicate=lambda c: True,
                  starts="router", policy=PerEvent(),
                  input_builder=lambda c: {k: c.event.payload[k] for k in ("discrepancy_id", "first_divergence")})
        b.trigger("resolver-on-acceptance", subscription=on("CANDIDATE_ACCEPTED"),
                  predicate=lambda c: bool(open_on(ledger, d.name, unit_of(c.event.payload["candidate_id"]))),
                  starts="resolver", input_builder=lambda c: {"candidate_id": c.event.payload["candidate_id"]},
                  policy=PerEvent())
        b.trigger("close-on-last-step", subscription=on("substrate.ProducerCompleted"), predicate=done,
                  starts="harness_close", input_builder=lambda c: None, policy=Once())
        b.termination(threshold_count("SESSION_COMPLETED", 1))

    result = asyncio.run(Runtime(runs / run_id).run(topology))
    root = Path(result.record_root)
    kinds_seen = Counter(e["kind"] for e in read_record(root) if not e["kind"].startswith("substrate."))
    ledger_now = rebuild_ledger(runs)
    (runs / LEDGER).write_text(json.dumps(ledger_now, indent=1, sort_keys=True) + "\n")
    return {"record": str(root), "verdicts": {u: v.verdict for u, v in state["verdicts"].items()},
            "events": dict(kinds_seen),
            "resolved": state["resolved"], "open": sum(v["resolved_in"] is None for v in ledger_now.values())}
