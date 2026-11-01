# SIGNAL_REPORT — Sprint 327 — A second vCPU runs the process

Made by `signal_report 327` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 327.

## 1. Observed

### scope_confirmation

> The run-loop design's card 4c3b2b, what Sprints 325 and 326 left of it: the Decision "Architecture: a vCPU with no
> thread" (the park, the wait, the wake, the deadlock over every vCPU) and "The threads" (the stop, the merged outcome),
> on a run of `n` vCPUs. Built before it: the lock held by `&` in `vcpu_loop` (324), the Machine refused above one vCPU
> (325), the vCPU made on the run's own thread (326), `Sched` and `threads::take` (319-321), each vCPU's private chain
> (315) and the switch on any vCPU (316). Today `vcpus_allowed` refuses any count above 1, "a second vCPU is not built",
> and the loop stops with `PARKED_ALONE` where a second vCPU would park (`run.rs` 1281, 1422).

### work_performed

- 411435de3 Sprint 327: a second vCPU runs the process
- f2220324a Sprint 327 card: a second vCPU runs the process

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; how many vCPUs run the process.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 327: its Rubber Duck Pass and close.
