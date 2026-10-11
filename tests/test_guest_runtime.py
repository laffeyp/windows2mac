# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Sprint 035: kernel32 runs as C inside the VM. The guest runtime image builds and names its
services; a run leaves the last error in the TEB at 0x68, logs its calls in the guest ring, and
matches the model; an import with no guest service stops the run by name; every compiled block is
discovered and published on the signal plane."""

from __future__ import annotations

import re

from collections import Counter
from pathlib import Path

import pytest
from substrate.api import read_record

from w2m import guest_runtime, vm_backend
from w2m.loader import load
from w2m.runtime import Runtime

ROOT = Path(__file__).resolve().parent.parent
SKELETON = "18f721e66de4b57e59330135d7c137c03890c94180455210cf979b8ae35074c8"


def _skeleton() -> bytes:
    return (ROOT / "store" / "binaries" / f"{SKELETON}.exe").read_bytes()


def test_api_ids_match_the_services() -> None:
    internal = set(guest_runtime.INTERNAL_APIS)                # ring entries that are not services
    other_dlls = {fn for services in vm_backend.DLL_SERVICES.values() for fn in services}   # SHELL32 (Sprint 047)
    ids = guest_runtime.api_ids()
    assert set(ids.values()) - internal == set(vm_backend.SERVICES) | other_dlls
    assert sorted(ids) == list(range(1, len(ids) + 1))        # 1 up, no gaps (api_ids.h, Sprint 049)


def test_service_table_reads_definitions_not_prototypes() -> None:
    """A prototype of another file's service (`u64 k32_X(...);`) is not a definition; W2M_VOID marks RAX left."""
    text = ("u64 k32_GetModuleFileNameW(u64 module, u64 buffer, u64 size);\n"
            "W2M_VOID u64 k32_Sleep(u64 ms) {\n    return 0;\n}\n"
            "u64 shell32_CommandLineToArgvW(u64 line,\n                               u64 count) {\n    return 0;\n}\n"
            "static u64 k32_helper(void) { return 0; }\n")
    table = guest_runtime.service_table({"x.c": text})
    assert table == {"KERNEL32": {"Sleep": ("k32_Sleep", False)},
                     "SHELL32.DLL": {"CommandLineToArgvW": ("shell32_CommandLineToArgvW", True)}}


def test_every_api_id_has_a_call_counter() -> None:
    """runtime.c counts a call only when its id is below W2M_RING_APIS; ids 64-68 went uncounted until
    Sprint 044 raised the bound (LCMapStringEx, the environment functions, SetUnhandledExceptionFilter)."""
    header = (ROOT / "tools" / "guest_runtime" / "runtime.h").read_text()
    bound = int(re.search(r"#define W2M_RING_APIS (\d+)", header).group(1))   # type: ignore[union-attr]
    assert bound == vm_backend.RING_APIS
    assert max(guest_runtime.api_ids()) < bound


@pytest.mark.hvf
def test_image_links_at_its_address_with_every_service() -> None:
    base = vm_backend.MONITOR_VA + vm_backend.RUNTIME_IMAGE
    im = guest_runtime.build(base, vm_backend.runtime_defines())
    assert im.segments[0][0] == base
    assert {sym for sym, _ in vm_backend.SERVICES.values()} <= set(im.symbols)


@pytest.mark.hvf
def test_kernel32_in_the_guest_matches_the_model_and_keeps_state_in_the_teb() -> None:
    vm = vm_backend.VM(load(_skeleton()))
    try:
        r = vm.run()
        m = Runtime(load(_skeleton())).run()
        assert (r.stop, r.exit_code) == ("ExitProcess", 0)
        assert r.calls == m.calls and r.stdout == m.stdout
        assert vm.teb_u32(0x68) == 1234                       # SetLastError(1234) wrote TEB+0x68
        assert vm.teb_u32(0x48) == 4                          # ClientId.UniqueThread
    finally:
        vm.close()


@pytest.mark.hvf
def test_import_without_a_guest_service_stops_by_name(monkeypatch: pytest.MonkeyPatch) -> None:
    services = dict(vm_backend.SERVICES)
    del services["GetStdHandle"]
    monkeypatch.setattr(vm_backend, "SERVICES", services)
    vm = vm_backend.VM(load(_skeleton()))
    try:
        r = vm.run()
        assert r.exit_code is None and r.stop == "Unimplemented: import KERNEL32.dll!GetStdHandle"
    finally:
        vm.close()


@pytest.mark.hvf
def test_every_compiled_block_is_discovered_and_published(tmp_path: Path) -> None:
    vm = vm_backend.VM(load(_skeleton()))
    try:
        vm.run()
        rec = vm_backend.record_translations(vm, f"sha256:{SKELETON}", tmp_path, "w2m-vm-test")
        evs = [(e["kind"], e["payload"]) for e in read_record(rec) if not e["kind"].startswith("substrate.")]
        kinds = Counter(k for k, _ in evs)
        assert kinds["CODE_TARGET_DISCOVERED"] == kinds["TRANSLATION_PUBLISHED"] == len(vm.translations) > 0
        discovered = [p for k, p in evs if k == "CODE_TARGET_DISCOVERED"]
        assert discovered[0]["guest_address"] == vm.ld.entry and discovered[0]["discovery"] == "static"
        assert {p["discovery"] for p in discovered[1:]} == {"runtime_transfer"}
        for (k1, d), (k2, t) in zip(evs[1:-1:2], evs[2:-1:2], strict=True):
            assert (k1, k2) == ("CODE_TARGET_DISCOVERED", "TRANSLATION_PUBLISHED")
            assert t["guest_range_start"] == d["guest_address"] < t["guest_range_end"]
    finally:
        vm.close()
