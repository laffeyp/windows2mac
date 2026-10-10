# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""contract.py — the contract file format (v0.2 sections 11.1-11.2).

A contract lives at `contracts/<id>/<version>.json`. It carries its behavioral definition and
the evidence for it; documentation and observation keep their identities (v0.2 11.1). Two
acceptance kinds are defined: `allowed_outcome_set`, and `exact` in the form "a read call
returns the argument of the role's latest write call" (Sprint 032). Other relations from v0.2
section 3 are added as contracts need them.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Annotated, Any, Literal

import msgspec
from msgspec import Meta, Struct

from w2m.events import ContractId, EnumEvidenceCategory, Version


class ContractError(ValueError):
    """A contract file is malformed or breaks a v0.2 evidence rule."""


class Evidence(Struct, frozen=True, forbid_unknown_fields=True):
    evidence_id: Annotated[str, Meta(min_length=1)]
    category: EnumEvidenceCategory
    source_ref: str
    applicability: str
    quote: str | None = None
    distinguishing_check: str | None = None
    execution_ids: tuple[str, ...] = ()     # observed: the executions whose traces witness it


class CallReturn(Struct, frozen=True, forbid_unknown_fields=True):
    """A role's GUEST_CALL_RETURNED with this api and return value."""

    role: str
    api: str
    return_value: int


class OutcomeRule(Struct, frozen=True, forbid_unknown_fields=True):
    """The outcome is the set of roles whose `api` call returned `return_value`."""

    api: str
    return_value: int
    over_roles: tuple[str, ...]


class Forbidden(Struct, frozen=True, forbid_unknown_fields=True):
    label: str
    outcome: tuple[str, ...]


class AllowedOutcomeSet(Struct, frozen=True, forbid_unknown_fields=True,
                        tag="allowed_outcome_set", tag_field="relation"):
    outcome: OutcomeRule
    allowed: tuple[tuple[str, ...], ...]
    forbidden: tuple[Forbidden, ...]
    postconditions: tuple[CallReturn, ...] = ()


class ExactReturnOfArgument(Struct, frozen=True, forbid_unknown_fields=True,
                            tag="exact", tag_field="relation"):
    """Each `read_api` call a role in `over_roles` makes returns exactly the integer argument of
    the role's most recent `write_api` call (v0.2 section 3, exact value: error codes). The trace
    carries that argument only as args_sha256 of 'Function(<decimal>)' (vocabulary 0.4 note), so
    the comparator hashes the rendering of each returned value and compares. `pairs` is the
    number of read calls the scenario makes per role; fewer is inconclusive."""

    write_api: str
    read_api: str
    over_roles: tuple[str, ...]
    pairs: Annotated[int, Meta(ge=1)]


class ExactRows(Struct, frozen=True, forbid_unknown_fields=True,
                tag="exact_rows", tag_field="relation"):
    """The target prints one JSON object per scenario; each must equal the reference row with the
    same `key` value, field for field, in the reference's order (v0.2 section 3, exact value or
    bits: defined integer results, error codes; Sprint 042). The rows are the reference
    execution's own output. The event's relation is `exact`; `exact_rows` names this acceptance
    form in the contract file only."""

    key: str
    rows: tuple[dict[str, Any], ...]


Acceptance = AllowedOutcomeSet | ExactReturnOfArgument | ExactRows


class Before(Struct, frozen=True, forbid_unknown_fields=True):
    """A role's GUEST_CALL_RETURNED line for `api`; a precondition must be seen ahead of it."""

    role: str
    api: str


class ThreadStatePrecondition(Struct, frozen=True, forbid_unknown_fields=True,
                              tag="thread_state", tag_field="kind"):
    """Each of `roles` is observed by `observer` (GUEST_THREAD_STATE_OBSERVED) in `thread_state`
    with `wait_reason`, earlier in the observer's thread order than `before` (Sprint 028)."""

    roles: tuple[str, ...]
    observer: str
    thread_state: int
    wait_reason: int
    before: Before
    evidence_ids: tuple[str, ...]     # documented meanings of the two values


Precondition = ThreadStatePrecondition


class Contract(Struct, frozen=True, forbid_unknown_fields=True):
    contract_id: ContractId
    version: Version
    status: Literal["proposed", "accepted", "superseded", "rejected"]
    summary: str
    applicability: str
    roles: tuple[str, ...]
    scenario: dict[str, Any]
    acceptance: Acceptance
    evidence: tuple[Evidence, ...]
    open_questions: tuple[str, ...]
    sections: dict[str, str]
    preconditions: tuple[Precondition, ...] = ()


_RETRIEVED = re.compile(r"retrieved \d{4}-\d{2}-\d{2}")


