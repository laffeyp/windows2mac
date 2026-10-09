"""Sprint 031 grading surface: the candidate loop on windows.thread.last_error.set_get@0.1.0.
A scripted Responder stands in for the model so the record's shape is checked exactly: repair
after a discrepancy, refusal of code outside the admitted language, and rejection when the
acceptance condition changes during the session."""

from __future__ import annotations

import json
import shutil
import sys
from pathlib import Path

import pytest
from substrate.api import read_record

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
from run_candidate_loop import run_loop, skeleton_probe  # noqa: E402

GOOD = '''```python
def SetLastError(thread, args):
    thread.last_error = args[0] & 0xFFFFFFFF
    return None

def GetLastError(thread, args):
    return thread.last_error
```'''
LOSES_VALUE = GOOD.replace("thread.last_error = args[0] & 0xFFFFFFFF", "pass")
IMPORTS = GOOD.replace("```python\n", "```python\nimport os\n")


class Scripted:
    """Replies in order; records each prompt it was given."""

    def __init__(self, *replies: str, side_effect: object = None) -> None:
        self.replies = list(replies)
        self.prompts: list[str] = []
        self.side_effect = side_effect

    def respond(self, prompt: str) -> str:
        self.prompts.append(prompt)
        if callable(self.side_effect):
            self.side_effect()
        return self.replies.pop(0)


@pytest.fixture
def store(tmp_path: Path) -> Path:
    """A store holding the skeleton and its committed Windows reference executions."""
    src = ROOT / "store"
    dst = tmp_path / "store"
    (dst / "binaries").mkdir(parents=True)
    (dst / "executions").mkdir()
    (dst / "traces").mkdir()
    probe = skeleton_probe(src)
    import hashlib
    sha = "sha256:" + hashlib.sha256(probe).hexdigest()
    (dst / "binaries" / f"{sha[7:]}.exe").write_bytes(probe)
    n = 0
    for m in (src / "executions").glob("*.json"):
        meta = json.loads(m.read_text())
        if meta.get("probe_sha256") == sha and meta.get("profile_kind", "windows_reference") == "windows_reference":
            shutil.copy(m, dst / "executions" / m.name)
            shutil.copy(src / "traces" / f"{m.stem}.jsonl", dst / "traces" / f"{m.stem}.jsonl")
            n += 1
    assert n == 3
    return dst


def _run(store: Path, tmp_path: Path, responder: Scripted, backend: str = "model",
         contracts: Path = ROOT / "contracts") -> list[tuple[str, dict]]:
    record = run_loop(responder, "scripted (tests/test_candidate_loop.py)",
                      contract="windows.thread.last_error.set_get@0.1.0", backend=backend,
                      runs=3, attempts=3, store=store, contracts=contracts,
                      runs_root=tmp_path / "runs", probe=skeleton_probe(store),
                      profile_id="w2m-test")
    return [(e["kind"], e["payload"]) for e in read_record(record)
            if not e["kind"].startswith("substrate.") and e["kind"] != "ModelUsage"]


def _kinds(evs: list[tuple[str, dict]], keep: set[str]) -> list[str]:
    return [k for k, _ in evs if k in keep]


LOOP = {"CANDIDATE_GENERATED", "CANDIDATE_ACCEPTED", "CANDIDATE_REJECTED",
        "ACCEPTANCE_CONDITION_ALTERED", "REDUCTION_COMPLETED", "DISCREPANCY_REPORTED",
        "DISCREPANCY_ROUTED", "DISCREPANCY_RESOLVED", "SESSION_INIT", "SESSION_COMPLETED"}


def test_repair_after_discrepancy(store: Path, tmp_path: Path) -> None:
    responder = Scripted(LOSES_VALUE, GOOD)
    evs = _run(store, tmp_path, responder)
    assert _kinds(evs, LOOP) == [
        "SESSION_INIT", "CANDIDATE_GENERATED", "CANDIDATE_REJECTED", "REDUCTION_COMPLETED",
        "DISCREPANCY_REPORTED", "DISCREPANCY_ROUTED", "CANDIDATE_GENERATED",
        "CANDIDATE_ACCEPTED", "DISCREPANCY_RESOLVED", "SESSION_COMPLETED"]
    p = dict(evs)
    routed = next(pl for k, pl in evs if k == "DISCREPANCY_ROUTED")
    assert routed["repair_path"] == "implementation_repair" and routed["reference_conforms"]
    gens = [pl for k, pl in evs if k == "CANDIDATE_GENERATED"]
    assert gens[1]["attempt"] == 2 and gens[1]["discrepancy_id"] == routed["discrepancy_id"]
    assert gens[1]["parent_candidate_id"] == gens[0]["candidate_id"]
    accepted = p["CANDIDATE_ACCEPTED"]
    conforming = {pl["comparison_id"] for k, pl in evs
                  if k == "COMPARISON_COMPLETED" and pl["verdict"] == "conforms"}
    assert len(accepted["comparison_ids"]) == 3 and set(accepted["comparison_ids"]) <= conforming
    assert p["DISCREPANCY_RESOLVED"]["resolving_ref"] == accepted["candidate_id"]
    assert p["SESSION_COMPLETED"]["outcome"] == "completed_with_discrepancies"
    assert "value lost" in responder.prompts[1]                 # the structured divergence
    assert "1234" not in responder.prompts[0]                   # the scenario value is withheld


