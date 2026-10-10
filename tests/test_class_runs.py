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

JUDGED_ELSEWHERE = {"CPUID"}


def test_every_rule_has_cases_or_a_named_judge() -> None:
    assert set(registry()) - set(class_run.classes_in_case_file()) == JUDGED_ELSEWHERE


def test_the_committed_index_accepts_every_class_on_its_current_rule() -> None:
    """store/runs/class-runs.json: one accepted record per class, for the rule as it is now (a rule
    edited since its run has a different sha256 and must be run again)."""
    index = {s["iclass"]: s for s in json.loads((ROOT / "store" / "runs" / "class-runs.json").read_text())}
    assert set(index) == set(class_run.classes_in_case_file())
    for ic, s in index.items():
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
    the CPU in 200 cases). Checked on every case of every case set."""
    from w2m.lift import lift
    from w2m.vm_backend import _temps_written
    import semantics_differential as sd
    twice = Counter()
    for k in sd.CASE_SETS:
        seed, classes, locked, control = sd.CASE_SETS[k]
        for c in sd.build_cases(seed, classes, locked, control)[0]:
            ins = lift(c.code + b"\x90" * 15, sd.CODE_VA)
            written = Counter(t for op in ins.ops for t in _temps_written(op))
            if any(n > 1 for n in written.values()):
                twice[ins.iclass] += 1
    assert twice == Counter()
