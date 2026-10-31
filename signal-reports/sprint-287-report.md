# SIGNAL_REPORT — Sprint 287 — The model step reaches guest memory through the view

Made by `signal_report 287` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 287.

## 1. Observed

### scope_confirmation

> Census row D7's next module (the step 6 sequence Decision, annotated at 278-286). `crates/w2m_vm/src/model_step.rs`,
> which steps one instruction through the model at an hvc #10 or #11 exit, holds 4 slice calls, none in tests: the
> model's own loads and stores (`GuestModelMemory`, lines 53 and 58), the monitor data and state block read before the
> step (93) and written after it (158). The run loop (`run.rs` 954) and the case runner (`cases.rs` 588) call it while
> the run runs; with a second vCPU its loads and stores are the guest's, made while the other vCPU writes the same pages.

### work_performed

- 714c6f2db Sprint 287: the model step reaches guest memory through the view
- c5d6b01e2 Sprint 287 card: the model step reaches guest memory through the view

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; an access path, held by the model-steps run against the Python host's and the case runner against Windows'.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 287: its Rubber Duck Pass and close.
