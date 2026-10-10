# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""record_contracts.py — the contract registry on the signal plane (Sprint 038; review 2026-10-09
progress K1).

Every version under contracts/ is replayed through the contract lifecycle, each step caused by the
one before:

  proposer  per version, in version order: EVIDENCE_RECORDED for each evidence item, then
            CONTRACT_PROPOSED (proposer agent)
  acceptor  on each proposal: the contract loader's checks (src/w2m/contract.py) run on the file;
            a version whose status is `accepted` and that loads gets CONTRACT_ACCEPTED; a proposed
            version that later versions replaced stays proposed
  reviser   on each acceptance after the first of a contract id: CONTRACT_REVISED from the previous
            accepted version, then DEPENDENTS_INVALIDATED with what claimed conformance to it
            (candidates accepted under it in store/runs; evidence the new version dropped)

Usage: uv run python scripts/record_contracts.py [--runs runs]
"""

from __future__ import annotations

import argparse
import sys
import time
from collections.abc import AsyncIterator
from pathlib import Path
from typing import Any

import msgspec
from substrate.api import PerEvent, Subscription, TopologyBuilder, read_record, threshold_count

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
from w2m import VOCAB_VERSION, events  # noqa: E402
from w2m.contract import Contract, ContractError, load_contract  # noqa: E402
from w2m.record import producer, run_with_provenance  # noqa: E402

CONTRACTS = ROOT / "contracts"
STORE_RUNS = ROOT / "store" / "runs"


def _version_key(v: str) -> tuple[int, ...]:
    return tuple(int(x) for x in v.split("."))


def accepted_candidates(contract_id: str, version: str) -> int:
    n = 0
    for d in STORE_RUNS.glob("candidates-*"):
        for e in read_record(d):
            p = e["payload"]
            if e["kind"] == "CANDIDATE_ACCEPTED" and p["contract_id"] == contract_id and p["contract_version"] == version:
                n += 1
    return n


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", type=Path, default=ROOT / "runs")
    a = ap.parse_args(argv)
    files = sorted(CONTRACTS.glob("*/*.json"), key=lambda p: (p.parent.name, _version_key(p.stem)))
    loaded: dict[tuple[str, str], Contract] = {}
    last_accepted: dict[str, str] = {}
    pending = [len(files)]
    t0 = time.monotonic()

    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile="contracts", contract_version=VOCAB_VERSION,
            harness_version="0.0.1", purpose="contract_construction"))

    async def proposer(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        for f in files:
            c = load_contract(f)
            loaded[(c.contract_id, c.version)] = c
            for ev in c.evidence:
                yield events.validate(events.EVIDENCE_RECORDED(
                    evidence_id=ev.evidence_id, evidence_category=ev.category, contract_id=c.contract_id,
                    source_ref=ev.source_ref, applicability=ev.applicability,
                    **({"distinguishing_check": ev.distinguishing_check} if ev.distinguishing_check else {})))
            yield events.validate(events.CONTRACT_PROPOSED(
                contract_id=c.contract_id, version=c.version,
                evidence_ids=tuple(ev.evidence_id for ev in c.evidence), proposer="agent"))

    async def acceptor(inp: dict[str, Any]) -> AsyncIterator[msgspec.Struct]:
        cid, ver = inp["contract_id"], inp["version"]
        path = CONTRACTS / cid / f"{ver}.json"
        try:
            c = load_contract(path)                       # the loader's checks are the acceptance gate
        except ContractError:
            return
        if c.status == "accepted":
            yield events.validate(events.CONTRACT_ACCEPTED(
                contract_id=cid, version=ver, evidence_ids=tuple(ev.evidence_id for ev in c.evidence),
                open_question_count=len(c.open_questions), applicability=c.applicability))

    async def reviser(inp: dict[str, Any]) -> AsyncIterator[msgspec.Struct]:
        cid, ver = inp["contract_id"], inp["version"]
        prev = last_accepted.get(cid)
        last_accepted[cid] = ver
        if prev is None:
            return
        old, new = loaded[(cid, prev)], loaded[(cid, ver)]
        dropped = {e.evidence_id for e in old.evidence} - {e.evidence_id for e in new.evidence}
        yield events.validate(events.CONTRACT_REVISED(
            contract_id=cid, from_version=prev, to_version=ver,
            rationale_ref=new.sections.get("supersedes") or f"contracts/{cid}/{ver}.json sections and open questions",
            evidence_ids=tuple(e.evidence_id for e in new.evidence)))
        yield events.validate(events.DEPENDENTS_INVALIDATED(
            contract_id=cid, from_version=prev, to_version=ver, artifact_count=0,
            candidate_count=accepted_candidates(cid, prev), evidence_count=len(dropped)))

    async def closer(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed", discrepancy_count=0, accepted_count=0,
            elapsed_seconds=round(time.monotonic() - t0, 3)))

    def proposals_done(c: Any) -> bool:
        pending[0] -= 1
        return pending[0] == 0

    def topology(b: TopologyBuilder) -> None:
        b.baseline(contracts=[str(f.relative_to(ROOT)) for f in files])
        b.producer_kind("harness", schemas=[events.SESSION_INIT], schema_version=1, factory=lambda: producer(harness))
        b.producer_kind("proposer", schemas=[events.EVIDENCE_RECORDED, events.CONTRACT_PROPOSED],
                        schema_version=1, factory=lambda: producer(proposer))
        b.producer_kind("acceptor", schemas=[events.CONTRACT_ACCEPTED], schema_version=1, factory=lambda: producer(acceptor))
        b.producer_kind("reviser", schemas=[events.CONTRACT_REVISED, events.DEPENDENTS_INVALIDATED],
                        schema_version=1, factory=lambda: producer(reviser))
        b.producer_kind("closer", schemas=[events.SESSION_COMPLETED], schema_version=1, factory=lambda: producer(closer))
        b.initial("harness", input=None)

        def on(kinds: set[str]) -> Subscription:
            return Subscription(kinds=frozenset(kinds))

        payload = {"contract_id", "version"}
        b.trigger("propose", subscription=on({"SESSION_INIT"}), predicate=lambda c: True, starts="proposer",
                  policy=PerEvent(), input_builder=lambda c: None)
        b.trigger("accept", subscription=on({"CONTRACT_PROPOSED"}), predicate=lambda c: True, starts="acceptor",
                  policy=PerEvent(), input_builder=lambda c: {k: c.event.payload[k] for k in payload})
        b.trigger("revise", subscription=on({"CONTRACT_ACCEPTED"}), predicate=lambda c: True, starts="reviser",
                  policy=PerEvent(), input_builder=lambda c: {k: c.event.payload[k] for k in payload})
        b.trigger("close", subscription=on({"substrate.ProducerCompleted"}),
                  predicate=lambda c: c.event.payload.get("producer", {}).get("kind") == "acceptor" and proposals_done(c),
                  starts="closer", policy=PerEvent(), input_builder=lambda c: None)
        b.termination(threshold_count("SESSION_COMPLETED", 1))

    result = run_with_provenance(a.runs / f"contracts-{time.time_ns()}", topology)
    from collections import Counter
    kinds = Counter(e["kind"] for e in read_record(result.record_root) if not e["kind"].startswith("substrate."))
    for e in read_record(result.record_root):
        if e["kind"] in ("CONTRACT_ACCEPTED", "CONTRACT_REVISED", "DEPENDENTS_INVALIDATED", "substrate.ProducerFailed"):
            print(e["kind"], {k: v for k, v in e["payload"].items() if k not in ("evidence_ids", "applicability", "rationale_ref")})
    print(dict(kinds))
    print(f"record {result.record_root}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