def _check(c: Contract, path: Path) -> None:
    if path.parent.name != c.contract_id or path.stem != c.version:
        raise ContractError(
            f"{path} must be contracts/{c.contract_id}/{c.version}.json")
    if not c.evidence:
        raise ContractError(f"{c.contract_id}: no evidence")
    ids = [e.evidence_id for e in c.evidence]
    if len(ids) != len(set(ids)):
        raise ContractError(f"{c.contract_id}: duplicate evidence_id")
    for e in c.evidence:
        if e.category == "documented":
            if not e.source_ref.startswith("https://") or not _RETRIEVED.search(e.source_ref):
                raise ContractError(
                    f"{e.evidence_id}: documented evidence needs an https URL and 'retrieved YYYY-MM-DD'")
            if not e.quote:
                raise ContractError(f"{e.evidence_id}: documented evidence needs a verbatim quote")
        if e.category == "hypothesis" and not e.distinguishing_check:
            raise ContractError(f"{e.evidence_id}: hypothesis needs distinguishing_check (E2)")
        if e.category == "observed" and not e.execution_ids and not re.search(
                r"\bexecutions?\b", e.source_ref):
            raise ContractError(f"{e.evidence_id}: observed evidence needs execution_ids (or, in "
                                "contracts before 0.2.1, a source_ref naming the executions)")
    roles = set(c.roles)
    if isinstance(c.acceptance, ExactReturnOfArgument):
        x = c.acceptance
        if not set(x.over_roles) <= roles:
            raise ContractError(f"{c.contract_id}: roles {sorted(set(x.over_roles) - roles)} undeclared")
        if x.write_api == x.read_api:
            raise ContractError(f"{c.contract_id}: write_api and read_api must differ")
    elif isinstance(c.acceptance, ExactRows):
        keys = [r.get(c.acceptance.key) for r in c.acceptance.rows]
        if not keys or None in keys or len(keys) != len(set(keys)):
            raise ContractError(f"{c.contract_id}: every row needs a distinct '{c.acceptance.key}'")
        if not any(e.category == "observed" for e in c.evidence):
            raise ContractError(f"{c.contract_id}: exact_rows rows are observed; cite the executions")
    else:
        _check_allowed_outcome_set(c, c.acceptance, roles)
    ev_ids = {e.evidence_id for e in c.evidence}
    for pre in c.preconditions:
        named = {*pre.roles, pre.observer, pre.before.role}
        if not named <= roles:
            raise ContractError(f"{c.contract_id}: precondition roles {sorted(named - roles)} undeclared")
        if not pre.evidence_ids or not set(pre.evidence_ids) <= ev_ids:
            raise ContractError(f"{c.contract_id}: precondition needs evidence_ids from its evidence")
    # Reference runs are optional (user decision 2026-10-07): documented or observed evidence
    # can carry acceptance; a hypothesis alone cannot.
    if c.status == "accepted" and not any(e.category != "hypothesis" for e in c.evidence):
        raise ContractError(
            f"{c.contract_id}: an accepted contract needs documented or observed evidence")


def _check_allowed_outcome_set(c: Contract, a: AllowedOutcomeSet, roles: set[str]) -> None:
    over = set(a.outcome.over_roles)
    if not over <= roles:
        raise ContractError(f"{c.contract_id}: outcome roles {sorted(over - roles)} undeclared")
    sets = [tuple(sorted(s)) for s in a.allowed] + [tuple(sorted(f.outcome)) for f in a.forbidden]
    for s in sets:
        if not set(s) <= over:
            raise ContractError(f"{c.contract_id}: outcome {list(s)} names roles outside over_roles")
    allowed = {tuple(sorted(s)) for s in a.allowed}
    if len(allowed) != len(a.allowed):
        raise ContractError(f"{c.contract_id}: an allowed outcome is listed twice")
    clash = allowed & {tuple(sorted(f.outcome)) for f in a.forbidden}
    if clash:
        raise ContractError(f"{c.contract_id}: outcome both allowed and forbidden: {sorted(clash)}")
    for p in a.postconditions:
        if p.role not in roles:
            raise ContractError(f"{c.contract_id}: postcondition role {p.role} undeclared")


def load_contract(path: Path, store: Path | None = None) -> Contract:
    """Decode `path` strictly and enforce the format and evidence rules; raise ContractError.
    With `store`, every execution id cited by observed evidence must have a trace in the store
    (vocabulary evidence constraint E3: an execution_id with a trace on record)."""
    try:
        c = msgspec.json.decode(path.read_bytes(), type=Contract)
    except msgspec.ValidationError as e:
        raise ContractError(f"{path}: {e}") from None
    _check(c, path)
    if store is not None:
        known = {json.loads(t.read_bytes().split(b"\n", 1)[0])["execution_id"]
                 for t in (store / "traces").glob("*.jsonl")}
        for ev in c.evidence:
            missing = [x for x in ev.execution_ids if x not in known]
            if missing:
                raise ContractError(f"{ev.evidence_id}: no trace in {store} for {missing}")
    return c
