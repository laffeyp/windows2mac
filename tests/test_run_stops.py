# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""A run ends with its record when asked (Sprint 065). Frontier 643a6283510c-1791820793446300000 wrote the reference's
168 log lines by 09:19 on 2026-10-12, ran two more hours toward --max-steps with no line printed, and its stop by
signal lost the record. The Machine's stop_requested (a signal) and stop_when (asked at each progress line) end the
main process's run as any other stop does."""

from __future__ import annotations

import tempfile
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
CODE_PAGES = ROOT / "store" / "binaries" / "9395509bfe645a3c33c322e88efdd81285f919b9897020d7a134db5e8cbd45bf.exe"


def stopped(requested: str | None, when: str | None, monkeypatch: pytest.MonkeyPatch) -> tuple[str, int, int]:
    from w2m import vm_backend
    from w2m.file_bridge import FileBridge
    from w2m.kernel import Machine
    from w2m.loader import load
    monkeypatch.setattr(vm_backend, "PROGRESS_SECONDS", 0)       # a progress line, and the test, at every exit
    asked = []
    with tempfile.TemporaryDirectory() as d:
        files = FileBridge(Path(d))
        vm = vm_backend.VM(load(CODE_PAGES.read_bytes(), lambda n: None, "probe.exe"), files=files,
                           image_name="probe.exe")
        try:
            machine = Machine(lambda *a: None, max_steps=1 << 40, timeout_ms=60_000)
            machine.stop_requested = requested
            machine.stop_when = lambda: asked.append(1) or when      # type: ignore[func-returns-value]
            r = machine.run(vm)
        finally:
            vm.close()
            files.close_all()
    return r.stop, r.instructions_executed, len(asked)


@pytest.mark.hvf
def test_a_requested_stop_ends_the_run_before_it_starts(monkeypatch: pytest.MonkeyPatch) -> None:
    stop, executed, _ = stopped("stopped by signal 15", None, monkeypatch)
    assert (stop, executed) == ("stopped by signal 15", 0)


@pytest.mark.hvf
def test_the_runs_test_ends_it_at_the_first_progress_line(monkeypatch: pytest.MonkeyPatch) -> None:
    stop, _, asked = stopped(None, "the log holds the reference's 168 lines", monkeypatch)
    assert (stop, asked) == ("the log holds the reference's 168 lines", 1)


@pytest.mark.hvf
def test_a_test_that_names_no_reason_leaves_the_run_to_its_end(monkeypatch: pytest.MonkeyPatch) -> None:
    stop, _, asked = stopped(None, None, monkeypatch)
    assert stop == "ExitProcess" and asked > 1


def test_a_stop_the_harness_chose_is_killed_not_faulted() -> None:
    # review of 2026-10-12, B4: frontier 643a6283510c-1791830368398119000, ended by the log rule, was recorded faulted
    import sys
    sys.path.insert(0, str(ROOT / "scripts"))
    import run_frontier
    for stop in ("stopped by signal 15", "the log holds the reference's 168 lines", "max_steps"):
        assert stop.startswith(run_frontier.HARNESS_STOPS)
    for stop in ("GuestFault: access at 0x10 outside mapped memory (x86 0x1)", "LiftError: no operand template for FLD"):
        assert not stop.startswith(run_frontier.HARNESS_STOPS)
