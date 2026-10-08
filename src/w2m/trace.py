"""trace.py — guest-trace ingest (vocabulary operator `trace_ingester`).

A guest trace is a JSONL file in the content store at `<store>/traces/<hex>.jsonl`. Only a trace
that passes every check here becomes citable evidence (TRACE_INGESTED); anything else yields
TRACE_REJECTED with the 1-based line number of the first problem. Checks: content hash,
strict per-line decode, T5 (OPENED first, CLOSED last with matching counts), S14 (nothing
after CLOSED), T6 (per-thread thread_seq 0, 1, 2, ...), and hb_refs that name existing lines.
"""

from __future__ import annotations

import hashlib
from collections.abc import AsyncIterator, Callable
from pathlib import Path
from typing import Any

import msgspec

from w2m import events

TRACES = "traces"


class _Reject(Exception):
    def __init__(self, line: int, reason: str) -> None:
        super().__init__(reason)
        self.line = line
        self.reason = reason


def trace_path(store_root: Path, trace_sha256: str) -> Path:
    return store_root / TRACES / f"{trace_sha256.removeprefix('sha256:')}.jsonl"


def _check(raw: bytes, trace_sha256: str, execution_id: str) -> tuple[int, int, str]:
    actual = "sha256:" + hashlib.sha256(raw).hexdigest()
    if actual != trace_sha256:
        raise _Reject(0, f"content hash {actual} does not match {trace_sha256}")
    lines = raw.splitlines()
    if not lines:
        raise _Reject(0, "empty trace")
    next_seq: dict[int, int] = {}
    seen: set[str] = set()
    pending_hb: list[tuple[int, str]] = []
    closed: events.GUEST_TRACE_CLOSED | None = None
    trace_version = ""
    for n, line in enumerate(lines, start=1):
        if closed is not None:
            raise _Reject(n, "line after GUEST_TRACE_CLOSED (S14)")
        try:
            ev = events.decode_guest_line(line)
        except msgspec.ValidationError as e:
            raise _Reject(n, f"invalid guest line: {e}") from None
        if n == 1:
            if not isinstance(ev, events.GUEST_TRACE_OPENED):
                raise _Reject(1, "first line is not GUEST_TRACE_OPENED (T5)")
            if ev.execution_id != execution_id:
                raise _Reject(1, f"execution_id {ev.execution_id} does not match {execution_id}")
            if ev.vocab_version not in events.GUEST_PLANE_VERSIONS:
                raise _Reject(1, f"vocab_version {ev.vocab_version} not in "
                                 f"{list(events.GUEST_PLANE_VERSIONS)}")
            trace_version = ev.vocab_version
            continue
        if isinstance(ev, events.GUEST_TRACE_OPENED):
            raise _Reject(n, "second GUEST_TRACE_OPENED (T5)")
        if isinstance(ev, events.GUEST_TRACE_CLOSED):
            closed = ev
            continue
        tid, seq = ev.thread_id, ev.thread_seq
        if isinstance(ev, events.GUEST_THREAD_STARTED):
            if tid in next_seq:
                raise _Reject(n, f"thread {tid} started twice (T6)")
            if seq != 0:
                raise _Reject(n, f"thread {tid} starts at thread_seq {seq}, not 0 (T6)")
        elif tid not in next_seq:
            raise _Reject(n, f"thread {tid} has a line before GUEST_THREAD_STARTED (T6)")
        elif seq != next_seq[tid]:
            raise _Reject(n, f"thread {tid} thread_seq {seq}, expected {next_seq[tid]} (T6)")
        next_seq[tid] = seq + 1
        seen.add(f"{tid}:{seq}")
        pending_hb.extend((n, ref) for ref in ev.hb_refs)
    if closed is None:
        raise _Reject(len(lines), "last line is not GUEST_TRACE_CLOSED (T5)")
    for n, ref in pending_hb:
        if ref not in seen:
            raise _Reject(n, f"hb_refs entry {ref} names no line in this trace")
    if closed.line_count != len(lines):
        raise _Reject(len(lines), f"line_count {closed.line_count}, file has {len(lines)}")
    if closed.thread_count != len(next_seq):
        raise _Reject(len(lines), f"thread_count {closed.thread_count}, trace has {len(next_seq)}")
    return len(lines), len(next_seq), trace_version


def ingest(
    store_root: Path, trace_sha256: str, execution_id: str
) -> events.TRACE_INGESTED | events.TRACE_REJECTED:
    """Validate one guest trace and return the validated harness event that records the outcome."""
    path = trace_path(store_root, trace_sha256)
    try:
        line_count, thread_count, version = _check(path.read_bytes(), trace_sha256, execution_id)
    except FileNotFoundError:
        return events.validate(events.TRACE_REJECTED(
            trace_sha256=trace_sha256, execution_id=execution_id, first_invalid_line=0,
            reason=f"trace not in store: {path.absolute()}"))
    except _Reject as r:
        return events.validate(events.TRACE_REJECTED(
            trace_sha256=trace_sha256, execution_id=execution_id,
            first_invalid_line=r.line, reason=r.reason))
    return events.validate(events.TRACE_INGESTED(
        trace_sha256=trace_sha256, execution_id=execution_id, line_count=line_count,
        thread_count=thread_count, vocab_version=version))


def trace_ingester(
    store_root: Path,
) -> Callable[[Any], AsyncIterator[events.TRACE_INGESTED | events.TRACE_REJECTED]]:
    """Substrate Producer: input {"trace_sha256", "execution_id"} -> one ingest outcome event."""

    async def start(inp: Any) -> AsyncIterator[events.TRACE_INGESTED | events.TRACE_REJECTED]:
        yield ingest(store_root, inp["trace_sha256"], inp["execution_id"])

    return start
