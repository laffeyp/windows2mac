# SIGNAL_REPORT — Sprint 292 — The slice constructor test-only

Made by `signal_report 292` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 292.

## 1. Observed

### scope_confirmation

> Reviewer-3's Low 1 at 278, as Surfaced dispositioned it: "the card that moves the last module makes the slice
> constructor test-only, leaving `memory_view` and `from_raw_parts`". Sprint 290 moved the last module that writes guest
> memory while the run runs. `ConcurrentGuestMemory::new(&mut [u8])` builds a view from a slice; outside the view's own
> tests it is called at `guest.rs` 853, the code cache at load, and in `monitor_code.rs`' and `run.rs`' tests. A view
> built from a slice is sound only while no vCPU runs; built from the mapping (`guest::memory_view`) it needs no such
> care.

### work_performed

- 364763e11 Sprint 292: the slice constructor test-only
- 340a50e1b Sprint 292 card: the slice constructor test-only; reviewer-3's Low 1 at 278

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a constructor's reach, held by a doc test and the type checker.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 292: its Rubber Duck Pass and close.
