# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""SetFocus while another process keeps the foreground (Sprint 065). Game run 38275716702's hook saw Windows send the
game WM_ACTIVATEAPP (1, 0) and WM_NCACTIVATE (0, 0), a Windows Terminal window the foreground throughout; the windows
probe, which takes the foreground, has the lParam not NULL and WM_NCACTIVATE 1 (its family's rows). The windows probe
run with the foreground held answers as the game run."""

from __future__ import annotations

import dataclasses
import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import family_run  # noqa: E402


@pytest.mark.hvf
def test_a_window_activated_in_the_background_has_no_deactivated_thread() -> None:
    f = family_run.FAMILIES["windows"]
    binary = next(iter(f.runs.values())).binary
    held = dataclasses.replace(f, foreground_held=True)
    out, _, _ = family_run.run_in_vm(binary, None, held)
    rows = {r["s"]: r for r in (json.loads(x) for x in out.decode().splitlines() if x.startswith("{"))}
    assert (rows["msg.game.9"]["msg"], rows["msg.game.9"]["lparam_null"]) == (0x1C, True)
    assert (rows["msg.game.13"]["msg"], rows["msg.game.13"]["wparam"]) == (0x86, 0)
    assert rows["msg.game.14"]["wparam"] == 1                     # WM_ACTIVATE: WA_ACTIVE all the same
