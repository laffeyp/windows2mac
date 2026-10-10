"""Sprint 003 grading surface: guest-trace ingest, standalone and as a substrate Producer."""

from __future__ import annotations

import hashlib
import json
from collections.abc import AsyncIterator, Callable
from pathlib import Path
from typing import Any

import msgspec
import pytest
from substrate.api import (
    PerEvent, Runtime, Subscription, TopologyBuilder, assert_event, assert_no_event,
    quiescence_with_watchdog, read_record,
)

from w2m import events
from w2m.trace import ingest, trace_ingester, trace_path
from w2m.record import producer

H = "sha256:" + "cd" * 32


def good_lines() -> list[dict[str, Any]]:
    """Thread 2 waits on an event that thread 1 signals; the wait return cites 1:2."""
    return [
        {"tag": "GUEST_TRACE_OPENED", "execution_id": "e1", "vocab_version": "0.1",
         "profile_id": "win11-x64-ref", "trace_format_version": "0.1"},
        {"tag": "GUEST_THREAD_STARTED", "thread_id": 1, "thread_seq": 0, "hb_refs": [],
         "parent_thread_id": 0, "start_address": 0x140001000, "suspended": False},
        {"tag": "GUEST_THREAD_STARTED", "thread_id": 2, "thread_seq": 0, "hb_refs": ["1:0"],
         "parent_thread_id": 1, "start_address": 0x140002000, "suspended": False},
        {"tag": "GUEST_CLOCK_READ", "thread_id": 1, "thread_seq": 1, "hb_refs": [],
         "clock": "qpc", "value": 1000},
        {"tag": "GUEST_CALL_RETURNED", "thread_id": 1, "thread_seq": 2, "hb_refs": [],
         "api": "kernel32!SetEvent", "args_sha256": H, "return_value": 1, "last_error": 0},
        {"tag": "GUEST_CALL_RETURNED", "thread_id": 2, "thread_seq": 1, "hb_refs": ["1:2"],
         "api": "kernel32!WaitForSingleObject", "args_sha256": H, "return_value": 0,
         "last_error": 0},
        {"tag": "GUEST_THREAD_EXITED", "thread_id": 2, "thread_seq": 2, "hb_refs": [],
         "exit_code": 0},
        {"tag": "GUEST_TRACE_CLOSED", "line_count": 8, "thread_count": 2, "dropped_count": 0},
    ]


def store(root: Path, lines: list[dict[str, Any]]) -> str:
    raw = "".join(json.dumps(x) + "\n" for x in lines).encode()
    digest = "sha256:" + hashlib.sha256(raw).hexdigest()
    p = trace_path(root, digest)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_bytes(raw)
    return digest


def test_valid_trace_is_ingested(tmp_path: Path) -> None:
    out = ingest(tmp_path, store(tmp_path, good_lines()), "e1")
    assert isinstance(out, events.TRACE_INGESTED)
    assert (out.line_count, out.thread_count, out.vocab_version) == (8, 2, "0.1")


def _mutate(i: int, **kv: Any) -> Callable[[list[dict[str, Any]]], list[dict[str, Any]]]:
    def f(lines: list[dict[str, Any]]) -> list[dict[str, Any]]:
        lines[i] = {**lines[i], **kv}
        return lines
    return f


@pytest.mark.parametrize(("name", "mutate", "line", "needle"), [
    ("extra field", _mutate(3, extra=1), 4, "invalid guest line"),
    ("seq gap", _mutate(4, thread_seq=3), 5, "expected 2"),
    ("dangling hb", _mutate(5, hb_refs=["1:9"]), 6, "names no line"),
    ("wrong line_count", _mutate(7, line_count=9), 8, "line_count 9"),
    ("wrong execution", _mutate(0, execution_id="e2"), 1, "does not match"),
    ("line after close", lambda ls: ls + [ls[3]], 9, "after GUEST_TRACE_CLOSED"),
    ("not opened first", lambda ls: ls[1:], 1, "not GUEST_TRACE_OPENED"),
])
def test_malformed_traces_are_rejected_with_the_line(
    tmp_path: Path, name: str, mutate: Callable[[list[dict[str, Any]]], list[dict[str, Any]]],
    line: int, needle: str,
) -> None:
    out = ingest(tmp_path, store(tmp_path, mutate(good_lines())), "e1")
    assert isinstance(out, events.TRACE_REJECTED), name
    assert out.first_invalid_line == line, (name, out.reason)
    assert needle in out.reason, (name, out.reason)


