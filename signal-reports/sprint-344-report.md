# SIGNAL_REPORT — Sprint 344 — Each thread's own values stay its own on two vCPUs

Made by `signal_report 344` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 344.

## 1. Observed

### scope_confirmation

> Sprint 342 took the runtime word off seven services of lock class none. Their judges show the word gone and the call
> ring intact. None shows the risk the class takes: one thread reading the last error, TLS value or thread id of
> another thread served on another vCPU at the same moment. Reviewer-4's thirteenth review, Medium 1, raised this.

### work_performed

- c3bbae356 Sprint 344 card: each thread's own values stay its own on two vCPUs; the probe thread_values_race; the isolation Decision corrected

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a probe and a test.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 344: its Rubber Duck Pass and close.
