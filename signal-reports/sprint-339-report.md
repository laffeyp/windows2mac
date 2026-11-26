# SIGNAL_REPORT — Sprint 339 — The thread-local calls and QueryPerformanceCounter take no runtime word

Made by `signal_report 339` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 339.

## 1. Observed

### scope_confirmation

> Change 1 of the host design: "a runtime call takes the lock of what it touches, or none"
> (`docs/design-rust-host-2026-10-24.md` 95-140). Its first lock class is **None**, "for what one thread owns or what
> no one writes". The design names those calls, which include the three hottest of the game's import mix:

### work_performed

- 6db5fcdfd Sprint 339 card: the thread-local calls and QueryPerformanceCounter take no runtime word

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - The new measurements record: MEASUREMENT_RECORDED `runtime_word_hold_ns_p50` and `runtime_word_hold_ns_p99` for
>   `qpc` as before, their `variant` naming the takings, and the rest of Sprint 337's record unchanged in form.

## 3. Delta

### emits_check

- `MEASUREMENT_RECORDED`: NOT FIRED

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 339: its Rubber Duck Pass and close.
