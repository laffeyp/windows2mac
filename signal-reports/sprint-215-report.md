# SIGNAL_REPORT — Sprint 215 — A thread suspended, read and resumed

Made by `signal_report 215` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 215.

## 1. Observed

### scope_confirmation

> The Decision "Sequence after Sprint 214": the batch run stops at row 3131, inside mono's SuspendThread on thread 4, on
> HOST_THREAD's request 4, "not served by the Rust host yet". mono's collector suspends a thread, reads its context and
> resumes it; two guests, `mutual` and `mutual_context`, already stop at request 5. The frozen host serves the three in
> `_thread_request` (vm_backend 3212-3260): THREAD_SUSPEND counts the suspension and answers the previous count (M64 for a
> thread that has ended); THREAD_RESUME answers the count before it falls (0 when it is 0 or the thread has ended);
> THREAD_CONTEXT writes the thread's x86 context into the caller's CONTEXT (1232 bytes, the flags at 48), its EFLAGS
> rebuilt from the monitor's lazy flags. The card reads that code against Windows's rows first
> (contracts/windows.thread.running_context, windows.thread.threads) and follows it where they agree (the user's rule of
> 2026-10-29); the frozen host's waits for another vCPU (`_stop_elsewhere`, `_held_elsewhere`) do not arise on the Rust
> host's one vCPU and are named as such.

### work_performed

- 5d577372a Sprint 215: a thread suspended, read and resumed
- 309ef13ad Sprint 215 card: a thread suspended, read and resumed

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - The batch run's record carries the new stop and the comparison; no new emit site.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 215: its Rubber Duck Pass and close.
