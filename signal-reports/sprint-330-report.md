# SIGNAL_REPORT — Sprint 330 — The kick

Made by `signal_report 330` from 0 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 330.

## 1. Observed

### scope_confirmation

> The run-loop design's card 4c3b3, "The kick" (the Python host's Sprint 119): SuspendThread of a thread another vCPU
> holds stops that vCPU (`VcpuHandle::stop`) and waits, the word and the guard let go, until the thread's `vcpu` word
> clears; the vCPU stopped leaves through the host, its `SLICE_UNTIL` 0 and the C's switch declining the suspended
> thread. Today `threads::suspend` (`threads.rs`) raises the count alone, and the thread runs on to its vCPU's next
> switch. Measured before this card, the threads family probe (`store/family_runs/known-2026-10-29.tsv`'s `threads`
> row, through `w2m-vm` with its arguments) on one vCPU and on two: the spinner, on vCPU 1 while the main thread loops
> on SwitchToThread, kept counting after SuspendThread, `suspend.frozen`'s `count_unchanged` 0 where one vCPU reads 1,
> and GetThreadContext read it with `rip_in_image` 0 where one vCPU reads 1; every other row the same.

### work_performed

- e952a31e5 Sprint 330: a kick in the one-vCPU case runner fails the case by name
- d9879467f Sprint 330: the kick; a thread suspended on another vCPU stops before SuspendThread returns
- 5e45d2446 Sprint 330 card: the kick

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; when a suspended thread stops.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 330: its Rubber Duck Pass and close.