def test_hash_mismatch_and_missing_file(tmp_path: Path) -> None:
    digest = store(tmp_path, good_lines())
    p = trace_path(tmp_path, digest)
    p.write_bytes(p.read_bytes() + b"\n")
    out = ingest(tmp_path, digest, "e1")
    assert isinstance(out, events.TRACE_REJECTED) and "content hash" in out.reason
    missing = ingest(tmp_path, H, "e1")
    assert isinstance(missing, events.TRACE_REJECTED) and "not in store" in missing.reason


async def _run(tmp_path: Path, lines: list[dict[str, Any]]) -> tuple[str, Path]:
    """Observation contract: stand-in runner -> EXECUTION_COMPLETED -> trace_ingester."""
    digest = store(tmp_path / "store", lines)

    async def runner(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.EXECUTION_STARTED(
            execution_id="e1", subject_kind="probe", subject_sha256=H,
            profile_kind="windows_reference", profile_id="win11-x64-ref",
            initial_state_sha256=H, injected_inputs_sha256=H,
            timeout_seconds=60.0, kill_grace_seconds=5.0))
        yield events.validate(events.EXECUTION_COMPLETED(
            execution_id="e1", exit_status="exited", exit_code=0, trace_sha256=digest,
            guest_line_count=len(lines), elapsed_seconds=0.2))

    def topology(b: TopologyBuilder) -> None:
        b.producer_kind("reference_runner", schemas=[events.EXECUTION_STARTED,
                        events.EXECUTION_COMPLETED], schema_version=1, factory=lambda: producer(runner))
        b.producer_kind("trace_ingester", schemas=[events.TRACE_INGESTED, events.TRACE_REJECTED],
                        schema_version=1, factory=lambda: producer(trace_ingester(tmp_path / "store")))
        b.initial("reference_runner", input=None)
        b.trigger(
            "ingest-on-complete",
            subscription=Subscription(kinds=frozenset({"EXECUTION_COMPLETED"})),
            predicate=lambda ctx: True,
            starts="trace_ingester",
            input_builder=lambda ctx: {"trace_sha256": ctx.event.payload["trace_sha256"],
                                       "execution_id": ctx.event.payload["execution_id"]},
            policy=PerEvent(),
        )
        b.termination(quiescence_with_watchdog(seconds=1))

    result = await Runtime(tmp_path / "run").run(topology)
    assert result.status == "finalised"
    return digest, Path(result.record_root)


async def test_observation_ingested_on_the_record(tmp_path: Path) -> None:
    digest, rec = await _run(tmp_path, good_lines())
    kinds = [e["kind"] for e in read_record(rec) if not e["kind"].startswith("substrate.")]
    assert kinds == ["EXECUTION_STARTED", "EXECUTION_COMPLETED", "TRACE_INGESTED"]
    assert_event(rec, "TRACE_INGESTED", trace_sha256=digest, line_count=8, thread_count=2)
    assert_no_event(rec, "substrate.ProducerFailed")
    assert_no_event(rec, "substrate.ProducerEmittedInvalidEvent")
    # replay side: every recorded payload converts back into its Struct and validates
    # (RFC 8785 writes 5.0 as 5; msgspec reads it back as a float)
    by_tag = {c.__name__: c for c in events.HARNESS_EVENTS}
    for e in read_record(rec):
        if e["kind"] in by_tag:
            events.validate(msgspec.convert(e["payload"], by_tag[e["kind"]]))


async def test_observation_rejected_on_the_record(tmp_path: Path) -> None:
    digest, rec = await _run(tmp_path, _mutate(4, thread_seq=3)(good_lines()))
    assert_event(rec, "TRACE_REJECTED", trace_sha256=digest, first_invalid_line=5)
    assert_no_event(rec, "TRACE_INGESTED")


def test_unknown_vocab_version_rejected_and_old_versions_kept(tmp_path: Path) -> None:
    lines = good_lines()
    lines[0] = {**lines[0], "vocab_version": "9.9"}
    out = ingest(tmp_path, store(tmp_path, lines), "e1")
    assert isinstance(out, events.TRACE_REJECTED) and "9.9" in out.reason
    assert "0.1" in events.GUEST_PLANE_VERSIONS          # committed 0.1 traces stay evidence


def test_every_committed_reference_trace_still_ingests() -> None:
    root = Path(__file__).resolve().parent.parent / "store"
    traces = sorted((root / "traces").glob("*.jsonl"))
    assert traces
    for t in traces:
        eid = json.loads(t.read_bytes().splitlines()[0])["execution_id"]
        out = ingest(root, "sha256:" + t.stem, eid)
        assert isinstance(out, events.TRACE_INGESTED), (t.name, out)
