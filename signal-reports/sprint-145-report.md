# SIGNAL_REPORT — Sprint 145 — The case runner: completed cases judged against Windows in a Rust VM

Made by `signal_report 145` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 145.

## 1. Observed

### scope_confirmation

> The case runner's second half (`docs/design-host-compile-path-2026-10-24.md`, link 3; the formats were Sprint 144).
> The Rust host runs CPU cases in a VM in lockstep and judges each completed case against the Windows result, as
> `scripts/vm_oracle.py` `run_cases` and `judge_done` (175-258, 126-167) do. One instruction runs per block. A dispatch
> miss swaps the next case in, and the next miss reads the case's state out.

### work_performed

- 1c0065f26 Sprint 145: the case runner, completed cases judged against Windows in a Rust VM

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

See the BLACKBOARD tail entry for Sprint 145: its Rubber Duck Pass and close.
