"""Sprint 004 grading surface: the contract format and the first contract."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from w2m.contract import ContractError, load_contract

ROOT = Path(__file__).resolve().parent.parent
EVENT = ROOT / "contracts" / "windows.event.auto_reset.single_signal" / "0.1.0.json"


def test_first_contract_loads() -> None:
    c = load_contract(EVENT)
    assert (c.contract_id, c.version, c.status) == (
        "windows.event.auto_reset.single_signal", "0.1.0", "proposed")
    assert {tuple(s) for s in c.acceptance.allowed} == {("a",), ("b",)}
    assert [e.category for e in c.evidence] == ["documented", "documented", "hypothesis"]


def _write(tmp: Path, doc: dict[str, Any]) -> Path:
    p = tmp / "contracts" / doc["contract_id"] / f"{doc['version']}.json"
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(doc))
    return p


@pytest.mark.parametrize(("name", "edit", "needle"), [
    ("unknown field", lambda d: d.update(extra=1), "unknown field"),
    ("hypothesis without check",
     lambda d: d["evidence"][2].pop("distinguishing_check"), "distinguishing_check"),
    ("documented without quote", lambda d: d["evidence"][0].pop("quote"), "verbatim quote"),
    ("documented without date",
     lambda d: d["evidence"][0].update(source_ref="https://learn.microsoft.com/x"), "retrieved"),
    ("allowed and forbidden clash",
     lambda d: d["acceptance"]["allowed"].append(["a", "b"]), "both allowed and forbidden"),
    ("undeclared role",
     lambda d: d["acceptance"]["outcome"].update(over_roles=["a", "c"]), "undeclared"),
    ("accepted on hypothesis alone",
     lambda d: d.update(status="accepted", evidence=[d["evidence"][2]]), "documented or observed"),
])
def test_bad_contracts_are_refused(tmp_path: Path, name: str, edit: Any, needle: str) -> None:
    doc = json.loads(EVENT.read_text())
    edit(doc)
    with pytest.raises(ContractError, match=needle):
        load_contract(_write(tmp_path, doc))


def test_documented_evidence_can_carry_acceptance(tmp_path: Path) -> None:
    doc = json.loads(EVENT.read_text())
    doc["status"] = "accepted"
    assert load_contract(_write(tmp_path, doc)).status == "accepted"


def test_location_must_match_id_and_version(tmp_path: Path) -> None:
    doc = json.loads(EVENT.read_text())
    p = tmp_path / "contracts" / doc["contract_id"] / "9.9.9.json"
    p.parent.mkdir(parents=True)
    p.write_text(json.dumps(doc))
    with pytest.raises(ContractError, match="must be contracts/"):
        load_contract(p)


def test_accepted_version_with_observed_evidence() -> None:
    c = load_contract(EVENT.parent / "0.2.0.json")
    assert c.status == "accepted"
    assert sum(e.category == "observed" for e in c.evidence) == 2


def test_observed_evidence_ids_resolve_in_store() -> None:
    """Sprint 026: execution_ids checked against store traces (review 2026-10-08, 4.9)."""
    store = Path(__file__).resolve().parent.parent / "store"
    c = load_contract(EVENT.parent / "0.2.1.json", store=store)
    assert {x for e in c.evidence for x in e.execution_ids} == (
        {f"evt-{i}" for i in range(1, 11)} | {"evt24-1", "evt24-2"})


def test_observed_evidence_without_executions_refused(tmp_path: Path) -> None:
    doc = json.loads((EVENT.parent / "0.2.1.json").read_text())
    for e in doc["evidence"]:
        if e["category"] == "observed":
            e["execution_ids"] = []
            e["source_ref"] = "a run on the runner"
    with pytest.raises(ContractError, match="execution_ids"):
        load_contract(_write(tmp_path, doc))


def test_unknown_execution_id_refused(tmp_path: Path) -> None:
    doc = json.loads((EVENT.parent / "0.2.1.json").read_text())
    doc["evidence"][-1]["execution_ids"] = ["evt-999"]
    with pytest.raises(ContractError, match="evt-999"):
        load_contract(_write(tmp_path, doc), store=Path(__file__).resolve().parent.parent / "store")
