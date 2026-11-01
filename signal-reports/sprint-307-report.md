# SIGNAL_REPORT — Sprint 307 — The flush's registers and a pending cancel

Made by `signal_report 307` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 307.

## 1. Observed

### scope_confirmation

> Reviewer-3's Low 1 at 306: Deferred (Sprint 305) held two tests of `run::flush_through_stub` for card 4's second
> thread, and neither needs one. The register test needs a stopped vCPU; the cancel test needs the pending case
> `hv_vcpu.h` 377-378 states, a cancel sent before `hv_vcpu_run` making that run return at once, so the nested run meets
> it every time.

### work_performed

- 4b30e78d3 Sprint 307: the flush's registers and a pending cancel
- 484116746 Sprint 307 card: the flush's registers and a pending cancel

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 307: its Rubber Duck Pass and close.
