# SIGNAL_REPORT — Sprint 127 — Signal Reports in Rust

Made by `signal_report 127` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 127.

## 1. Observed

### scope_confirmation

> Step 2 of `docs/rust-port-plan-2026-10-21.md`, one of its judges. A Rust binary, `signal_report`, writes a sprint's
> Signal Report from its records as `scripts/signal_report.py` did (Sprint 111): the card's scope, the sprint's
> commits, the signal trace (each kind of record counted; each single record narrated by `substrate narrate`, with each
> event's time from the run's start), and the card's Emits checked tag by tag against what fired. It reads the record
> files itself (`events-*.jsonl`, one JSON frame a line) for the first frame's baseline and the count of each kind; the
> frames' integrity stays `substrate replay`'s. A record belongs to a sprint when its baseline's `w2m_commit` is one of
> the sprint's commits, or, for a record whose baseline names no commit (`substrate run --file`, Sprints 125 and 126),
> when one of the sprint's commits added it.

### work_performed

- eb939ec38 Sprint 127: Signal Reports in Rust

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> None. The binary reads records and writes Markdown; it puts nothing on a record.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 127: its Rubber Duck Pass and close.
