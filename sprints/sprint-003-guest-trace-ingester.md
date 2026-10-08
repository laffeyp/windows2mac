# Sprint 003 — guest-trace ingester

```yaml
---
id: 003
status: closed
phase: 1
pass_kind: functional
---
```

## scope

Author `src/w2m/trace.py`: `ingest(store_root, trace_sha256, execution_id)` reads `store_root/traces/<hex>.jsonl`, verifies the file hashes to `trace_sha256`, decodes every line strictly through `events.decode_guest_line`, and checks invariants T5 (first line GUEST_TRACE_OPENED with matching execution_id and vocab_version; last line GUEST_TRACE_CLOSED with matching line_count and thread_count), T6 (per-thread `thread_seq` starts at 0 on GUEST_THREAD_STARTED and rises by 1), and S14 (nothing after CLOSED); every `hb_refs` entry must name a `thread_id:thread_seq` present in the trace. It returns a validated TRACE_INGESTED or TRACE_REJECTED. `trace_ingester()` wraps it as a substrate Producer.

## prerequisites

- 002

## context_files

- `signals/0.1.json` (guest tags, T4, T5, T6, S14, E7, E8)
- `src/w2m/events.py`
- substrate `docs/api.md` (Trigger, Subscription, PerEvent, quiescence_with_watchdog, assert_event)

## signal contract

### Emits

- `TRACE_INGESTED` (trace_sha256, execution_id, line_count, thread_count, vocab_version)
- `TRACE_REJECTED` (trace_sha256, execution_id, first_invalid_line, reason)
- In the observation test, a stand-in runner also emits `EXECUTION_STARTED` and `EXECUTION_COMPLETED`.

### Invariants

- No out-of-vocabulary tags; every emitted event passes `events.validate`.
- `signals/0.1.json` unchanged.

## artifact contract

### Files created

- `src/w2m/trace.py`
- `tests/test_trace.py`

### Content assertions

- `src/w2m/trace.py` defines `def ingest(` and `def trace_ingester(`.

### Command exit codes

- `uv run pytest` returns 0 (all sprints' tests)
- `uv run ruff check` returns 0
- `uv run mypy` returns 0
- `uv run python scripts/gen_events.py --check` returns 0

## observation contract

### Input fixtures

- A valid two-thread trace (thread 2 waits on an event thread 1 signals; hb edge 1:2) written into a temp store.
- Rejection fixtures: hash mismatch, extra field, thread_seq gap, dangling hb ref, line after CLOSED, wrong line_count.

### Expected runtime signals

- Substrate record: `EXECUTION_STARTED`, `EXECUTION_COMPLETED`, then `TRACE_INGESTED` with the same `trace_sha256` and `line_count=8`, `thread_count=2` (invariant T4 pairing), no `substrate.ProducerFailed`, no `substrate.ProducerEmittedInvalidEvent`.
- A second run with the gap fixture records `TRACE_REJECTED` with `first_invalid_line` pointing at the gap.

## done criteria

A trace becomes citable evidence only through TRACE_INGESTED, and each named malformation is refused with the line number.

## self-review checklist

- [x] Scope concrete.
- [x] context_files listed.
- [x] Emits are locked tags.
- [x] Assertions gradable.
- [x] Observation contract present (functional).
- [x] Two files, one concept.
