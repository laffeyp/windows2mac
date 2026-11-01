# SIGNAL_REPORT — Sprint 309 — How the vCPU threads share the guest

Made by `signal_report 309` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 309.

## 1. Observed

### scope_confirmation

> The run-loop design's cards 4b and 4c (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`, split at Sprint 307's
> close) leave one question open, found before 4b's card was committed: in what form the vCPU threads share `Guest`. A
> first draft of 4b made `served` a `Mutex<Served>` reached by `get_mut` at about 230 sites while one vCPU holds `&mut
> Guest`; 4c would rewrite every one when the threads share `Guest`, so the form is decided first. Two forms:

### work_performed

- 114921f8f Sprint 309: how the vCPU threads share the guest, decided
- f413f51ee Sprint 309 card: how the vCPU threads share the guest

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

See the BLACKBOARD tail entry for Sprint 309: its Rubber Duck Pass and close.
