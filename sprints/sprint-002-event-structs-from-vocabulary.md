# Sprint 002 — event Structs generated from the vocabulary

```yaml
---
id: 002
status: closed
phase: 1
pass_kind: architecture
---
```

## scope

Author `scripts/gen_events.py`, which reads `signals/0.1.json` and writes `src/w2m/events.py`: one frozen msgspec Struct per tag whose class name is the tag (harness plane), tagged Structs for the 9 `guest` tags with a `GuestLine` union for strict JSONL decoding (guest plane), and `validate(obj)` that enforces field types and constraints by round-trip `msgspec.convert`. `--check` exits 1 when the committed module differs from a fresh generation. A test emits SESSION_INIT and SESSION_COMPLETED through a real substrate run and finds them on the record.

## prerequisites

- 001

## context_files

- `signals/0.1.json` (`transport`, `types`, `enums`, `tags`)
- `signals/0.1-rationale.md` (transport decision)
- `substrate/src/substrate/kernel/sequencer.py` lines 168-200 (kind = class name; schema check)
- substrate `docs/api.md` (`TopologyBuilder`, `Runtime`, `assert_event`)
- `src/w2m/__init__.py`

## signal contract

### Emits

- `SESSION_INIT` (vocab_version, profile, contract_version, harness_version, purpose) — in the test run
- `SESSION_COMPLETED` (outcome, discrepancy_count, accepted_count, elapsed_seconds) — in the test run

### Invariants

- Generated class names equal tag names; 51 harness classes + 9 guest classes = 60.
- `signals/0.1.json` unchanged.

## artifact contract

### Files created

- `scripts/gen_events.py`
- `src/w2m/events.py` (generated)
- `tests/test_events.py`

### Content assertions

- `src/w2m/events.py` contains `class SESSION_INIT(` and `GuestLine =`.
- `src/w2m/events.py` first line names `scripts/gen_events.py` and the vocabulary sha256.

### Command exit codes

- `uv run python scripts/gen_events.py --check` returns 0
- `uv run pytest tests/test_events.py` returns 0
- `uv run ruff check` returns 0
- `uv run mypy` returns 0

## observation contract

### Input fixtures

- A two-event substrate topology in `tests/test_events.py` writing to a pytest `tmp_path` record.

### Expected runtime signals

- Record contains `SESSION_INIT` with `vocab_version="0.1"` then `SESSION_COMPLETED` with `outcome="completed"`, and no `substrate.ProducerEmittedInvalidEvent`.

## done criteria

Every tag is a typed Struct that substrate accepts on its bus, and malformed payloads fail `validate()`.

## notes

Three files: generator, generated output, test. The generated file is the generator's output, not a second concept; the test is the grading surface.

## self-review checklist

- [x] Scope concrete: one generator, its output, one test.
- [x] context_files include the substrate source the transport depends on.
- [x] Emits reference only locked tags.
- [x] Assertions gradable.
- [x] Observation contract present (substrate record).
- [x] One concept (vocabulary → types); three files including the generated one.
