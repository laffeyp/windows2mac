# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The frontier's record names the code that ran (Sprint 060; the ground-up review of 2026-10-11): 15 of the 20
frontier records in store/runs/ named a dirty tree, and the later runs went to the ignored runs/."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import run_frontier  # noqa: E402


def test_a_run_into_the_store_from_an_uncommitted_tree_is_refused(monkeypatch: pytest.MonkeyPatch,
                                                                   capsys: pytest.CaptureFixture[str]) -> None:
    monkeypatch.setattr(run_frontier, "uncommitted", lambda: ["src/w2m/vm_backend.py"])
    monkeypatch.setattr(run_frontier, "run_with_provenance", lambda *a: pytest.fail("the run started"))
    assert run_frontier.main([str(ROOT / "pyproject.toml")]) == 2
    assert "uncommitted: src/w2m/vm_backend.py" in capsys.readouterr().out
