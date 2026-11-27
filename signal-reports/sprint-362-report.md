# SIGNAL_REPORT — Sprint 362 — The objects that cross processes behind their own lock, `kernel`

Made by `signal_report 362` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 362.

## 1. Observed

### scope_confirmation

> Card 4f1 of `docs/design-a-process-that-starts-another-2026-11-26.md` ("The cards that follow", item 1). The design is
> the spec. `kernel.rs`' `Objects` holds what processes share: named events, pipes, and a record per process. Today it
> lives in the first process's `Guest` (`served.kernel`), and the turn form's `Supervisor` with it. `turn` moves both out
> of the `Guest` and back in around each hand-over (`run.rs` 948-949, 969-972 at ae82b92ba). This card moves them into one
> mutex beside `GuestLock`, the host design's Change 4 `kernel` lock: "`kernel`, for processes, cross-process names,
> handles and pipes, one request at a time" (`docs/design-rust-host-2026-10-24.md` 289).

### work_performed

- e0e8fd619 Sprint 362 card: the objects that cross processes behind their own lock, kernel (card 4f1)

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None: where the shared objects live and which lock covers them; no emit site changes. Every run's records are
>   unchanged.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 362: its Rubber Duck Pass and close.
