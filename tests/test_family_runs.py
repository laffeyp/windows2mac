# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The committed family index accepts every family on its current guest file (Sprint 044), as the class
index does for every rule: an edit to a family's file after its item run leaves the acceptance stale
(heap.c was, after the environment family added w2m_heap_take)."""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import family_run  # noqa: E402


def test_the_committed_index_accepts_every_family_on_its_current_file() -> None:
    index = json.loads((ROOT / "store" / "runs" / "family-runs.json").read_text())
    assert set(family_run.FAMILIES) <= set(index)
    for name, f in family_run.FAMILIES.items():
        s = index[name]
        assert s["events"].get("CANDIDATE_ACCEPTED") == 1, name
        assert s["events"]["COMPARISON_COMPLETED"] == len(f.runs), name
        assert s["candidate_sha256"] == family_run.candidate_sha256(f), \
            f"{name}: {f.guest_file} or its host files changed since its item run"
        assert (ROOT / s["record"]).is_dir(), s["record"]


def test_every_plant_finds_its_text_once() -> None:
    """A plant whose text left its file tests nothing, and only a plant run (not a gate) used to find it: six had
    gone so when Sprint 057 ran every family's plants (their files had changed in Sprints 051-055)."""
    import importlib
    for name, f in family_run.FAMILIES.items():
        for plant, p in f.plants.items():
            path = (Path(importlib.import_module(p.host).__file__ or "") if p.host is not None
                    else family_run.GUEST / (p.file or f.guest_file))
            assert path.read_text().count(p.old) == 1, f"{name} plant {plant}: its text is not in {path.name} once"
