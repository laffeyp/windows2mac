# SIGNAL_REPORT — Sprint 157 — A thread ends: ExitThread, the exit code and the host's switch, and `sleeper` to ExitProcess

Made by `signal_report 157` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 157.

## 1. Observed

### scope_confirmation

> Link 5's run-loop judge, `sleeper` to its end (`docs/design-host-compile-path-2026-10-24.md` F.5). After Sprint 156
> the Rust host's run of `tests/data/timed_waits/sleeper.exe` prints `woke\n` and stops at `hvc #8` op 30, thread
> request 7 (THREAD_EXIT_CODE, from `WaitForSingleObject` on the spinner's handle). The Python host's run with one vCPU
> goes on to ExitProcess with `woke\ndone\n`. Between the two lie three thread requests and one hand-over the Rust host
> does not serve: THREAD_EXIT_CODE (7), THREAD_EXIT_STUB (2, ExitThread's detach stub), THREAD_END (3, W2mThreadEnd's
> final exit code), and `hvc #12` (HVC_SCHEDULE) with SCHEDULE_END, the thread's end, which threads.c `w2m_switch`
> leaves to the host. The Rust host now serves them as the Python host does, and `sleeper` runs to ExitProcess.

### work_performed

- 3251efabc Sprint 157: a thread ends; ExitThread, the exit code and the host's hand-over served as the Python host serves them, and sleeper runs to ExitProcess
- a9971f7af Sprint 157: its card, before its code (a thread ends; sleeper to ExitProcess)

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

See the BLACKBOARD tail entry for Sprint 157: its Rubber Duck Pass and close.
