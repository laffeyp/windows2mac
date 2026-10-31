# SIGNAL_REPORT — Sprint 298 — The guest layout its own crate

Made by `signal_report 298` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 298.

## 1. Observed

### scope_confirmation

> Reviewer-3's Low 1 at 297: `pe_loader` and `virtual_memory` depend on `guest_runtime`, the crate that builds the C
> runtime's image (`lib.rs`: `build`, the clang call, `frozen_root`), for its `layout` module alone (3 uses and 1).
> `layout.rs` (241 lines) imports nothing.

### work_performed

- 0f1a9bfb8 Sprint 298: the guest layout its own crate
- 2d5dd545a Sprint 298 card: the guest layout its own crate; reviewer-3's Low 1 at 297

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a crate's boundary, held by tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 298: its Rubber Duck Pass and close.
