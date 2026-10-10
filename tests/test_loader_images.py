"""Several images (Sprint 046): a DLL the program imports is mapped, relocated, bound and started."""

from __future__ import annotations

import os
from pathlib import Path

import pytest

from w2m import loader
from w2m.pe import parse

DEPOT = Path(os.path.expanduser("~/w2m-depots/hail-to-the-rainbow"))
LAUNCHER, PLAYER = DEPOT / "Hail to the Rainbow.exe", DEPOT / "UnityPlayer.dll"
needs_depot = pytest.mark.skipif(not PLAYER.exists(), reason="the game's depot is not on this machine")


def _dlls(name: str) -> bytes | None:
    p = DEPOT / name
    return p.read_bytes() if p.is_file() else None


@needs_depot
def test_relocations_move_every_site_by_the_delta_and_nothing_else() -> None:
    """UnityPlayer.dll at its preferred base and at DLL_BASE: the images differ exactly at its DIR64
    sites (44,285), each by the delta."""
    raw = PLAYER.read_bytes()
    pe = parse(raw)
    a, b = loader.Memory(), loader.Memory()
    loader._map_image(a, raw, pe, pe.image_base, "a")
    loader._map_image(b, raw, pe, loader.DLL_BASE, "b")
    x, y = a.regions[0].data, b.regions[0].data
    delta = loader.DLL_BASE - pe.image_base
    sites = {rva for rva, kind in pe.relocation_sites if kind == 10}
    assert len(sites) == len(pe.relocation_sites) == 44_285
    for rva in sites:
        assert int.from_bytes(y[rva:rva + 8], "little") - int.from_bytes(x[rva:rva + 8], "little") == delta
    covered = bytearray(len(x))
    for rva in sites:
        covered[rva:rva + 8] = b"\1" * 8
    assert all(covered[i] or x[i] == y[i] for i in range(len(x)))


@needs_depot
def test_the_launcher_binds_unitymain_and_starts_the_dll_first() -> None:
    ld = loader.load(LAUNCHER.read_bytes(), _dlls, LAUNCHER.name)
    assert [i.name for i in ld.images] == [LAUNCHER.name, "UnityPlayer.dll"]
    player = ld.images[1]
    assert player.base == player.pe.image_base                    # preferred base, free here
    slot = next(s for s, n in ld.image.iat.items() if n == "UnityPlayer.dll!UnityMain")
    region = next(r for r in ld.memory.regions if r.name == "image")
    assert int.from_bytes(region.data[slot:slot + 8], "little") == player.export("UnityMain")
    assert ld.entry == loader.LOADER_STUB and ld.init_order == [player]
    assert all(not n.startswith("UnityPlayer.dll!") for n in ld.thunks.values())


@needs_depot
def test_a_missing_export_refuses_the_load() -> None:
    """Windows refuses to start a process whose import a DLL does not export; so does the loader."""
    broken = PLAYER.read_bytes().replace(b"UnityMain\0", b"UnityMaim\0", 1)
    with pytest.raises(loader.LoadError, match="UnityMain"):
        loader.load(LAUNCHER.read_bytes(), lambda n: broken if n == "UnityPlayer.dll" else None, LAUNCHER.name)
