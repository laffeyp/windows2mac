# SIGNAL_REPORT — Sprint 156 — A second thread: CreateThread served by the Rust host, and `sleeper` past its thread request

Made by `signal_report 156` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 156.

## 1. Observed

### scope_confirmation

> Link 5's run-loop judge, its second program (`docs/design-host-compile-path-2026-10-24.md` F.5: "each lets `qpc`, then
> `sleeper`, run further"). `tests/data/timed_waits/sleeper.exe` in the Rust host's run loop (Sprint 155) stops at
> `hvc #8` op 30 (HOST_THREAD), sub-request 1: THREAD_CREATE, its `CreateThread` (measured: `HostRequest { op: 30 }`, no
> block compiled by the host, exits `{1: 1, 8: 1}`). The Rust host now serves it as the Python host's `_create_thread`
> does, and the run, with two threads, takes time slices and lets the VM's own switch (threads.c `w2m_switch`) move
> between them, as the Python's does with one vCPU; it goes on to its next stop, which it names.

### work_performed

- 147337ef8 Sprint 156: a second thread; CreateThread served as the Python host serves it, and sleeper runs on to THREAD_EXIT_CODE
- 5b3796597 Sprint 156: its card, before its code (a second thread; sleeper past its thread request)

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> None this sprint: the skeleton's record comes at its gate (link 6), as the design orders; the run's stop is the test's
> output.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 156: its Rubber Duck Pass and close.
