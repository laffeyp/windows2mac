# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PEB.Ldr grows when LoadLibrary maps a DLL (Sprint 063): EnumProcessModules reads the load-order list, and on Windows
mono-2.0-bdwgc.dll, which UnityPlayer loads after the start, is in it. The list was written once at the start; a
rewrite keeps every entry and name a program already holds at its address."""

from __future__ import annotations

import struct
import sys
import tempfile
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import family_run  # noqa: E402


def load_order(vm: object) -> list[tuple[int, int, int]]:
    """(entry address, DllBase, FullDllName.Buffer) for each InLoadOrderModuleList entry, read from the process area
    (PEB and loader data both live there; VM.mem does not map it)."""
    from w2m.vm_backend import PEB_OFF, PROCESS_VA

    def q(va: int) -> int:
        return int(struct.unpack_from("<Q", vm.mm, vm._process(va - PROCESS_VA))[0])  # type: ignore[attr-defined]

    ldr = q(PROCESS_VA + PEB_OFF + 0x18)
    head, out = ldr + 0x10, []
    e = q(head)
    while e != head:
        out.append((e, q(e + 0x30), q(e + 0x50)))
        e = q(e)
    return out


@pytest.mark.hvf
def test_a_dll_loaded_after_the_start_joins_the_list_last_and_the_rest_stay() -> None:
    from w2m.file_bridge import FileBridge
    from w2m.loader import load
    from w2m.vm_backend import VM
    f = family_run.FAMILIES["module_search"]
    run = next(iter(f.runs.values()))
    assert run.dll is not None
    exe = (ROOT / "store" / "binaries" / f"{run.binary}.exe").read_bytes()
    with tempfile.TemporaryDirectory() as d:
        (Path(d) / "module_search_dll.dll").write_bytes((ROOT / "store" / "binaries" / f"{run.dll}.dll").read_bytes())
        files = FileBridge(Path(d))
        vm = VM(load(exe, lambda n: None, "probe.exe"), files=files, image_name="probe.exe")
        try:
            before = load_order(vm)
            base = vm._load_dll("module_search_dll.dll")
            after = load_order(vm)
        finally:
            vm.close()
            files.close_all()
    assert base and len(after) == len(before) + 1
    assert after[:-1] == before                      # the same entries, bases and name buffers
    assert after[-1][1] == base
