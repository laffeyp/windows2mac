"""judge_traces.py — ingest and compare a directory of downloaded probe traces on one substrate run.

Input: a `traces-<id>` artifact directory from .github/workflows/windows-probe.yml
(trace-<i>.jsonl, roles-<i>.json, probe.sha256, and exec-<i>.json from 2026-10-08 on), or
`store:<github run id>` to re-judge executions already in the committed store. Each trace is copied
into the content store with its execution metadata (store/executions/<trace hex>.json: role map,
probe sha256, run id, and exit code and elapsed time when the runner recorded them); then one
substrate topology records, per trace:
EXECUTION_STARTED, EXECUTION_COMPLETED -> TRACE_INGESTED -> COMPARISON_COMPLETED.

The harness neither starts nor times runner executions, so EXECUTION_STARTED omits
timeout_seconds and kill_grace_seconds, and EXECUTION_COMPLETED omits what the runner did not
record; `unobserved` names each omission (vocabulary 0.4, P13; TECHNIQUES.md #47).

Usage:
    uv run python scripts/judge_traces.py <dir | store:RUN_ID> --contract <id>@<version>
        [--store store] [--runs runs] [--run-id RUN_ID]
Exit 0 when every comparison conforms; 1 otherwise.
"""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
import sys
import time
from collections.abc import AsyncIterator
from pathlib import Path
from typing import Any

import msgspec
from substrate.api import (
    PerEvent, Runtime, Subscription, TopologyBuilder, narration_summary,
    quiescence, read_record,
)

from w2m import VOCAB_VERSION, events
from w2m.compare import comparator
from w2m.trace import trace_ingester, trace_path
from w2m.record import producer

EMPTY = "sha256:" + hashlib.sha256(b"").hexdigest()


def load_dir(src: Path, store: Path, run_id: str | None) -> tuple[str, list[dict[str, Any]]]:
    probe_sha = "sha256:" + (src / "probe.sha256").read_text().strip().lower()
    items = []
    for t in sorted(src.glob("trace-*.jsonl"), key=lambda p: int(p.stem.split("-")[1])):
        raw = t.read_bytes()
        digest = "sha256:" + hashlib.sha256(raw).hexdigest()
        dst = trace_path(store, digest)
        dst.parent.mkdir(parents=True, exist_ok=True)
        dst.write_bytes(raw)
        i = t.stem.split("-")[1]
        meta: dict[str, Any] = {"probe_sha256": probe_sha, "github_run_id": run_id,
                                "roles": json.loads((src / f"roles-{i}.json").read_text())}
        if (src / f"exec-{i}.json").exists():
            meta.update(json.loads((src / f"exec-{i}.json").read_text()))
        mpath = store / "executions" / f"{digest.removeprefix('sha256:')}.json"
        mpath.parent.mkdir(parents=True, exist_ok=True)
        mpath.write_text(json.dumps(meta, indent=1, sort_keys=True) + "\n")
        items.append(_item(raw, digest, meta))
    return probe_sha, items


def _item(raw: bytes, digest: str, meta: dict[str, Any]) -> dict[str, Any]:
    opened = json.loads(raw.splitlines()[0])
    return {"execution_id": opened["execution_id"], "profile_id": opened["profile_id"],
            "trace_sha256": digest, "lines": raw.count(b"\n"), "roles": meta["roles"],
            "exit_code": meta.get("exit_code"), "elapsed_seconds": meta.get("elapsed_seconds"),
            "profile_kind": meta.get("profile_kind", "windows_reference")}


def load_store(store: Path, run_id: str) -> tuple[str, list[dict[str, Any]]]:
    """Executions of one runner run, from committed store data only."""
    found = []
    for m in sorted((store / "executions").glob("*.json")):
        meta = json.loads(m.read_text())
        if str(meta.get("github_run_id")) == run_id:
            digest = "sha256:" + m.stem
            found.append((meta, _item(trace_path(store, digest).read_bytes(), digest, meta)))
    if not found:
        raise SystemExit(f"no executions for run {run_id} in {store / 'executions'}")
    probes = {meta["probe_sha256"] for meta, _ in found}
    if len(probes) != 1:
        raise SystemExit(f"run {run_id} has {len(probes)} probe hashes")
    items = sorted((it for _, it in found), key=lambda it: int(it["execution_id"].rsplit("-", 1)[1]))
    return probes.pop(), items


