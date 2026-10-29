# SIGNAL_REPORT — Sprint 213 — A created thread's stack grown

Made by `signal_report 213` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 213.

## 1. Observed

### scope_confirmation

> The Decision "Sequence after Sprint 211": the batch run stops at row 1355, between calls, on HOST_THREAD's
> THREAD_STACK_GROW (9) for thread 8, a created thread, address 0x1180dce5f0, probe 1: "a created thread's growth, not
> carried yet" (host_request.rs, `HostError::NotServed`). The frozen host serves it in `_stack_grow` (vm_backend
> 3161-3188), which follows Windows's rows (contracts/windows.memory.created_thread_stack 0.1.0, default.grow and
> default.grown.*): the touched guard page and those above it committed read-write, StackLimit (TEB+0x10) moved to it,
> three guard pages below (PAGE_READWRITE | PAGE_GUARD); a probe grows page by page; 0, an access violation, when the
> address is outside the reservation's guard reach or the guard would reach the reservation's last page. The card reads
> that code against the contract's rows before porting it (the user's rule of 2026-10-29), and sweeps threads.rs for
> copied defects (the sweep's uncovered files).

### work_performed

- 72473ac31 Sprint 213: a created thread's stack grown
- f41c006e0 Sprint 213 card: a created thread's stack grown

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - The batch run's record carries the new stop; no new emit site.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 213: its Rubber Duck Pass and close.
