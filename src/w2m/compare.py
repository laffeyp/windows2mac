"""compare.py — the comparator operator (v0.2 section 3 relations).

Implements `allowed_outcome_set`: the contract names an outcome rule over roles, the allowed
outcomes, the forbidden outcomes, and postconditions. Implements `exact` in the form of
contract.ExactReturnOfArgument (Sprint 032): each read call returns the argument of the role's
latest write call, checked through the write line's args_sha256. A probe declares which guest
thread plays each role (`role_map`). Only per-thread lines are read, in thread_seq order, so the
verdict never depends on how the tracer interleaved threads in the file (v0.2 11.4: file order
is not guest order).
"""

from __future__ import annotations

import hashlib
from collections.abc import AsyncIterator, Callable, Mapping
from pathlib import Path
from typing import Any

from w2m import events
from w2m.contract import AllowedOutcomeSet, Contract, ExactReturnOfArgument, load_contract
from w2m.trace import trace_path


def _returns(trace_bytes: bytes) -> dict[int, list[events.GUEST_CALL_RETURNED]]:
    by_thread: dict[int, list[events.GUEST_CALL_RETURNED]] = {}
    for line in trace_bytes.splitlines():
        ev = events.decode_guest_line(line)
        if isinstance(ev, events.GUEST_CALL_RETURNED):
            by_thread.setdefault(ev.thread_id, []).append(ev)
    return by_thread


def unwitnessed(contract: Contract, trace_bytes: bytes, role_map: Mapping[str, int]) -> str | None:
    """The first contract precondition the trace does not witness, as text; None if all are.
    A precondition holds only if the observer recorded each role in the required state before
    its `before` call, in the observer's own thread order (v0.2 3: a schedule is witnessed,
    not assumed)."""
    seen: dict[int, list[events.GUEST_THREAD_STATE_OBSERVED]] = {}
    calls = _returns(trace_bytes)
    for line in trace_bytes.splitlines():
        ev = events.decode_guest_line(line)
        if isinstance(ev, events.GUEST_THREAD_STATE_OBSERVED):
            seen.setdefault(ev.thread_id, []).append(ev)
    for pre in contract.preconditions:
        if any(r not in role_map for r in (*pre.roles, pre.observer, pre.before.role)):
            return "precondition roles missing from the role map"
        anchor = next((c for c in calls.get(role_map[pre.before.role], [])
                       if c.api == pre.before.api), None)
        if anchor is None:
            return f"no {pre.before.api} line for {pre.before.role}"
        for role in pre.roles:
            ok = [o for o in seen.get(role_map[pre.observer], [])
                  if o.subject_thread_id == role_map[role] and o.thread_state == pre.thread_state
                  and o.wait_reason == pre.wait_reason
                  and (pre.observer != pre.before.role or o.thread_seq < anchor.thread_seq)]
            if not ok:
                return (f"precondition unwitnessed: {role} not observed by {pre.observer} in "
                        f"state {pre.thread_state}/{pre.wait_reason} before {pre.before.api}")
    return None


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
    assert isinstance(a, AllowedOutcomeSet)
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
    gap = unwitnessed(contract, trace_bytes, role_map)
    if gap is not None:
        return done("inconclusive", gap)          # the run is not an instance of the scenario
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


def compare_exact_return_of_argument(
    contract: Contract,
    trace_bytes: bytes,
    role_map: Mapping[str, int],
    comparison_id: str,
    reference_ref: str,
    target_execution_id: str,
) -> events.COMPARISON_COMPLETED:
    """Judge one ingested trace against an ExactReturnOfArgument acceptance. A read's return
    value r conforms when sha256 of '<write function>(<r in decimal>)' equals the args_sha256
    of the role's most recent write line; the comparator never learns the argument otherwise."""
    x = contract.acceptance
    assert isinstance(x, ExactReturnOfArgument)
    write_fn = x.write_api.split("!", 1)[1]

    def done(verdict: events.EnumVerdict, divergence: str | None = None) -> events.COMPARISON_COMPLETED:
        return events.validate(events.COMPARISON_COMPLETED(
            comparison_id=comparison_id, contract_id=contract.contract_id,
            contract_version=contract.version, relation="exact",
            reference_ref=reference_ref, target_execution_id=target_execution_id,
            verdict=verdict, first_divergence=divergence))

    if any(r not in role_map for r in x.over_roles):
        return done("inconclusive", "roles missing from the role map")
    calls = _returns(trace_bytes)
    for role in x.over_roles:
        mine = sorted((c for c in calls.get(role_map[role], [])
                       if c.api in (x.write_api, x.read_api)), key=lambda c: c.thread_seq)
        last_write: events.GUEST_CALL_RETURNED | None = None
        reads = 0
        for c in mine:
            if c.api == x.write_api:
                last_write = c
                continue
            reads += 1
            if last_write is None:
                return done("inconclusive", f"{role}: {x.read_api} at {c.thread_id}:{c.thread_seq} "
                                            f"precedes any {x.write_api}")
            rendered = f"{write_fn}({c.return_value})".encode()
            if "sha256:" + hashlib.sha256(rendered).hexdigest() != last_write.args_sha256:
                return done("discrepancy",
                            f"{role}: {x.read_api} at {c.thread_id}:{c.thread_seq} (read {reads} of "
                            f"{x.pairs}) returned {c.return_value}, which is not the argument of "
                            f"{x.write_api} at {last_write.thread_id}:{last_write.thread_seq}")
        if reads < x.pairs:
            return done("inconclusive", f"{role}: {reads} {x.read_api} lines; the scenario "
                                        f"makes {x.pairs}")
    return done("conforms")


def compare(contract: Contract, trace_bytes: bytes, role_map: Mapping[str, int],
            comparison_id: str, reference_ref: str,
            target_execution_id: str) -> events.COMPARISON_COMPLETED:
    """Judge with the relation the contract's acceptance names."""
    judge = (compare_exact_return_of_argument if isinstance(contract.acceptance, ExactReturnOfArgument)
             else compare_allowed_outcome_set)
    return judge(contract, trace_bytes, role_map, comparison_id, reference_ref, target_execution_id)


def comparator(
    store_root: Path, contracts_root: Path,
) -> Callable[[Any], AsyncIterator[events.COMPARISON_COMPLETED]]:
    """Substrate Producer. Input: contract_id, contract_version, trace_sha256, role_map,
    comparison_id, reference_ref, target_execution_id."""

    async def start(inp: Any) -> AsyncIterator[events.COMPARISON_COMPLETED]:
        contract = load_contract(
            contracts_root / inp["contract_id"] / f"{inp['contract_version']}.json")
        raw = trace_path(store_root, inp["trace_sha256"]).read_bytes()
        yield compare(
            contract, raw, {k: int(v) for k, v in inp["role_map"].items()},
            inp["comparison_id"], inp["reference_ref"], inp["target_execution_id"])

    return start
