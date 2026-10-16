# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Sprint 085: the static census (scripts/static_census.py) against what it did not read.

The judge (in CI, from committed data): every function Hail to the Rainbow called in its Windows run is in its census,
and every name it looked up at run time is but Unity's nine native-plugin entry points, which no file on disk exports.
Where the depots are on disk: pe.parse's PE32 import tables equal Xcode's objdump's, image by image, and the census
of the depots is the committed bytes.
"""

from __future__ import annotations

import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import static_census as sc  # noqa: E402
from w2m.pe import parse  # noqa: E402

CENSUS = ROOT / "store" / "census" / "static"
UNITY_PLUGIN_HOOKS = {"UnitySetGraphicsDevice", "UnityRenderEvent", "UnityGetAudioEffectDefinitions", "UnityPluginLoad",
                      "UnityPluginUnload", "UnityRenderingExtEvent", "UnityRenderingExtQuery",
                      "UnityShaderCompilerExtEvent", "UnitySetEventQueue"}


def test_the_census_holds_every_call_the_game_made_on_windows() -> None:
    c = json.loads((CENSUS / "hail-to-the-rainbow.json").read_text())
    j = sc.judge(c)
    assert j["imports"] == 803 and j["imports missing"] == [], j["imports missing"][:10]
    assert j["dynamic"] == 331 and set(j["dynamic missing"]) <= UNITY_PLUGIN_HOOKS, j["dynamic missing"]


BIOSHOCK = sc.DEPOTS / "bioshock2-remastered"


@pytest.mark.skipif(not BIOSHOCK.is_dir() or shutil.which("objdump") is None, reason="the depot or objdump is absent")
def test_pe32_imports_equal_objdumps() -> None:
    """pe.parse(x86=True) against llvm-objdump -p, over every 32-bit image of BioShock 2 Remastered."""
    checked = 0
    for p in sorted(BIOSHOCK.rglob("*")):
        if not p.is_file() or p.suffix.lower() not in sc.SUFFIXES or sc._machine(p.read_bytes()[:4096]) != 0x14C:
            continue
        img = parse(p.read_bytes(), x86=True)
        out = subprocess.run(["objdump", "-p", str(p)], capture_output=True, text=True, check=True).stdout
        theirs: dict[str, list[str]] = {}
        dll = None
        for line in out.splitlines():
            m = re.match(r"\s+DLL Name: (\S+)", line)
            if m:
                dll = m.group(1)
                theirs.setdefault(dll, [])
            elif dll and (m := re.match(r"\s+(\d+)\s+(\S+)$", line)):
                theirs[dll].append(m.group(2))
            elif dll and not line.strip():
                dll = None
        ours = {d: [n for n in names if not n.startswith("#")] for d, names in img.imports.items()}
        assert {d: sorted(n) for d, n in ours.items() if n} == {d: sorted(n) for d, n in theirs.items() if n}, p.name
        checked += 1
    assert checked == 16


@pytest.mark.skipif(not sc.DEPOTS.is_dir(), reason="the depots are absent")
def test_the_census_reproduces_its_committed_bytes(tmp_path: Path) -> None:
    sc.main(["--out", str(tmp_path)])
    for f in sorted(CENSUS.glob("*.json")):
        assert (tmp_path / f.name).read_bytes() == f.read_bytes(), f.name
