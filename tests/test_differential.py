# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Sprint 043 grading surface: a differential's record is causal, and a discrepancy one run reports
is resolved on the record of the run that fixed it.

The guest-state differential is the subject: a planted fault in the spec's CONTEXT.Rip offset
(reach: the CONTEXT unit, one field) makes one run reject CONTEXT, report and route; the next run,
on the true spec, is a repair attempt on CONTEXT, accepts it, and resolves the first run's
discrepancy by id.
"""

from __future__ import annotations

import argparse
import copy
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any

from substrate.api import read_first_envelope, read_record, trace_ancestry

from w2m import differential

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import check_guest_state  # noqa: E402

RUN = ROOT / "store" / "probe-runs" / "37989842529"


def _record(runs: Path, spec: dict[str, Any]) -> dict[str, Any]:
    a = argparse.Namespace(run_dir=RUN, runs=runs)
    return check_guest_state.record(a, spec, check_guest_state.runs_of(RUN),
                                    "gha-windows-2025 (test)", json.dumps(spec, sort_keys=True).encode())


def _events(rec: str) -> list[dict[str, Any]]:
    return list(read_record(rec))


def _chain(rec: Any, e: dict[str, Any]) -> list[str]:
    """`rec`: a record root, or its events already read (trace_ancestry reloads a root on every call: 308 calls
    took 22 s of this file's 23 s, 2026-10-10)."""
    return [x.kind for x in trace_ancestry(rec, e["producer"]["instance"])]


def test_a_discrepancy_is_resolved_by_the_run_that_fixed_it(tmp_path: Path) -> None:
    spec = json.loads(check_guest_state.SPEC.read_text())
    planted = copy.deepcopy(spec)
    planted["windows"]["CONTEXT"]["fields"]["Rip"] += 8

    first = _record(tmp_path, planted)
    assert first["verdicts"] == {u: ("discrepancy" if u == "CONTEXT" else "conforms") for u in check_guest_state.UNITS}
    evs = _events(first["record"])
    kinds = Counter(e["kind"] for e in evs)
    assert (kinds["CANDIDATE_GENERATED"], kinds["COMPARISON_COMPLETED"]) == (7, 7)
    assert (kinds["CANDIDATE_ACCEPTED"], kinds["CANDIDATE_REJECTED"]) == (6, 1)
    assert kinds["DISCREPANCY_REPORTED"] == kinds["DISCREPANCY_ROUTED"] == 1 and kinds["SESSION_COMPLETED"] == 1
    reported = next(e for e in evs if e["kind"] == "DISCREPANCY_REPORTED")
    did = reported["payload"]["discrepancy_id"]
    assert reported["payload"]["first_divergence"].startswith("CONTEXT: CONTEXT.Rip: spec 0x100")
    routed = next(e for e in evs if e["kind"] == "DISCREPANCY_ROUTED")
    assert routed["payload"]["repair_path"] == "contract_revision"
    assert _chain(first["record"], routed)[:3] == ["router", "acceptance", "comparator"]
    ledger = json.loads((tmp_path / differential.LEDGER).read_text())
    assert list(ledger) == [did] and ledger[did]["resolved_in"] is None

    second = _record(tmp_path, spec)
    assert set(second["verdicts"].values()) == {"conforms"}
    evs = _events(second["record"])
    repair = next(e for e in evs if e["kind"] == "CANDIDATE_GENERATED"
                  and e["payload"]["candidate_id"].startswith("guest-state:CONTEXT@"))
    assert repair["payload"]["discrepancy_id"] == did
    resolved = [e for e in evs if e["kind"] == "DISCREPANCY_RESOLVED"]
    assert len(resolved) == 1 and resolved[0]["payload"]["discrepancy_id"] == did
    assert resolved[0]["payload"]["resolution"] == "contract_revised"
    assert _chain(second["record"], resolved[0])[:4] == ["resolver", "acceptance", "comparator", "implementation"]
    ledger = json.loads((tmp_path / differential.LEDGER).read_text())
    assert ledger[did]["resolved_in"] == Path(second["record"]).name

    third = _record(tmp_path, spec)                                     # nothing open: no repair, no resolution
    kinds = Counter(e["kind"] for e in _events(third["record"]))
    assert kinds["DISCREPANCY_RESOLVED"] == 0 and kinds["CANDIDATE_ACCEPTED"] == 7


def test_every_decision_traces_to_its_candidate_and_the_session(tmp_path: Path) -> None:
    rec = _record(tmp_path, json.loads(check_guest_state.SPEC.read_text()))["record"]
    for e in _events(rec):
        if e["kind"] in ("CANDIDATE_ACCEPTED", "CANDIDATE_REJECTED"):
            chain = _chain(rec, e)
            assert chain == ["acceptance", "comparator", "implementation", "harness"], chain


STORE = ROOT / "store" / "runs"


def _differential_records() -> list[Path]:
    out = []
    for root in sorted(STORE.iterdir()):
        if root.is_dir() and not root.name.startswith(differential.NOT_DIFFERENTIAL):
            try:
                first = read_first_envelope(root)    # one CRC-checked line per record, not its whole segment
            except Exception:  # noqa: BLE001 — not a record
                continue
            if first is None:
                continue
            if (first.get("payload") or {}).get("baseline", {}).get("differential"):
                out.append(root)
    return out


def test_the_committed_ledger_is_what_the_records_say() -> None:
    assert json.loads((STORE / differential.LEDGER).read_text()) == differential.rebuild_ledger(STORE)


def test_every_stored_differential_record_is_causal_with_its_multiplicities() -> None:
    """Sprint 043 signal contract: per unit one candidate, one comparison, one decision; one route
    per report; one SESSION_INIT and one SESSION_COMPLETED; every decision caused by its comparison,
    every route by its report, every resolution by an acceptance."""
    records = _differential_records()
    assert len(records) >= 8
    for rec in records:
        evs = list(read_record(rec))
        k = Counter(e["kind"] for e in evs)
        units = next(e["payload"]["baseline"]["units"] for e in evs if e["kind"] == "substrate.RunStarted")
        assert k["SESSION_INIT"] == k["SESSION_COMPLETED"] == 1, rec.name
        assert k["CANDIDATE_GENERATED"] == k["COMPARISON_COMPLETED"] == units, rec.name
        assert k["CANDIDATE_ACCEPTED"] + k["CANDIDATE_REJECTED"] == units, rec.name
        assert k["DISCREPANCY_REPORTED"] == k["DISCREPANCY_ROUTED"], rec.name
        for e in evs:
            want = {"CANDIDATE_ACCEPTED": ["acceptance", "comparator", "implementation", "harness"],
                    "CANDIDATE_REJECTED": ["acceptance", "comparator", "implementation", "harness"],
                    "DISCREPANCY_ROUTED": ["router", "acceptance", "comparator", "implementation", "harness"],
                    "DISCREPANCY_RESOLVED": ["resolver", "acceptance", "comparator", "implementation", "harness"]}
            if e["kind"] in want:
                assert _chain(evs, e) == want[e["kind"]], (rec.name, e["kind"])
