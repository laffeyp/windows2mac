# SIGNAL_REPORT — Sprint 326 — The run makes its vCPU on a thread of its own

Made by `signal_report 326` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 326.

## 1. Observed

### scope_confirmation

> The run-loop design's "The threads": the run creates a host thread per vCPU, each making its own vCPU, running it and
> serving its exits. A `Vcpu` stays on the thread that made it (`hypervisor`'s `Vcpu`, neither `Send` nor `Sync`; `Vm`
> is `Sync`, and `hv_vcpu_create` makes a vCPU for the calling thread). Today each of the run's three callers makes the
> one vCPU on its own thread and hands it in: `w2m-vm.rs` `on_one_vcpu` (with a timer thread that stops it at the run's
> time limit through its handle), `tests/run_loop.rs` `run`, `tests/first_decline.rs`. Card 4c3b2b's second vCPU needs
> the run to make its vCPUs; this card moves the one vCPU's making into the run first, so 4c3b2b changes how many it
> makes and not where.

### work_performed

- e57889444 Sprint 326: the run makes its vCPU on a thread of its own
- 69a026e8a Sprint 326 card: the run makes its vCPU on a thread of its own

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; where the run's vCPU is made.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 326: its Rubber Duck Pass and close.
