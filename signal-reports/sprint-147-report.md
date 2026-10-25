# SIGNAL_REPORT — Sprint 147 — The fault path: cases Windows ended in a fault, judged through the guest's exception records

Made by `signal_report 147` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 147.

## 1. Observed

### scope_confirmation

> The case runner's second judge (`docs/design-host-compile-path-2026-10-24.md`, link 3): the cases Windows ended in a
> fault, run in the Rust VM and judged as `scripts/vm_oracle.py` `judge_fault` (96-123) judges them. A fault goes the way
> a Windows fault goes in w2m. The EL1 vector saves the registers. The guest runtime (`runtime.c` `w2m_exception`) maps
> the host PC to the instruction through the reconstruction table, builds the EXCEPTION_RECORD and CONTEXT, and dispatches
> them (`exceptions.c`). With no handler, the walk ends the process, leaving both records in the exception area.

### work_performed

- 5efcfd8ae Sprint 147: the fault path, faults judged through the guest's exception records

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> None yet: the record comes at the skeleton's gate.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 147: its Rubber Duck Pass and close.
