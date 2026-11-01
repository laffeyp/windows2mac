# SIGNAL_REPORT — Sprint 318 — A vCPU with no thread

Made by `signal_report 318` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 318.

## 1. Observed

### scope_confirmation

> The run-loop design's card 4c3b (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`, card 4) runs a second vCPU's
> thread, and its "The threads" section gives a parked vCPU one line: it "waits on the `sched` condition ... a thread
> readied on another vCPU wakes it". No host code parks, waits or wakes: `THREAD_WAKE` is answered 0 and nothing more
> (`host_request.rs` 665), and `threads::switch` calls a state with no ready thread and no timed waiter a deadlock, which
> on two vCPUs it is not while the other vCPU runs a thread. The Python host solved it at Sprints 118 to 121
> (`docs/design-several-vcpus-2026-10-17.md` 160-200: the park, `_wait_for_change` with the earliest deadline as its
> timeout, the notify, `RT_WAITING` and `W2M_THREAD_WAKE`, `_slice_until` past the waiting vCPUs, the deadlock only when
> every vCPU waits a whole second, and vCPU 0 driving the Machine), and the C runtime keeps its half (`threads.c` 51-52,
> 115-126). Building before the form is decided would repeat 309's first draft.

### work_performed

- 8572350ce Sprint 318: a vCPU with no thread, decided
- 7f413755e Sprint 318 card: a vCPU with no thread

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a design.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 318: its Rubber Duck Pass and close.
