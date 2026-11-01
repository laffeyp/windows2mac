# SIGNAL_REPORT — Sprint 320 — How a parked vCPU's wait ends

Made by `signal_report 320` from 0 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 320.

## 1. Observed

### scope_confirmation

> The run-loop design's "A vCPU with no thread" (Sprint 318), its timed wait and its deadlock: Sprint 319's `Sched`
> waits on a change with a timeout the caller gives, and nothing yet says what that timeout is or when a run of parked
> vCPUs is deadlocked. The Python host's Sprint 120: every host wait has the earliest deadline of a blocked thread as its
> timeout and makes expired waits ready when it wakes; a run is deadlocked only when every vCPU waits a whole second with
> no deadline and no notify.

### work_performed

- 86dcbf605 Sprint 320: the deadlock rule over every parked vCPU's deadline
- 52387e735 Sprint 320: how a parked vCPU's wait ends
- c1e4bebcc Sprint 320 card: how a parked vCPU's wait ends

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; the scheduler's waits.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 320: its Rubber Duck Pass and close.
