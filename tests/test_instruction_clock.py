# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The instruction clock (Sprint 065 item 10): the guest's 10 MHz counter is the instructions it ran at a set rate,
and when no thread can run it jumps to the earliest deadline, as QEMU's -icount sleep=off does. The wait_handoff probe
sleeps 200 ms at least eight times while its waiters block; under the instruction clock it gives Windows' rows, and
the clock has jumped past those sleeps."""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import family_run  # noqa: E402


@pytest.mark.hvf
def test_sleeps_pass_by_jumping_the_clock_and_the_rows_stay_windows() -> None:
    from w2m.file_bridge import FileBridge
    from w2m.kernel import Machine
    from w2m.loader import load
    from w2m.vm_backend import VM
    f = family_run.FAMILIES["wait_handoff"]
    run = next(iter(f.runs.values()))
    expected = {r["s"]: r for r in (json.loads(x) for x in
                (ROOT / "store" / "probe-runs" / next(iter(f.runs))).joinpath("body-1.jsonl").read_text().splitlines())}
    with tempfile.TemporaryDirectory() as d:
        files = FileBridge(Path(d))
        vm = VM(load((ROOT / "store" / "binaries" / f"{run.binary}.exe").read_bytes(), lambda n: None, "probe.exe"),
                files=files, image_name="probe.exe", instruction_clock=(4_000_000_000, 134362624920000000))
        try:
            r = Machine(lambda *a: None, max_steps=1 << 40, timeout_ms=60_000).run(vm)
            warp = vm._clock_warp()
        finally:
            vm.close()
            files.close_all()
    rows = {x["s"]: x for x in (json.loads(line) for line in r.stdout.decode().splitlines() if line.startswith("{"))}
    assert rows == expected
    assert warp >= 8 * 200 * 10_000            # eight 200 ms sleeps, in 100 ns ticks