def topology_for(probe_sha: str, items: list[dict[str, Any]], store: Path, contracts: Path,
                 cid: str, cver: str) -> Any:
    roles = {it["execution_id"]: it["roles"] for it in items}

    t0 = time.monotonic()
    seen: list[str] = []

    async def importer(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(          # T1: first application event
            vocab_version=VOCAB_VERSION, profile=items[0]["profile_id"], contract_version=cver,
            harness_version="0.0.1", purpose="conformance"))
        for it in items:
            yield events.validate(events.EXECUTION_STARTED(
                execution_id=it["execution_id"], subject_kind="probe", subject_sha256=probe_sha,
                profile_kind=it["profile_kind"], profile_id=it["profile_id"],
                initial_state_sha256=EMPTY, injected_inputs_sha256=EMPTY,
                unobserved="timeout_seconds, kill_grace_seconds: imported execution; the "
                           "harness set no timeout"))
            missing = [f for f in ("exit_code", "elapsed_seconds") if it[f] is None]
            observed: dict[str, Any] = {f: it[f] for f in ("exit_code", "elapsed_seconds") if it[f] is not None}
            if missing:
                observed["unobserved"] = (f"{', '.join(missing)}: not recorded by the runner workflow "
                                          "before 2026-10-08 (the job fails on a non-zero exit, so an "
                                          "uploaded trace implies exit 0; inferred, not emitted)")
            yield events.validate(events.EXECUTION_COMPLETED(
                execution_id=it["execution_id"], exit_status="exited",
                trace_sha256=it["trace_sha256"], guest_line_count=it["lines"],
                **observed))

    def last_verdict(ctx: Any) -> bool:
        seen.append(ctx.event.payload["verdict"])
        return len(seen) == len(items)

    async def closer(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        bad = sum(v != "conforms" for v in seen)
        yield events.validate(events.SESSION_COMPLETED(     # T2: last application event
            outcome="completed_with_discrepancies" if bad else "completed",
            discrepancy_count=bad, accepted_count=0,
            elapsed_seconds=round(time.monotonic() - t0, 3)))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(contract=f"{cid}@{cver}", probe_sha256=probe_sha, traces=len(items))
        b.producer_kind("reference_runner", schemas=[events.SESSION_INIT, events.EXECUTION_STARTED,
                        events.EXECUTION_COMPLETED], schema_version=1, factory=lambda: producer(importer))
        b.producer_kind("harness", schemas=[events.SESSION_COMPLETED], schema_version=1,
                        factory=lambda: producer(closer))
        b.producer_kind("trace_ingester", schemas=[events.TRACE_INGESTED, events.TRACE_REJECTED],
                        schema_version=1, factory=lambda: producer(trace_ingester(store)))
        b.producer_kind("comparator", schemas=[events.COMPARISON_COMPLETED], schema_version=1,
                        factory=lambda: producer(comparator(store, contracts)))
        b.initial("reference_runner", input=None)
        b.trigger("ingest", subscription=Subscription(kinds=frozenset({"EXECUTION_COMPLETED"})),
                  predicate=lambda ctx: True, starts="trace_ingester", policy=PerEvent(),
                  input_builder=lambda ctx: {"trace_sha256": ctx.event.payload["trace_sha256"],
                                             "execution_id": ctx.event.payload["execution_id"]})
        b.trigger("compare", subscription=Subscription(kinds=frozenset({"TRACE_INGESTED"})),
                  predicate=lambda ctx: True, starts="comparator", policy=PerEvent(),
                  input_builder=lambda ctx: {
                      "contract_id": cid, "contract_version": cver,
                      "trace_sha256": ctx.event.payload["trace_sha256"],
                      "role_map": roles[ctx.event.payload["execution_id"]],
                      "comparison_id": "cmp-" + ctx.event.payload["execution_id"],
                      "reference_ref": f"{cid}@{cver}",
                      "target_execution_id": ctx.event.payload["execution_id"]})
        b.trigger("close", subscription=Subscription(kinds=frozenset({"COMPARISON_COMPLETED"})),
                  predicate=last_verdict, starts="harness", policy=PerEvent(),
                  input_builder=lambda ctx: None)
        b.termination(quiescence())

    return topology


def released(store: Path, digest: str, roles: dict[str, int]) -> str:
    who = []
    for line in trace_path(store, digest).read_bytes().splitlines():
        ev = events.decode_guest_line(line)
        if (isinstance(ev, events.GUEST_CALL_RETURNED) and ev.api == "kernel32!WaitForSingleObject"
                and ev.return_value == 0):
            who += [r for r in ("a", "b") if roles[r] == ev.thread_id]
    return ",".join(who) or "none"


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("src")
    ap.add_argument("--run-id", help="GitHub run id recorded with a directory import")
    ap.add_argument("--contract", required=True)
    ap.add_argument("--store", type=Path, default=Path("store"))
    ap.add_argument("--contracts", type=Path, default=Path("contracts"))
    ap.add_argument("--runs", type=Path, default=Path("runs"))
    a = ap.parse_args(argv)
    cid, cver = a.contract.split("@")
    if a.src.startswith("store:"):
        probe_sha, items = load_store(a.store, a.src.removeprefix("store:"))
    else:
        probe_sha, items = load_dir(Path(a.src), a.store, a.run_id)
    root = a.runs / f"judge-{time.time_ns()}"
    result = asyncio.run(Runtime(root).run(
        topology_for(probe_sha, items, a.store, a.contracts, cid, cver)))
    verdicts = {e["payload"]["target_execution_id"]: e["payload"]
                for e in read_record(result.record_root) if e["kind"] == "COMPARISON_COMPLETED"}
    ok = True
    for it in items:
        v = verdicts.get(it["execution_id"])
        verdict = v["verdict"] if v else "missing"
        ok &= verdict == "conforms"
        print(f"{it['execution_id']}  {verdict:12}  released={released(a.store, it['trace_sha256'], it['roles'])}"
              f"  {it['trace_sha256'][:23]}  {it['profile_id']}")
    s = narration_summary(result.record_root)
    print(f"record {result.record_root}: {s.total_events} events, producers {s.producers_started} "
          f"started / {s.producers_failed} failed, work {s.application_events}")
    return 0 if ok and s.producers_failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
