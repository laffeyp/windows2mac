# SIGNAL_REPORT — Sprint 312 — Each vCPU's own pages

Made by `signal_report 312` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 312.

## 1. Observed

### scope_confirmation

> The run-loop design's precondition 9 and card 4c2 (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`, split at
> this card). The Rust host reaches per-vCPU state at fixed addresses: the monitor code its data at `MONITOR_VA +
> MONITOR_DATA`, the C runtime its state block and transport page at process-area offsets, the runtime's C frames one
> stack; `compile.rs` 6-7 writes a descriptor once, "One vCPU: no private tables". A second vCPU would share all of it.
> The Python host's answer, each vCPU's own level-0 table with a private chain to its own pages (Sprint 103's design B,
> built at 104, the process words moved at 106), was not carried to Rust. Building before the form is decided would
> repeat 309's first draft, so the form is decided first.

### work_performed

- 614e6fb63 Sprint 312: each vCPU's own pages, decided
- 84b4889ab Sprint 312 card: each vCPU's own pages

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a design.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 312: its Rubber Duck Pass and close.
