# SIGNAL_REPORT — Sprint 350 — A design for the handle table's lock and the sync objects' locks in the guest runtime

Made by `signal_report 350` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 350.

## 1. Observed

### scope_confirmation

> The host design's Change 1 (`docs/design-rust-host-2026-10-24.md` 95-140) names two lock classes still to build:
> - **the handle table's lock**, "held to take, free or look up one slot";
> - **a lock per sync object** (event, semaphore, mutant, critical section's slow path, timer), "held to change its state
>   and pick a waiter; readying the waiter then takes the run-table word".

### work_performed

- 1decadd51 Sprint 350 card amendment: a waiter's closed handle fixed in this sprint; the probe wait_closed_handle
- 23b860e79 Sprint 350 card: a design for the handle table's lock and the sync objects' locks in the guest runtime

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a design document.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 350: its Rubber Duck Pass and close.
