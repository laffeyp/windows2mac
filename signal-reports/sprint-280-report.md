# SIGNAL_REPORT — Sprint 280 — Host requests reach guest memory through the view

Made by `signal_report 280` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 280.

## 1. Observed

### scope_confirmation

> Census row D7's second module (the step 6 sequence Decision, annotated at 278). `crates/w2m_vm/src/host_request.rs`
> serves the guest's host requests (`hvc #8`): it reads the request from the transport page, copies the buffers the
> request names (a file read's destination, a path, a kernel request's structures, through `KernelMemory`, 236-262), and
> writes the answer back. It holds 22 slice calls, 20 of them outside its tests (before line 668); census D7 and
> `tests/slice_calls.rs` hold it at 22.

### work_performed

- e91524683 Sprint 280: host requests reach guest memory through the view
- c02a64269 Sprint 280 card: host requests reach guest memory through the view

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; an access path, held by tests and the one-vCPU runs.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 280: its Rubber Duck Pass and close.
