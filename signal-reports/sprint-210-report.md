# SIGNAL_REPORT — Sprint 210 — The batch run's record

Made by `signal_report 210` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 210.

## 1. Observed

### scope_confirmation

> The Decision "Sequence after Sprint 208" (Sprint 210, the batch run's record) and the Decision "Sequence after Sprint
> 205" (reviewer-2's Medium 3 of its review of Sprints 199-205; census row 14): every sprint of the batch-run line has
> declared "signal — none", and `w2m-vm` writes a record only for its translations (record.rs). The frozen host's
> frontier writes, per run (scripts/run_frontier.py 262-266, 380-407): EXECUTION_STARTED, MEASUREMENT_RECORDED per
> exit kind and the instructions a second, SERVICE_UNIMPLEMENTED when the guest stops at a service w2m lacks, and one
> EXECUTION_COMPLETED (exit status exited, timed_out, killed or faulted; elapsed seconds; the trace's sha256 and line
> count). Sprint 209's tail, Observation 7, adds a unit test of GetProcAddress's split between `Guest::imports` and
> `Guest::proc_thunks`, which only the macOS exit ledger judges now.

### work_performed

- 978c2f361 Sprint 210: the batch run's record
- 1a826ec54 Sprint 210 card: the batch run's record

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - SESSION_INIT, EXECUTION_STARTED, MEASUREMENT_RECORDED, SERVICE_UNIMPLEMENTED, EXECUTION_COMPLETED,
>   SESSION_COMPLETED: one session per `w2m-vm --game` or guest run.

## 3. Delta

### emits_check

- `EXECUTION_COMPLETED`: NOT FIRED
- `EXECUTION_STARTED`: NOT FIRED
- `MEASUREMENT_RECORDED`: NOT FIRED
- `SERVICE_UNIMPLEMENTED`: NOT FIRED
- `SESSION_COMPLETED`: NOT FIRED
- `SESSION_INIT`: NOT FIRED

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 210: its Rubber Duck Pass and close.
