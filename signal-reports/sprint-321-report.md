# SIGNAL_REPORT — Sprint 321 — A slice ends for a thread no vCPU will take

Made by `signal_report 321` from 0 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 321.

## 1. Observed

### scope_confirmation

> The run-loop design's "A vCPU with no thread" (Sprint 318), its slice rule: `threads::slice_until` ends the running
> thread's slice at its next stop whenever another unsuspended thread is READY. On several vCPUs that is wrong twice: a
> thread another vCPU runs is READY, so every slice would end at once; and a thread readied while a vCPU waits is that
> vCPU's to take, so the readying vCPU need not stop. The Python host's `_slice_until` (its Sprint 121; the C runtime's
> `readied()`, `threads.c` 118-127) ends the slice only for ready threads beyond the waiting vCPUs.

### work_performed

- a6f395456 Sprint 321: the wait test's upper time bounds removed
- 936e57d77 Sprint 321: a slice ends for a thread no vCPU will take
- aab178453 Sprint 321 card: a slice ends for a thread no vCPU will take

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; the slice's end.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 321: its Rubber Duck Pass and close.
