# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Sprint 086: the instruction census (scripts/class_census.py).

In CI: a probe binary's census counts every instruction the walk finds and the `.pdata` sweep's too, and splits the
lifted ones that the VM hands to the model. Where the depot is on disk, the judge: the instructions the batch run
compiled (store/stencil/record.txt.gz, which the census never reads) lie in their images' reachable sets.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import class_census as cc  # noqa: E402
from w2m import x86ref  # noqa: E402
from w2m.pe import parse  # noqa: E402

PROBE = ROOT / "store" / "binaries" / "0006234b83e841523632a42e8e331b054e7d24824d38b398bf6b1628b5b4a3f6.exe"
DEPOT = Path.home() / "w2m-depots" / "hail-to-the-rainbow"


def test_the_census_holds_the_walk_and_the_sweep() -> None:
    data = PROBE.read_bytes()
    img = parse(data)
    walked = set(x86ref.walk(img, data).insns)
    raws, only, tables = cc.reachable(img, data)
    c = cc.census(PROBE)
    assert walked <= set(raws) and c["reachable_instructions"] == len(raws) and c["walked"] == len(raws) - len(only)
    assert sum(v["instances"] for v in c["by_class"].values()) + c["undecoded"] == c["reachable_instructions"]
    assert all(v["model"] <= v["lifted"] <= v["instances"] for v in c["by_class"].values())
    # Sprint 087 (large review H2): no sweep-only instruction holds a jump-table word or splits a walked one
    assert c["precision"]["holding an RVA into their function"] == 0 and c["precision"]["overlapping a walked instruction"] == 0


@pytest.mark.depot
@pytest.mark.timeout(900)                                 # every image walked and swept: about five minutes
@pytest.mark.skipif(not DEPOT.is_dir(), reason="the depot is absent")
def test_the_batch_runs_instructions_are_in_the_census() -> None:
    j = cc.judge_record(DEPOT, ROOT / "store" / "census" / "static" / "hail-to-the-rainbow.json",
                        ROOT / "store" / "stencil" / "record.txt.gz")
    share = {Path(rel).name: reach / recorded for rel, (recorded, reach, _) in j.items()}
    assert share["UnityPlayer.dll"] > 0.97 and share["mono-2.0-bdwgc.dll"] > 0.99 and share["steam_api64.dll"] > 0.99, j
