"""Sprint 002 grading surface: generated Structs ride the substrate bus under their tag names,
and malformed payloads fail at the speaker's mouth (`validate`) or at ingest (`decode_guest_line`).
"""

from __future__ import annotations

import json
from collections.abc import AsyncIterator
from pathlib import Path
from typing import Any

import msgspec
import pytest
from substrate.api import Runtime, TopologyBuilder, assert_event, assert_no_event, read_record
from substrate.api import threshold_count

from w2m import VOCAB_VERSION, events

SIGNALS = Path(__file__).resolve().parent.parent / "signals"
VOCAB = json.loads((SIGNALS / (json.loads((SIGNALS / "current.json").read_text())["version"]
                              + ".json")).read_text())
H = "sha256:" + "ab" * 32


def test_one_class_per_tag_named_as_the_tag() -> None:
    names = {cls.__name__ for cls in events.HARNESS_EVENTS + events.GUEST_EVENTS}
    assert names == {t["name"] for t in VOCAB["tags"]}
    assert len(events.HARNESS_EVENTS) == 51 and len(events.GUEST_EVENTS) == 9
    assert events.VOCAB_VERSION == VOCAB_VERSION == VOCAB["version"]


async def test_session_events_land_on_a_substrate_record(tmp_path: Path) -> None:
    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile="host-m5max", contract_version="0.0.0",
            harness_version="0.0.1", purpose="conformance"))
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed", discrepancy_count=0, accepted_count=0, elapsed_seconds=0.01))

    def topology(b: TopologyBuilder) -> None:
        b.producer_kind("harness", schemas=list(events.HARNESS_EVENTS), schema_version=1,
                        factory=lambda: harness)
        b.initial("harness", input=None)
        b.termination(threshold_count("substrate.ProducerCompleted", 1))

    result = await Runtime(tmp_path / "run").run(topology)
    assert result.status == "finalised"
    rec = result.record_root
    assert_event(rec, "SESSION_INIT", vocab_version=VOCAB_VERSION, purpose="conformance")
    assert_event(rec, "SESSION_COMPLETED", outcome="completed")
    assert_no_event(rec, "substrate.ProducerEmittedInvalidEvent")
    assert_no_event(rec, "substrate.ProducerFailed")
    kinds = [e["kind"] for e in read_record(rec) if not e["kind"].startswith("substrate.")]
    assert kinds == ["SESSION_INIT", "SESSION_COMPLETED"]


@pytest.mark.parametrize("bad", [
    {"trace_sha256": "nope"},                       # sha256 pattern
    {"exit_status": "exploded"},                    # enum
    {"guest_line_count": -1},                       # u64
])
def test_validate_rejects_bad_fields(bad: dict[str, Any]) -> None:
    good = dict(execution_id="e1", exit_status="exited", exit_code=0, trace_sha256=H,
                guest_line_count=3, elapsed_seconds=0.5)
    events.validate(events.EXECUTION_COMPLETED(**good))
    with pytest.raises(msgspec.ValidationError):
        events.validate(events.EXECUTION_COMPLETED(**{**good, **bad}))


def test_harness_integers_stay_in_substrate_canonical_range() -> None:
    # substrate's RFC 8785 encoder admits |n| <= 2^53-1; validate() must refuse first.
    ok = events.CODE_TARGET_DISCOVERED(image_sha256=H, guest_address=0x7FFF_FFFF_FFFF,
                                       discovery="static", stage="install")
    events.validate(ok)
    with pytest.raises(msgspec.ValidationError):
        events.validate(events.CODE_TARGET_DISCOVERED(
            image_sha256=H, guest_address=2**53, discovery="static", stage="install"))
    # the guest plane keeps the full u64 range (plain JSONL, never canonicalized)
    line = {"tag": "GUEST_MEMORY_OBSERVED", "thread_id": 1, "thread_seq": 1, "hb_refs": [],
            "address": 2**64 - 1, "width": 8, "value": 2**64 - 1, "access": "load"}
    assert events.decode_guest_line(json.dumps(line)).value == 2**64 - 1


def test_guest_line_decoding_is_strict() -> None:
    line = {"tag": "GUEST_CLOCK_READ", "thread_id": 4, "thread_seq": 7,
            "hb_refs": ["1:3"], "clock": "qpc", "value": 123456}
    decoded = events.decode_guest_line(json.dumps(line))
    assert isinstance(decoded, events.GUEST_CLOCK_READ) and decoded.hb_refs == ("1:3",)
    for bad in ({**line, "extra": 1}, {**line, "tag": "GUEST_UNKNOWN"}, {**line, "clock": "sundial"}):
        with pytest.raises(msgspec.ValidationError):
            events.decode_guest_line(json.dumps(bad))
