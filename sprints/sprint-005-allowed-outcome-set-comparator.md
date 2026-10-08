# Sprint 005 — comparator for allowed_outcome_set

```yaml
---
id: 005
status: closed
phase: 1
pass_kind: functional
---
```

## scope

Author `src/w2m/compare.py`: `compare_allowed_outcome_set(contract, trace_bytes, role_map, comparison_id, reference_ref, target_execution_id)` reads an ingested guest trace, computes the outcome (the roles in `over_roles` whose thread has a GUEST_CALL_RETURNED with the rule's api and return value), and returns a validated COMPARISON_COMPLETED: `conforms` when the outcome is allowed and every postcondition is observed; `discrepancy` with `first_divergence` when the outcome is forbidden or unlisted or a postcondition is missing; `inconclusive` when a role has no call to the rule's api at all. `comparator()` wraps it as a substrate Producer triggered by TRACE_INGESTED.

## prerequisites

- 003, 004

## context_files

- `src/w2m/contract.py`, `src/w2m/trace.py`, `src/w2m/events.py`
- `contracts/windows.event.auto_reset.single_signal/0.1.0.json`
- `signals/0.1.json` (COMPARISON_COMPLETED; E6; S3, S4)
- v0.2 §3 (relations), §11.3

## signal contract

### Emits

- `COMPARISON_COMPLETED` (comparison_id, contract_id, contract_version, relation, reference_ref, target_execution_id, verdict; first_divergence when verdict=discrepancy)

### Invariants

- Verdict never depends on wall-clock values or file order across threads; only per-thread lines are read.
- E6: `first_divergence` is set exactly when verdict=discrepancy.

## artifact contract

### Files created

- `src/w2m/compare.py`
- `tests/test_compare.py`

### Content assertions

- `src/w2m/compare.py` defines `def compare_allowed_outcome_set(` and `def comparator(`.

### Command exit codes

- `uv run pytest` returns 0
- `uv run ruff check` returns 0
- `uv run mypy` returns 0

## observation contract

### Input fixtures

- Event-probe traces in the shape the contract describes, for outcomes {a}, {b}, {a,b}, {}, a missing postcondition, and a waiter with no wait call.

### Expected runtime signals

- Substrate record: `EXECUTION_COMPLETED` → `TRACE_INGESTED` → `COMPARISON_COMPLETED(verdict=conforms)` for the {b} trace; a second run with the {a,b} trace records `verdict=discrepancy` with `first_divergence` naming `both_waits_complete_from_the_single_signal`. No `substrate.ProducerFailed`.

## done criteria

The first contract's acceptance condition is executable against guest traces, end to end on a substrate record.

## self-review checklist

- [x] Scope concrete.
- [x] context_files listed.
- [x] Emits a locked tag.
- [x] Assertions gradable.
- [x] Observation contract present.
- [x] Two files, one concept.
