"""Sprint 005 grading surface: allowed_outcome_set comparison, standalone and on a record."""

from __future__ import annotations

import hashlib
import json
from collections.abc import AsyncIterator
from pathlib import Path
from typing import Any

import msgspec
import pytest
from substrate.api import (
    PerEvent, Runtime, Subscription, TopologyBuilder, assert_event, assert_no_event,
    quiescence_with_watchdog, read_record,
)

from w2m import events
from w2m.compare import compare_allowed_outcome_set, comparator
from w2m.contract import load_contract
from w2m.trace import trace_ingester, trace_path

ROOT = Path(__file__).resolve().parent.parent
CONTRACTS = ROOT / "contracts"
CID, CVER = "windows.event.auto_reset.single_signal", "0.1.0"
ROLES = {"main": 1, "a": 2, "b": 3}
H = "sha256:" + "ef" * 32
WAIT, OK, TIMEOUT = "kernel32!WaitForSingleObject", 0, 258


def event_probe_trace(a_ret: int | None, b_ret: int | None, post: int | None = TIMEOUT) -> bytes:
    """Lines for the contract's scenario. None drops that wait call from the trace."""
    lines: list[dict[str, Any]] = [
        {"tag": "GUEST_TRACE_OPENED", "execution_id": "e1", "vocab_version": "0.1",
         "profile_id": "win11-x64-ref", "trace_format_version": "0.1"}]
    seq = {1: 0, 2: 0, 3: 0}

    def add(tid: int, tag: str, hb: list[str], **kv: Any) -> str:
        lines.append({"tag": tag, "thread_id": tid, "thread_seq": seq[tid], "hb_refs": hb, **kv})
        seq[tid] += 1
        return f"{tid}:{seq[tid] - 1}"

    def call(tid: int, api: str, ret: int, hb: list[str]) -> str:
        return add(tid, "GUEST_CALL_RETURNED", hb, api=api, args_sha256=H, return_value=ret,
                   last_error=0)

    add(1, "GUEST_THREAD_STARTED", [], parent_thread_id=0, start_address=0x1000, suspended=False)
    add(2, "GUEST_THREAD_STARTED", ["1:0"], parent_thread_id=1, start_address=0x2000,
        suspended=False)
    add(3, "GUEST_THREAD_STARTED", ["1:0"], parent_thread_id=1, start_address=0x3000,
        suspended=False)
    signal = call(1, "kernel32!SetEvent", 1, [])
    for tid, ret in ((2, a_ret), (3, b_ret)):
        if ret is not None:
            call(tid, WAIT, ret, [signal] if ret == OK else [])
    if post is not None:
        call(1, WAIT, post, [])
    lines.append({"tag": "GUEST_TRACE_CLOSED", "line_count": len(lines) + 1, "thread_count": 3,
                  "dropped_count": 0})
    return "".join(json.dumps(x) + "\n" for x in lines).encode()


def judge(raw: bytes) -> events.COMPARISON_COMPLETED:
    c = load_contract(CONTRACTS / CID / f"{CVER}.json")
    return compare_allowed_outcome_set(c, raw, ROLES, "cmp1", "ref-e0", "e1")


@pytest.mark.parametrize(("a", "b", "post", "verdict", "needle"), [
    (OK, TIMEOUT, TIMEOUT, "conforms", None),
    (TIMEOUT, OK, TIMEOUT, "conforms", None),
    (OK, OK, TIMEOUT, "discrepancy", "both_waits_complete_from_the_single_signal"),
    (TIMEOUT, TIMEOUT, TIMEOUT, "discrepancy", "no_wait_completes_from_the_signal"),
    (OK, TIMEOUT, OK, "discrepancy", "postcondition unobserved"),
    (OK, TIMEOUT, None, "discrepancy", "postcondition unobserved"),
    (OK, None, TIMEOUT, "inconclusive", None),
])
def test_verdicts(a: int | None, b: int | None, post: int | None, verdict: str,
                  needle: str | None) -> None:
    out = judge(event_probe_trace(a, b, post))
    assert out.verdict == verdict
    if needle is None:
        assert out.first_divergence is None          # E6
    else:
        assert out.first_divergence is not None and needle in out.first_divergence


def test_unmapped_role_is_inconclusive() -> None:
    c = load_contract(CONTRACTS / CID / f"{CVER}.json")
    out = compare_allowed_outcome_set(c, event_probe_trace(OK, TIMEOUT), {"a": 2, "b": 3},
                                      "cmp1", "ref-e0", "e1")
    assert out.verdict == "inconclusive"


