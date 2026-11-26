# SIGNAL_REPORT — Sprint 346 — A new block's code is visible to every vCPU before its entry is published

Made by `signal_report 346` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 346.

## 1. Observed

### scope_confirmation

> Sprint 345 closed partial at a fault older than its card (BLACKBOARD Deferred (Sprint 345, partial) (a); the card's
> result, its diagnosis paragraph). On two vCPUs, `probes/thread_slots_race` stops at "guest runtime fault: ELR
> 0x7f00000680xx ESR 0x2000000 (EC 0x0)":
> - 6 of 21 runs on the tree before card 345, 6 of 25 runs of its test after it, 0 of 42 runs on one vCPU;
> - every fault lands on the first word of the newest host-compiled block, with no in-VM compile logged
>   (`VMC_LOG_COUNT` 0);
> - the block is the return from WaitForSingleObject that the 16 FLS threads reach once the main thread's SetEvent
>   wakes them all;
> - that word reads `f1000af7`, a valid `subs`, at the stop and 50 ms later, while the CPU took it as undecodable;
> - the other vCPU was in the dispatcher's wait for the runtime word.

### work_performed

- 725c88fa9 Sprint 346 card: a new block's code is visible to every vCPU before its entry is published

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; the order of a block's maintenance and its publish.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 346: its Rubber Duck Pass and close.
