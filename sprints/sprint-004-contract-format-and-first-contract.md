# Sprint 004 — contract format and the first contract

```yaml
---
id: 004
status: closed
phase: 1
pass_kind: architecture
---
```

## scope

Author `src/w2m/contract.py`: a strict msgspec schema for contract files and `load_contract(path)`, which decodes a contract and enforces the v0.2 §11.1-11.2 rules that are mechanical today: id and version match the file location `contracts/<id>/<version>.json`; documented evidence cites an https URL, a retrieval date, and a verbatim quote; hypothesis evidence carries a `distinguishing_check` (constraint E2); an `allowed_outcome_set` acceptance names only declared roles, lists each outcome once, and shares no outcome with its forbidden list. Author the first contract, `contracts/windows.event.auto_reset.single_signal/0.1.0.json`, from the v0.2 §11.2 fragment, status `proposed`, with two documented evidence entries quoted from Microsoft Learn (Event Objects; WaitForSingleObject) and one hypothesis.

## prerequisites

- 002

## context_files

- `windows_to_macos_research_direction_v0_2.md` §3, §11.1-11.3
- `signals/0.1.json` (EVIDENCE_RECORDED, CONTRACT_PROPOSED; E2, E3)
- `src/w2m/events.py`
- Microsoft Learn "Event Objects (Synchronization)" and "WaitForSingleObject function", read 2026-10-05

## signal contract

### Emits

None (format and data; `CONTRACT_PROPOSED` is emitted by the contract_builder operator in a later sprint).

### Invariants

- `signals/0.1.json` unchanged; contract files carry no behavior that the vocabulary must name.

## artifact contract

### Files created

- `src/w2m/contract.py`
- `contracts/windows.event.auto_reset.single_signal/0.1.0.json`
- `tests/test_contract.py`

### Content assertions

- The contract file contains `"Do not assume a first-in, first-out (FIFO) order."` and `"allowed_outcome_set"`.
- `src/w2m/contract.py` defines `def load_contract(`.

### Command exit codes

- `uv run pytest` returns 0
- `uv run ruff check` returns 0
- `uv run mypy` returns 0

## observation contract

Not applicable (no runtime behavior; the comparator in Sprint 005 is the first consumer).

## done criteria

The first contract loads strictly and every malformed variant in the test is refused with a named reason.

## notes

JSON, not YAML: the stdlib parses it and no new dependency is added. v0.2 §11.2's YAML is illustrative. The contract makes the outcome observable through the vocabulary as it stands: waiters use a timed `WaitForSingleObject`, so the released waiter returns `WAIT_OBJECT_0` (0) and the other returns `WAIT_TIMEOUT` (258); the postcondition is a zero-timeout wait by `main` returning 258. No GUEST_OBJECT_TRANSITIONED (P4) is needed.

## self-review checklist

- [x] Scope concrete.
- [x] context_files listed, sources read today.
- [x] Emits empty, consistent.
- [x] Assertions gradable.
- [x] Observation contract not required (architecture).
- [x] One concept (contract format) — schema, its first instance, its test.
