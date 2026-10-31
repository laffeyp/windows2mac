# SIGNAL_REPORT — Sprint 282 — The run loop reaches guest memory through the view

Made by `signal_report 282` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 282.

## 1. Observed

### scope_confirmation

> Census row D7's fourth module (the step 6 sequence Decision, annotated at 278 and 281). `crates/w2m_vm/src/run.rs`
> holds 23 slice calls, none in its tests (which start at line 1108), and 7 views built from a slice, one in its tests.
> It is the run loop: each exit's service, the count's give-back, the vector swap of a thread switch, the transport
> page's answer, the dispatch insert, the reconstruction table's lookup and the stop's figures.

### work_performed

- 7800ac78e Sprint 282: the run loop reaches guest memory through the view
- d27871927 Sprint 282 card: the run loop reaches guest memory through the view

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

See the BLACKBOARD tail entry for Sprint 282: its Rubber Duck Pass and close.
