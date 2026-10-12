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


def traced_threads(start: str | None) -> list[Any]:
    from w2m.file_bridge import FileBridge
    from w2m.kernel import Machine
    from w2m.loader import load
    from w2m.vm_backend import VM
    f = family_run.FAMILIES["threads"]
    run = next(iter(f.runs.values()))
    assert f.dll_name is not None and run.dll is not None
    exe = (ROOT / "store" / "binaries" / f"{run.binary}.exe").read_bytes()
    dll = (ROOT / "store" / "binaries" / f"{run.dll}.dll").read_bytes()
    with tempfile.TemporaryDirectory() as d:
        files = FileBridge(Path(d))
        loaded = load(exe, lambda n: dll if n.lower() == f.dll_name.lower() else None, "probe.exe")   # type: ignore[union-attr]
        vm = VM(loaded, files=files, image_name="probe.exe", call_trace=("probe.exe", f.dll_name),
                call_trace_from=start)
        try:
            Machine(lambda *a: None, max_steps=1 << 40, timeout_ms=60_000).run(vm)
            calls, _ = vm.traced_calls()
        finally:
            vm.close()
            files.close_all()
    return calls


@pytest.mark.hvf
def test_the_trace_opens_where_it_is_told_to() -> None:
    """jit_capture.c hooks the images just before UnityMain, after the DLLs' DllMain ran their C runtime's start; the
    frontier opens w2m's trace at UnityMain. Here the threads DLL's start (InitializeCriticalSectionEx, from its C
    runtime) is in a trace open from the start and absent from one opened at the program's first call into the DLL."""
    whole = traced_threads(None)
    opened = traced_threads("threads_dll.dll!threads_dll_count")
    start_calls = [c for c in whole if c[2] == "threads_dll.dll" and c[4] == "InitializeCriticalSectionEx"]
    assert start_calls
    assert not [c for c in opened if c[2] == "threads_dll.dll" and c[4] == "InitializeCriticalSectionEx"]
    assert 0 < len(opened) < len(whole)


@pytest.mark.hvf
def test_a_traced_images_dllmain_runs_with_the_trace_closed() -> None:
    """A traced image loaded after the trace opened (mono-2.0-bdwgc.dll in the game) has its DllMain run between a
    close and an open of the trace header's `on` word: on Windows its DllMain ran before the hooks."""
    import struct

    from w2m.loader import load
    from w2m.vm_backend import PROCESS_VA, TRACE_OFF, VM
    vm = VM(load(CODE_PAGES.read_bytes()), image_name="probe.exe", call_trace=("probe.exe", "fake.dll"))
    try:
        vm.modules.dlls["fake.dll"] = (0x180000000, 0x1000, {})
        traced_stub = bytes(vm.mem.read(vm._init_stub(0x180000000, 0x180000100), 0x80))
        plain_stub = bytes(vm.mem.read(vm._init_stub(0x190000000, 0x190000100), 0x80))
    finally:
        vm.close()
    header = struct.pack("<Q", PROCESS_VA + TRACE_OFF)
    close, reopen = b"\x48\xb9" + header + bytes.fromhex("48c70100000000"), b"\x48\xb9" + header + bytes.fromhex("48c70101000000")
    assert traced_stub.index(close) < traced_stub.index(b"\xff\xd0") < traced_stub.index(reopen)
    assert close not in plain_stub and reopen not in plain_stub
