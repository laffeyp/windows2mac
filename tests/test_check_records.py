# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""check_records.py (Sprint 109): the vocabulary's rules over a record's application events. A clean frontier-shaped
record passes; each plant breaks one rule of each kind the checker reads, and the checker names that rule."""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import check_records  # noqa: E402

SHA = "sha256:" + "ab" * 32


def ev(seq: int, kind: str, t: float = 0.0, **payload: Any) -> dict[str, Any]:
    return {"seq": seq, "kind": kind, "t": t, "payload": payload}


def clean() -> list[dict[str, Any]]:
    return [ev(0, "SESSION_INIT"),
            ev(1, "EXECUTION_STARTED", 0.0, execution_id="x", timeout_seconds=10, kill_grace_seconds=1),
            ev(2, "EXECUTION_COMPLETED", 5.0, execution_id="x", trace_sha256=SHA),
            ev(3, "TRACE_INGESTED", trace_sha256=SHA),
            ev(4, "COMPARISON_COMPLETED", comparison_id="c", verdict="discrepancy", first_divergence="seq 3"),
            ev(5, "REDUCTION_COMPLETED", comparison_id="c", source_program_sha256=SHA, minimized_program_sha256=SHA),
            ev(6, "DISCREPANCY_REPORTED", comparison_id="c", discrepancy_id="d", minimized_program_sha256=SHA),
            ev(7, "DISCREPANCY_ROUTED", discrepancy_id="d"),
            ev(8, "SESSION_COMPLETED")]


def rules(events: list[dict[str, Any]]) -> set[str]:
    found, _, _ = check_records.Checker(check_records.vocabulary()).check(events)
    return {rule for rule, _ in found}


def test_a_clean_record_passes() -> None:
    assert rules(clean()) == set()


def drop(events: list[dict[str, Any]], kind: str) -> list[dict[str, Any]]:
    return [e for e in events if e["kind"] != kind]


@pytest.mark.parametrize("plant, rule", [
    ("SESSION_INIT second", "T1"),
    ("no SESSION_COMPLETED", "T2"),
    ("completed past the timeout", "T3"),
    ("no TRACE_INGESTED", "T4"),
    ("report with no reduction", "S5"),
    ("a short sha256", "E1"),
    ("the zero placeholder", "E1"),
    ("a discrepancy with no first divergence", "E6"),
])
def test_each_planted_violation_is_named(plant: str, rule: str) -> None:
    events = clean()
    if plant == "SESSION_INIT second":
        events[0], events[1] = events[1], events[0]
    elif plant == "no SESSION_COMPLETED":
        events = events[:-1]
    elif plant == "completed past the timeout":
        events[2]["t"] = 12.0
    elif plant == "no TRACE_INGESTED":
        events = drop(events, "TRACE_INGESTED")
    elif plant == "report with no reduction":
        events = drop(events, "REDUCTION_COMPLETED")
    elif plant == "a short sha256":
        events[3]["payload"]["trace_sha256"] = SHA[:-1]
    elif plant == "the zero placeholder":
        events[6]["payload"]["minimized_program_sha256"] = check_records.PLACEHOLDER
    elif plant == "a discrepancy with no first divergence":
        events[4]["payload"]["first_divergence"] = None
    assert rule in rules(events)


def test_no_timeout_leaves_t3_unchecked() -> None:
    events = clean()
    del events[1]["payload"]["timeout_seconds"]
    _, applied, unchecked = check_records.Checker(check_records.vocabulary()).check(events)
    assert applied["T3"] == 1 and unchecked["T3"] == 1
