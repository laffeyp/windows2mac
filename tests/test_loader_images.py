# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Several images (Sprint 046): a DLL the program imports is mapped, relocated, bound and started.

The images are probe binaries MSVC built on the Windows runners (store/binaries), so the suite runs
wherever the repository is checked out; the game's depot is not (CI run 38048554941 failed on it)."""

from __future__ import annotations

from pathlib import Path

import pytest

from w2m import loader
from w2m.pe import parse

BINARIES = Path(__file__).resolve().parent.parent / "store" / "binaries"
# family "loader", run 38047927781: loader.exe imports loader_dll.dll, which has no relocations
LOADER_EXE = BINARIES / "2bc372b71143f8a74323f8ecac7f5a73ec622d6a329e9000647f5e779d6d8e04.exe"
LOADER_DLL = BINARIES / "6a3dd26e2da2b4f25525c3f0940e7d03b6c3b1bf3ce325808d738ce2a0c62ff7.dll"
# family "static TLS", windows-2022: static_tls_dll.dll is built on the static C runtime, 780 DIR64 sites
TLS_EXE = BINARIES / "f9d9233c10ced3784aecdeba7de1de96b7931296d3548217383c4f6529069ada.exe"
TLS_DLL = BINARIES / "3f6b51cc422038c706dda36ef9d49002c8869402d7161f6d31e8f46aa0ba1ff2.dll"


def test_relocations_move_every_site_by_the_delta_and_nothing_else() -> None:
    """static_tls_dll.dll at its preferred base and at DLL_BASE: the images differ exactly at its DIR64
    sites (780), each by the delta."""
    raw = TLS_DLL.read_bytes()
    pe = parse(raw)
    a, b = loader.Memory(), loader.Memory()
    loader.map_image(a, raw, pe, pe.image_base, "a")
    loader.map_image(b, raw, pe, loader.DLL_BASE, "b")
    x, y = a.regions[0].data, b.regions[0].data
    delta = loader.DLL_BASE - pe.image_base
    sites = {rva for rva, kind in pe.relocation_sites if kind == 10}
    assert len(sites) == len(pe.relocation_sites) == 780
    for rva in sites:
        assert int.from_bytes(y[rva:rva + 8], "little") - int.from_bytes(x[rva:rva + 8], "little") == delta
    covered = bytearray(len(x))
    for rva in sites:
        covered[rva:rva + 8] = b"\1" * 8
    assert all(covered[i] or x[i] == y[i] for i in range(len(x)))


def test_the_program_binds_the_dll_exports_and_starts_the_dll_first() -> None:
    dll = LOADER_DLL.read_bytes()
    ld = loader.load(LOADER_EXE.read_bytes(), lambda n: dll if n == "loader_dll.dll" else None, "loader.exe")
    assert [i.name for i in ld.images] == ["loader.exe", "loader_dll.dll"]
    library = ld.images[1]
    assert library.base == library.pe.image_base                  # preferred base, free here
    region = next(r for r in ld.memory.regions if r.name == "image")
    for export in ("loader_dll_call", "loader_dll_count"):
        slot = next(s for s, n in ld.image.iat.items() if n == f"loader_dll.dll!{export}")
        assert int.from_bytes(region.data[slot:slot + 8], "little") == library.export(export)
    assert ld.entry == loader.LOADER_STUB and ld.init_order == [library]
    assert all(not n.startswith("loader_dll.dll!") for n in ld.thunks.values())


def test_a_missing_export_refuses_the_load() -> None:
    """Windows refuses to start a process whose import a DLL does not export; so does the loader."""
    broken = LOADER_DLL.read_bytes().replace(b"loader_dll_call\0", b"loader_dll_cell\0", 1)
    with pytest.raises(loader.LoadError, match="loader_dll_call"):
        loader.load(LOADER_EXE.read_bytes(), lambda n: broken if n == "loader_dll.dll" else None, "loader.exe")
