# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Sprint 038: w2m.record.one_step_run writes a session around a single step's events, and the
record replays."""

from __future__ import annotations

from pathlib import Path

from substrate.api import assert_replayable

from w2m import events
from w2m.record import one_step_run


def test_one_step_run_frames_the_work(tmp_path: Path) -> None:
    def work():  # type: ignore[no-untyped-def]
        yield events.PROBE_AUTHORED(probe_id="p", probe_sha256="sha256:" + "0" * 64, contract_ids=(), origin="hand")
    root, kinds = one_step_run(tmp_path / "r", profile="t", contract_version="0.6", purpose="measurement",
                               work=work, baseline={"x": 1})
    assert kinds == {"SESSION_INIT": 1, "PROBE_AUTHORED": 1, "SESSION_COMPLETED": 1}
    assert_replayable(root, "2")