async def _run(tmp_path: Path, raw: bytes) -> Path:
    store = tmp_path / "store"
    digest = "sha256:" + hashlib.sha256(raw).hexdigest()
    p = trace_path(store, digest)
    p.parent.mkdir(parents=True)
    p.write_bytes(raw)

    async def runner(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.EXECUTION_COMPLETED(
            execution_id="e1", exit_status="exited", exit_code=0, trace_sha256=digest,
            guest_line_count=raw.count(b"\n"), elapsed_seconds=0.1))

    def topology(b: TopologyBuilder) -> None:
        b.producer_kind("destination_runner", schemas=[events.EXECUTION_COMPLETED],
                        schema_version=1, factory=lambda: runner)
        b.producer_kind("trace_ingester", schemas=[events.TRACE_INGESTED, events.TRACE_REJECTED],
                        schema_version=1, factory=lambda: trace_ingester(store))
        b.producer_kind("comparator", schemas=[events.COMPARISON_COMPLETED], schema_version=1,
                        factory=lambda: comparator(store, CONTRACTS))
        b.initial("destination_runner", input=None)
        b.trigger("ingest", subscription=Subscription(kinds=frozenset({"EXECUTION_COMPLETED"})),
                  predicate=lambda ctx: True, starts="trace_ingester", policy=PerEvent(),
                  input_builder=lambda ctx: {"trace_sha256": ctx.event.payload["trace_sha256"],
                                             "execution_id": ctx.event.payload["execution_id"]})
        b.trigger("compare", subscription=Subscription(kinds=frozenset({"TRACE_INGESTED"})),
                  predicate=lambda ctx: True, starts="comparator", policy=PerEvent(),
                  input_builder=lambda ctx: {
                      "contract_id": CID, "contract_version": CVER,
                      "trace_sha256": ctx.event.payload["trace_sha256"], "role_map": ROLES,
                      "comparison_id": "cmp1", "reference_ref": "ref-e0",
                      "target_execution_id": ctx.event.payload["execution_id"]})
        b.termination(quiescence_with_watchdog(seconds=1))

    result = await Runtime(tmp_path / "run").run(topology)
    assert result.status == "finalised"
    rec = Path(result.record_root)
    assert_no_event(rec, "substrate.ProducerFailed")
    assert_no_event(rec, "substrate.ProducerEmittedInvalidEvent")
    kinds = [e["kind"] for e in read_record(rec) if not e["kind"].startswith("substrate.")]
    assert kinds == ["EXECUTION_COMPLETED", "TRACE_INGESTED", "COMPARISON_COMPLETED"]
    return rec


async def test_observation_conforming_run(tmp_path: Path) -> None:
    rec = await _run(tmp_path, event_probe_trace(TIMEOUT, OK))
    assert_event(rec, "COMPARISON_COMPLETED", verdict="conforms", contract_id=CID,
                 relation="allowed_outcome_set")


async def test_observation_forbidden_run(tmp_path: Path) -> None:
    rec = await _run(tmp_path, event_probe_trace(OK, OK))
    hit = assert_event(rec, "COMPARISON_COMPLETED", verdict="discrepancy")
    assert "both_waits_complete_from_the_single_signal" in hit["payload"]["first_divergence"]


# Sprint 028: the pending-waiters precondition must be witnessed in the trace.
STORE = Path(__file__).resolve().parent.parent / "store"


def _witnessed_trace() -> tuple[bytes, dict[str, int]]:
    for m in sorted((STORE / "executions").glob("*.json")):
        meta = json.loads(m.read_text())
        if meta.get("github_run_id") == "37854891607":
            return (STORE / "traces" / f"{m.stem}.jsonl").read_bytes(), meta["roles"]
    pytest.skip("witnessed runner traces not in store")


def _judge(raw: bytes, roles: dict[str, int], version: str) -> events.COMPARISON_COMPLETED:
    c = load_contract(CONTRACTS / CID / f"{version}.json")
    return compare_allowed_outcome_set(c, raw, roles, "cmp", "ref", "exec")


def test_witnessed_run_conforms() -> None:
    raw, roles = _witnessed_trace()
    assert _judge(raw, roles, "0.3.1").verdict == "conforms"


def test_missing_witness_is_inconclusive() -> None:
    raw, roles = _witnessed_trace()
    stripped = b"\n".join(x for x in raw.splitlines() if b"GUEST_THREAD_STATE_OBSERVED" not in x)
    v = _judge(stripped + b"\n", roles, "0.3.1")
    assert v.verdict == "inconclusive" and "precondition unwitnessed" in (v.first_divergence or "")


def test_witness_after_setevent_is_inconclusive() -> None:
    raw, roles = _witnessed_trace()
    late = raw.replace(b'"thread_seq":1,"hb_refs":[],"subject', b'"thread_seq":99,"hb_refs":[],"subject')
    late = late.replace(b'"thread_seq":2,"hb_refs":[],"subject', b'"thread_seq":98,"hb_refs":[],"subject')
    assert _judge(late, roles, "0.3.1").verdict == "inconclusive"


def test_wrong_state_is_inconclusive() -> None:
    raw, roles = _witnessed_trace()
    running = raw.replace(b'"thread_state":5', b'"thread_state":2', 1)      # Running, not Wait
    assert _judge(running, roles, "0.3.1").verdict == "inconclusive"


def test_unwitnessed_old_run_conforms_under_0_2_1_only() -> None:
    old = next(m for m in sorted((STORE / "executions").glob("*.json"))
               if json.loads(m.read_text()).get("github_run_id") == "37738035459")
    raw = (STORE / "traces" / f"{old.stem}.jsonl").read_bytes()
    roles = json.loads(old.read_text())["roles"]
    assert _judge(raw, roles, "0.2.1").verdict == "conforms"
    assert _judge(raw, roles, "0.3.1").verdict == "inconclusive"
