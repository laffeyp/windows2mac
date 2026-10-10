"""The committed family index accepts every family on its current guest file (Sprint 044), as the class
index does for every rule: an edit to a family's file after its item run leaves the acceptance stale
(heap.c was, after the environment family added w2m_heap_take)."""

from __future__ import annotations

import hashlib
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
        sha = "sha256:" + hashlib.sha256((family_run.GUEST / f.guest_file).read_bytes()).hexdigest()
        assert s["candidate_sha256"] == sha, f"{name}: {f.guest_file} changed since its item run"
        assert (ROOT / s["record"]).is_dir(), s["record"]
