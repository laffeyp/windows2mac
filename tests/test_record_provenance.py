# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Review 2026-10-10 D2: every record names the code that wrote it. w2m records are written through
w2m.record.run_with_provenance, which puts w2m's commit and the substrate clone's into the run's
baseline (RunStarted)."""

from __future__ import annotations

import re
from pathlib import Path

from substrate.api import read_record

from w2m.record import one_step_run, provenance

ROOT = Path(__file__).resolve().parent.parent


def test_no_record_is_written_around_the_stamp() -> None:
    direct = re.compile(r"\bRuntime\([^)]*\)\.run\(")
    offenders = [str(p.relative_to(ROOT)) for d in ("scripts", "src")
                 for p in (ROOT / d).rglob("*.py")
                 if p.name != "record.py" and direct.search(p.read_text())]
    assert offenders == []


def test_a_record_carries_both_commits(tmp_path: Path) -> None:
    root, _ = one_step_run(tmp_path / "run", profile="test", contract_version="0.0.1", purpose="measurement",
                           work=lambda: [], baseline={})
    started = next(e for e in read_record(root) if e["kind"] == "substrate.RunStarted")
    base = started["payload"]["baseline"]
    assert base["w2m_commit"] == provenance()["w2m_commit"] and re.fullmatch(r"[0-9a-f]{40}(-dirty)?", base["w2m_commit"])
    assert re.fullmatch(r"[0-9a-f]{40}(-dirty)?", base["substrate_commit"])
    assert base["substrate_commit"].removesuffix("-dirty") == (ROOT / "substrate.commit").read_text().strip()


def test_a_session_that_accepts_counts_it(tmp_path: Path) -> None:
    """SESSION_COMPLETED.accepted_count counts the run's acceptances (review 2026-10-15, small review of 080:
    one_step_run wrote 0 after a CANDIDATE_ACCEPTED)."""
    from w2m import events
    accept = events.CANDIDATE_ACCEPTED(candidate_id="c", contract_id="w2m.test.accept", contract_version="0.0.1",
                                       comparison_ids=("cmp",))
    root, _ = one_step_run(tmp_path / "run", profile="test", contract_version="0.0.1", purpose="measurement",
                           work=lambda: [accept], baseline={})
    done = next(e for e in read_record(root) if e["kind"] == "SESSION_COMPLETED")
    assert done["payload"]["accepted_count"] == 1
