# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""w2m records a traced image's calls as tools/jit_capture/call_trace.h records the game's on Windows (Sprint 060):
the first 32 calls of each function from each image, with the sequence number, thread, eight arguments, return and
last error; a wait the scheduler retries is recorded once, at the entry that completes."""

from __future__ import annotations

import sys
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import family_run  # noqa: E402

CODE_PAGES = ROOT / "store" / "binaries" / "9395509bfe645a3c33c322e88efdd81285f919b9897020d7a134db5e8cbd45bf.exe"


def traced(exe: bytes, images: tuple[str, ...], dll: tuple[str, bytes] | None = None) -> tuple[list[Any], int, bytes, str]:
    from w2m.file_bridge import FileBridge
    from w2m.kernel import Machine
    from w2m.loader import load
    from w2m.vm_backend import VM
    with tempfile.TemporaryDirectory() as d:
        files = FileBridge(Path(d))
        loaded = load(exe, lambda n: dll[1] if dll is not None and n.lower() == dll[0].lower() else None, "probe.exe")
        vm = VM(loaded, files=files, image_name="probe.exe", call_trace=images)
        try:
            r = Machine(lambda *a: None, max_steps=1 << 40, timeout_ms=60_000).run(vm)
            calls, dropped = vm.traced_calls()
        finally:
            vm.close()
            files.close_all()
    return calls, dropped, r.stdout, r.stop


@pytest.mark.hvf
def test_the_programs_calls_are_recorded_with_arguments_return_and_last_error() -> None:
    calls, dropped, _, stop = traced(CODE_PAGES.read_bytes(), ("probe.exe",))
    assert stop == "ExitProcess" and dropped == 0
    seqs = [c[0] for c in calls]
    assert seqs == sorted(seqs) and len(set(seqs)) == len(seqs)
    assert max(Counter((c[2], c[4]) for c in calls).values()) <= 32
    first = calls[:3]
    assert [(c[2], c[3], c[4]) for c in first] == [("probe.exe", "KERNEL32.dll", "SetLastError"),
                                                    ("probe.exe", "KERNEL32.dll", "GetACP"),
                                                    ("probe.exe", "KERNEL32.dll", "GetLastError")]
    assert first[0][5][0] == 0xDEAD and first[1][6] == 1252 and first[2][6] == 0xDEAD == first[2][7]


@pytest.mark.hvf
def test_no_image_named_no_calls_recorded() -> None:
    calls, dropped, _, _ = traced(CODE_PAGES.read_bytes(), ())
    assert calls == [] and dropped == 0


@pytest.mark.hvf
def test_a_retried_wait_is_recorded_once_with_its_final_answer_and_the_rows_are_unchanged() -> None:
    f = family_run.FAMILIES["threads"]
    run = next(iter(f.runs.values()))
    assert f.dll_name is not None and run.dll is not None
    exe = (ROOT / "store" / "binaries" / f"{run.binary}.exe").read_bytes()
    dll = (ROOT / "store" / "binaries" / f"{run.dll}.dll").read_bytes()
    calls, dropped, out, stop = traced(exe, ("probe.exe", f.dll_name), (f.dll_name, dll))
    assert stop == "ExitProcess" and dropped == 0
    assert family_run.unequal_rows(f, out) == []
    waits = [c for c in calls if c[4] == "WaitForSingleObject"]
    assert waits and all(c[6] == 0 for c in waits)               # WAIT_OBJECT_0, never a retry's interim value
    assert len({c[1] for c in calls}) > 1                        # the workers' calls carry their own thread ids
