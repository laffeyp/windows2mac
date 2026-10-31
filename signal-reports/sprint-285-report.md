# SIGNAL_REPORT — Sprint 285 — The case runner reaches guest memory through the view

Made by `signal_report 285` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 285.

## 1. Observed

### scope_confirmation

> Census row D7's next module (the step 6 sequence Decision, annotated at 278-284). `crates/w2m_vm/src/cases.rs`, the
> case runner that judges the CPU cases against Windows' results, holds 12 slice calls, none in its tests (which start
> at line 762), and one view built from a slice (507). It runs one vCPU, but census D7 counts every slice in
> `crates/w2m_vm/src/` until each module is moved or named as a one-vCPU tool, and the case runner's helpers (`windows`,
> `vm_ymm_hi`, `vm_x87`) read the same areas the run loop's do.

### work_performed

- bd86cb8a3 Sprint 285: the case runner reaches guest memory through the view
- c265fd02d Sprint 285 card: the case runner reaches guest memory through the view

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; an access path, held by the case runner's test against Windows' records.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 285: its Rubber Duck Pass and close.