def test_code_outside_admitted_language_never_runs(store: Path, tmp_path: Path) -> None:
    evs = _run(store, tmp_path, Scripted(IMPORTS, GOOD))
    failed = [pl for k, pl in evs if k == "EXECUTION_FAILED"]
    assert len(failed) == 1 and failed[0]["failure"] == "launch_failed"
    assert "Import is not allowed" in failed[0]["diagnostic"] or "only function" in failed[0]["diagnostic"]
    assert not any(k == "EXECUTION_COMPLETED" and pl["execution_id"] == failed[0]["execution_id"]
                   for k, pl in evs)
    assert _kinds(evs, {"CANDIDATE_REJECTED", "CANDIDATE_ACCEPTED", "DISCREPANCY_REPORTED"}) == [
        "CANDIDATE_REJECTED", "CANDIDATE_ACCEPTED"]


def test_altered_contract_rejects_every_candidate(store: Path, tmp_path: Path) -> None:
    contracts = tmp_path / "contracts"
    shutil.copytree(ROOT / "contracts", contracts)
    target = contracts / "windows.thread.last_error.set_get" / "0.1.0.json"

    def alter() -> None:
        target.write_text(target.read_text().replace('"value lost"', '"value gone"'))

    evs = _run(store, tmp_path, Scripted(GOOD, GOOD, GOOD, side_effect=alter), contracts=contracts)
    altered = [pl for k, pl in evs if k == "ACCEPTANCE_CONDITION_ALTERED"]
    assert altered and altered[0]["altered"] == "contract"
    assert altered[0]["expected_sha256"] != altered[0]["observed_sha256"]
    assert not any(k == "CANDIDATE_ACCEPTED" for k, _ in evs)
    assert evs[-1][0] == "SESSION_COMPLETED" and evs[-1][1]["outcome"] == "aborted"


@pytest.mark.hvf
def test_candidate_accepted_in_managed_vm(store: Path, tmp_path: Path) -> None:
    evs = _run(store, tmp_path, Scripted(GOOD), backend="managed_vm")
    started = [pl for k, pl in evs if k == "EXECUTION_STARTED"]
    assert {pl["profile_kind"] for pl in started} == {"managed_vm"} and len(started) == 3
    assert _kinds(evs, LOOP) == ["SESSION_INIT", "CANDIDATE_GENERATED", "CANDIDATE_ACCEPTED",
                                 "SESSION_COMPLETED"]


def test_generator_failure_closes_the_session(store: Path, tmp_path: Path) -> None:
    class Unreachable:
        def respond(self, prompt: str) -> str:
            raise RuntimeError("model unreachable")

    evs = _run(store, tmp_path, Unreachable())  # type: ignore[arg-type]
    assert [k for k, _ in evs] == ["SESSION_INIT", "SESSION_COMPLETED"]
    assert evs[-1][1]["outcome"] == "aborted"                 # T2 holds when an operator fails


def test_one_scenario_contract_accepts_a_constant(store: Path, tmp_path: Path) -> None:
    """Known defect, recorded 2026-10-08 (Sprint 031): 0.1.0 judges one scenario, SetLastError(1234)
    then GetLastError, so `return 1234` conforms. The fix is a contract version whose scenario
    the candidate cannot predict; when it lands this test must flip to CANDIDATE_REJECTED."""
    constant = "def SetLastError(thread, args):\n    return None\n\n" \
               "def GetLastError(thread, args):\n    return 1234\n"
    evs = _run(store, tmp_path, Scripted(constant))
    assert _kinds(evs, {"CANDIDATE_ACCEPTED", "CANDIDATE_REJECTED"}) == ["CANDIDATE_ACCEPTED"]
