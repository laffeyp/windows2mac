"""compare.py — the comparator operator (v0.2 section 3 relations).

Implements `allowed_outcome_set`: the contract names an outcome rule over roles, the allowed
outcomes, the forbidden outcomes, and postconditions. A probe declares which guest thread plays
each role (`role_map`). Only per-thread lines are read, so the verdict never depends on how the
tracer interleaved threads in the file (v0.2 11.4: file order is not guest order).
"""

from __future__ import annotations

from collections.abc import AsyncIterator, Callable, Mapping
from pathlib import Path
from typing import Any

from w2m import events
from w2m.contract import Contract, load_contract
from w2m.trace import trace_path


def _returns(trace_bytes: bytes) -> dict[int, list[events.GUEST_CALL_RETURNED]]:
    by_thread: dict[int, list[events.GUEST_CALL_RETURNED]] = {}
    for line in trace_bytes.splitlines():
        ev = events.decode_guest_line(line)
        if isinstance(ev, events.GUEST_CALL_RETURNED):
            by_thread.setdefault(ev.thread_id, []).append(ev)
    return by_thread


def compare_allowed_outcome_set(
    contract: Contract,
    trace_bytes: bytes,
    role_map: Mapping[str, int],
    comparison_id: str,
    reference_ref: str,
    target_execution_id: str,
) -> events.COMPARISON_COMPLETED:
    """Judge one ingested trace against the contract's allowed_outcome_set acceptance."""
    a = contract.acceptance
    rule = a.outcome
    calls = _returns(trace_bytes)

    def done(verdict: events.EnumVerdict, divergence: str | None = None) -> events.COMPARISON_COMPLETED:
        return events.validate(events.COMPARISON_COMPLETED(
            comparison_id=comparison_id, contract_id=contract.contract_id,
            contract_version=contract.version, relation="allowed_outcome_set",
            reference_ref=reference_ref, target_execution_id=target_execution_id,
            verdict=verdict, first_divergence=divergence))

    missing_roles = [r for r in (*rule.over_roles, *(p.role for p in a.postconditions))
                     if r not in role_map]
    if missing_roles:
        return done("inconclusive")
    outcome: list[str] = []
    witnesses: list[str] = []
    for role in rule.over_roles:
        mine = [c for c in calls.get(role_map[role], []) if c.api == rule.api]
        if not mine:
            return done("inconclusive")
        hit = next((c for c in mine if c.return_value == rule.return_value), None)
        if hit is not None:
            outcome.append(role)
            witnesses.append(f"{hit.thread_id}:{hit.thread_seq}")
    key = tuple(sorted(outcome))
    for f in a.forbidden:
        if key == tuple(sorted(f.outcome)):
            return done("discrepancy",
                        f"forbidden outcome {f.label}: {list(key)} at {witnesses or 'no line'}")
    if key not in {tuple(sorted(s)) for s in a.allowed}:
        return done("discrepancy", f"outcome {list(key)} is not in the allowed set")
    for p in a.postconditions:
        if not any(c.api == p.api and c.return_value == p.return_value
                   for c in calls.get(role_map[p.role], [])):
            return done("discrepancy",
                        f"postcondition unobserved: {p.role} {p.api} returning {p.return_value}")
    return done("conforms")


def comparator(
    store_root: Path, contracts_root: Path,
) -> Callable[[Any], AsyncIterator[events.COMPARISON_COMPLETED]]:
    """Substrate Producer. Input: contract_id, contract_version, trace_sha256, role_map,
    comparison_id, reference_ref, target_execution_id."""

    async def start(inp: Any) -> AsyncIterator[events.COMPARISON_COMPLETED]:
        contract = load_contract(
            contracts_root / inp["contract_id"] / f"{inp['contract_version']}.json")
        raw = trace_path(store_root, inp["trace_sha256"]).read_bytes()
        yield compare_allowed_outcome_set(
            contract, raw, {k: int(v) for k, v in inp["role_map"].items()},
            inp["comparison_id"], inp["reference_ref"], inp["target_execution_id"])

    return start
