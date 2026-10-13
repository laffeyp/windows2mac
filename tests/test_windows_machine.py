# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The laptop's controller (Sprint 067): a run fails when its bundle did not come back, when run-job failed, or when
it holds fewer executions than asked for (review of 2026-10-13: three runs came back with a binary and a profile and
no exec-*.json, and passed); store_probe_runs.py names a laptop run's machine `laptop`."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import store_probe_runs  # noqa: E402
import windows_machine  # noqa: E402


def bundle(tmp: Path, executions: int) -> Path:
    for i in range(1, executions + 1):
        (tmp / f"exec-{i}.json").write_text('{"exit_code": 0}')
    return tmp


def test_a_full_bundle_passes(tmp_path: Path) -> None:
    assert windows_machine.verdict(0, True, bundle(tmp_path, 2), 2) == (0, "2 of 2 executions")


def test_a_short_bundle_fails_though_run_job_exited_0(tmp_path: Path) -> None:
    assert windows_machine.verdict(0, True, bundle(tmp_path, 0), 2)[0] == 1
    assert windows_machine.verdict(0, True, bundle(tmp_path, 1), 2)[0] == 1


def test_run_jobs_failure_is_the_runs(tmp_path: Path) -> None:
    assert windows_machine.verdict(3, True, bundle(tmp_path, 2), 2)[0] == 3


def test_a_bundle_not_copied_back_fails_and_says_the_laptop_kept_it(tmp_path: Path) -> None:
    status, line = windows_machine.verdict(0, False, tmp_path, 2)
    assert status == 1 and "kept" in line


def test_the_machine_is_read_from_profile_txt() -> None:
    assert store_probe_runs.image("laptop-x64-10.0.26200.7840-Client") == "laptop"
    assert store_probe_runs.image("gha-windows-2025-x64-10.0.26100.33438-Server") == "windows-2025"
    assert store_probe_runs.image("gha-windows-2022-x64-10.0.20348.4171-Server") == "windows-2022"
