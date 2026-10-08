"""judge_traces.py — ingest and compare a directory of downloaded probe traces on one substrate run.

Input directory (a `traces-<id>` artifact from .github/workflows/windows-probe.yml):
    trace-<i>.jsonl, roles-<i>.json, probe.sha256
Each trace is copied into the content store, then one substrate topology records, per trace:
EXECUTION_STARTED, EXECUTION_COMPLETED -> TRACE_INGESTED -> COMPARISON_COMPLETED.

Usage:
    uv run python scripts/judge_traces.py <dir> --contract <id>@<version> [--store store] [--runs runs]
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
    quiescence_with_watchdog, read_record,
)

from w2m import events
from w2m.compare import comparator
from w2m.trace import trace_ingester, trace_path

EMPTY = "sha256:" + hashlib.sha256(b"").hexdigest()


def load_dir(src: Path, store: Path) -> tuple[str, list[dict[str, Any]]]:
    probe_sha = "sha256:" + (src / "probe.sha256").read_text().strip().lower()
    items = []
    for t in sorted(src.glob("trace-*.jsonl"), key=lambda p: int(p.stem.split("-")[1])):
        raw = t.read_bytes()
        digest = "sha256:" + hashlib.sha256(raw).hexdigest()
        dst = trace_path(store, digest)
        dst.parent.mkdir(parents=True, exist_ok=True)
        dst.write_bytes(raw)
        opened = json.loads(raw.splitlines()[0])
        roles = json.loads((src / f"roles-{t.stem.split('-')[1]}.json").read_text())
        items.append({"execution_id": opened["execution_id"], "profile_id": opened["profile_id"],
                      "trace_sha256": digest, "lines": raw.count(b"\n"), "roles": roles})
    return probe_sha, items


def topology_for(probe_sha: str, items: list[dict[str, Any]], store: Path, contracts: Path,
                 cid: str, cver: str) -> Any:
    roles = {it["execution_id"]: it["roles"] for it in items}

    async def importer(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        for it in items:
            yield events.validate(events.EXECUTION_STARTED(
                execution_id=it["execution_id"], subject_kind="probe", subject_sha256=probe_sha,
                profile_kind="windows_reference", profile_id=it["profile_id"],
                initial_state_sha256=EMPTY, injected_inputs_sha256=EMPTY,
                timeout_seconds=600.0, kill_grace_seconds=0.0))
            yield events.validate(events.EXECUTION_COMPLETED(
                execution_id=it["execution_id"], exit_status="exited", exit_code=0,
                trace_sha256=it["trace_sha256"], guest_line_count=it["lines"],
                elapsed_seconds=0.0))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(contract=f"{cid}@{cver}", probe_sha256=probe_sha, traces=len(items))
        b.producer_kind("reference_runner", schemas=[events.EXECUTION_STARTED,
                        events.EXECUTION_COMPLETED], schema_version=1, factory=lambda: importer)
        b.producer_kind("trace_ingester", schemas=[events.TRACE_INGESTED, events.TRACE_REJECTED],
                        schema_version=1, factory=lambda: trace_ingester(store))
        b.producer_kind("comparator", schemas=[events.COMPARISON_COMPLETED], schema_version=1,
                        factory=lambda: comparator(store, contracts))
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
        b.termination(quiescence_with_watchdog(seconds=2))

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
    ap.add_argument("src", type=Path)
    ap.add_argument("--contract", required=True)
    ap.add_argument("--store", type=Path, default=Path("store"))
    ap.add_argument("--contracts", type=Path, default=Path("contracts"))
    ap.add_argument("--runs", type=Path, default=Path("runs"))
    a = ap.parse_args(argv)
    cid, cver = a.contract.split("@")
    probe_sha, items = load_dir(a.src, a.store)
    root = a.runs / f"judge-{int(time.time())}"
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
