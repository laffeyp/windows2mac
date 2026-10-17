# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""signal_report.py (Sprint 111): a card's Emits checked against the tags a sprint's records fired."""

from __future__ import annotations

import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import signal_report  # noqa: E402

TAGS = signal_report.tag_names()


def test_named_tags_that_fired_are_counted_and_others_listed() -> None:
    fired = Counter({"EXECUTION_STARTED": 1, "MEASUREMENT_RECORDED": 107, "SESSION_COMPLETED": 1})
    lines = signal_report.emits_check("- EXECUTION_STARTED; MEASUREMENT_RECORDED, one a metric", fired, TAGS)
    assert lines == ["- `EXECUTION_STARTED`: fired 1 time", "- `MEASUREMENT_RECORDED`: fired 107 times",
                     "- fired, not named in Emits: `SESSION_COMPLETED` (1)"]


def test_planted_tag_named_and_not_fired_is_reported() -> None:
    lines = signal_report.emits_check("- TRACE_REJECTED when the trace breaks T6", Counter({"TRACE_INGESTED": 1}), TAGS)
    assert "- `TRACE_REJECTED`: NOT FIRED" in lines


def test_an_emits_of_none_with_fires_is_named_as_such() -> None:
    lines = signal_report.emits_check("- None new.", Counter({"SESSION_INIT": 2}), TAGS)
    assert lines[-1] == "- the card's Emits names no tag, and the sprint's records fired the ones above"
    assert signal_report.emits_check("- None.", Counter(), TAGS) == [
        "- the card's Emits names no tag, and the sprint made no record"]


def test_the_card_sections_are_read() -> None:
    card = "## scope\n\nOne concept.\n\nMore.\n\n## signal contract\n\n### Emits\n\n- SESSION_INIT\n\n### Invariants\n"
    assert signal_report.section(card, "### Emits") == "- SESSION_INIT"
    assert signal_report.section(card, "## scope").split("\n\n")[0] == "One concept."
