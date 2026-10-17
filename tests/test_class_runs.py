# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Sprint 041 grading surface: a class is accepted by its own item run.

Every iclass with a rule is either judged by an item run on Sprint 039's cases or named as judged
elsewhere (CPUID: the guest profile, tests/test_vm_state.py). An item run's acceptance traces back
through the comparisons to the rule (substrate trace_ancestry), and a planted wrong rule is reported
and routed on its record.
"""

from __future__ import annotations

import json
import sys
from collections import Counter
from pathlib import Path

import pytest
from substrate.api import read_record, trace_ancestry

from w2m.rules import Ctx, registry

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import class_run  # noqa: E402

JUDGED_ELSEWHERE = {"CPUID", "XGETBV"}      # the guest profile, not the runner's CPU
# Classes whose discrepancy is open, each pinned to the differences its record holds (Sprint 065): FSIN and FCOS
# differ from the Xeon 8573C (and the 8370C, bit for bit the same) in the last bit of 18 of 1,598 results, each within
# 0.08 ulp of a rounding boundary. Intel documents the 66-bit pi it reduces with, not the polynomial or its internal
# precision (Intel community thread "x87 FSIN/FCOS mediocre accuracy"); rounding the reduced argument or the result to
# 66-72 bits matches fewer cases, not more. The rule stays the exact rounding of the 66-bit-pi reduction.
OPEN = {"FSIN": {"FSIN st0": 8, "FSIN FSW": 4}, "FCOS": {"FCOS st0": 10, "FCOS FSW": 4}}


def test_every_rule_has_cases_or_a_named_judge() -> None:
    assert set(registry()) - set(class_run.classes_in_case_file()) == JUDGED_ELSEWHERE


def test_the_committed_index_accepts_every_class_on_its_current_rule() -> None:
    """store/runs/class-runs.json: one accepted record per class, for the rule as it is now (a rule
    edited since its run has a different sha256 and must be run again)."""
    index = {s["iclass"]: s for s in json.loads((ROOT / "store" / "runs" / "class-runs.json").read_text())}
    assert set(index) == set(class_run.classes_in_case_file())
    for ic, s in index.items():
        if ic in OPEN:
            assert s["verdicts"] == {"model": "discrepancy", "managed_vm": "discrepancy"}, ic
            assert s["rule_sha256"] == registry()[ic].sha256, f"{ic}: rule changed since its item run"
            continue
        assert s["verdicts"] == {"model": "conforms", "managed_vm": "conforms"}, ic
        assert s["rule_sha256"] == registry()[ic].sha256, f"{ic}: rule changed since its item run"
        assert (ROOT / s["record"]).exists(), s["record"]
        kinds = Counter(e["kind"] for e in read_record(ROOT / s["record"]))
        # one decision per candidate (Sprint 042: the decide trigger fired once per comparison before Once())
        assert kinds["CANDIDATE_ACCEPTED"] == 1 and kinds["SESSION_COMPLETED"] == 1, (ic, dict(kinds))


@pytest.mark.hvf
def test_acceptance_traces_back_to_the_rule(tmp_path: Path) -> None:
    s = class_run.item_run("ADD", tmp_path)
    evs = list(read_record(s["record"]))
    acc = next(e for e in evs if e["kind"] == "CANDIDATE_ACCEPTED")
    chain = [x.kind for x in trace_ancestry(s["record"], acc["producer"]["instance"])]
    assert chain[0] == "acceptance" and chain[-1] == "harness" and "rule_source" in chain
    assert chain[1] in ("model_judge", "vm_judge")
    gen = next(e for e in evs if e["kind"] == "CANDIDATE_GENERATED")
    assert gen["payload"]["candidate_sha256"] == registry()["ADD"].sha256


@pytest.mark.hvf
def test_planted_wrong_rule_is_reported_and_routed(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """ADD's rule computing SUB: both executors differ from the CPU; the run reports and routes."""
    from w2m.rules import _int

    def add_as_sub(ctx: Ctx) -> None:
        ctx.ic = "SUB"
        _int.alu(ctx)
    real = registry()["ADD"]
    monkeypatch.setitem(registry(), "ADD", type(real)(real.module, real.path, add_as_sub, real.lower, real.shared))
    s = class_run.item_run("ADD", tmp_path)
    kinds = [e["kind"] for e in read_record(s["record"])]
    assert s["verdicts"] == {"model": "discrepancy", "managed_vm": "discrepancy"}
    assert "CANDIDATE_ACCEPTED" not in kinds
    assert kinds.count("DISCREPANCY_REPORTED") == kinds.count("DISCREPANCY_ROUTED") > 0
    assert kinds.count("SESSION_COMPLETED") == 1


def test_every_rule_writes_each_temporary_once() -> None:
    """The VM gives each write of a temporary its own register before reading the operands, so a
    rule that writes a temporary twice reads garbage there (Sprint 044: BT's first draft, caught by
    the CPU in 200 cases). Checked on every encoding of every committed case file."""
    from w2m.lift import lift
    from w2m.vm_backend import _temps_written
    import semantics_differential as sd
    twice: Counter[str] = Counter()
    codes = {c.code for k in sd.CASE_SETS for c in sd.read_cases(sd.case_file(k))}   # each encoding once
    for code in codes:
        ins = lift(code + b"\x90" * 15, sd.CODE_VA)
        written = Counter(t for op in ins.ops for t in _temps_written(op))
        if any(n > 1 for n in written.values()):
            twice[ins.iclass] += 1
    assert twice == Counter()


def test_every_index_names_its_records_relative_to_the_repository() -> None:
    """An absolute record path exists on the machine that wrote it and nowhere else: CI run 38087819989 failed on seven
    class entries written with an absolute --runs (Sprint 047)."""
    for name in ("class-runs.json", "family-runs.json", "file-runs.json"):
        index = json.loads((ROOT / "store" / "runs" / name).read_text())
        entries = index if isinstance(index, list) else list(index.values())
        assert [s["record"] for s in entries if Path(s["record"]).is_absolute()] == [], name


@pytest.mark.parametrize("ic", sorted(OPEN))
def test_an_open_class_differs_exactly_as_pinned(ic: str) -> None:
    """The model's differences on the open class are those OPEN names, no more and no fewer: a change either way
    shows here before it reaches the index."""
    _, cases, results = class_run.cases_for(ic)
    assert dict(class_run.judge_model(cases, results)[0]) == OPEN[ic]
