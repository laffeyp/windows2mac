# SIGNAL_REPORT — Sprint 290 — LoadLibrary reaches guest memory through the view

Made by `signal_report 290` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 290.

## 1. Observed

### scope_confirmation

> Census row D7's next module, by run-time access: `crates/w2m_vm/src/modules.rs` serves the guest's LoadLibrary
> (`host_request.rs` 428, `modules::module`) while the run runs, and its four slice calls write guest memory then: a
> system DLL's synthetic page (146), a loaded DLL's image (189), its import slots (208), and PEB.Ldr's lists (237). With
> a second vCPU, another thread may read those pages, the loader's lists above all, as the request is served. Of the 31
> guest slices left, the others run at load, in tests, between cases, or in one-vCPU diagnostics; `turns.rs`' two serve
> a child process the supervisor runs alone, and `monitor.rs`' two reads are test helpers' (`MonitorMemory::read_u*`,
> called from `tests/` only).

### work_performed

- d0e2fa622 Sprint 290: LoadLibrary reaches guest memory through the view
- fb6bc3ee3 Sprint 290 card: LoadLibrary reaches guest memory through the view

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; an access path, held by the batch run, the family runs and the process area's tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 290: its Rubber Duck Pass and close.
