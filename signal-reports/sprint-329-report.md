# SIGNAL_REPORT — Sprint 329 — A stop found at the word's take

Made by `signal_report 329` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 329.

## 1. Observed

### scope_confirmation

> Deferred (Sprint 327) and reviewer-3's Low 2 at 327. At a run's stop, `monitor_code::release_ended_vcpus_word` lets go
> an ended vCPU's guest's hold of the runtime word, so a vCPU waiting on the word is not left waiting for ever. The
> waiter's service then reads what the word guards (the code cursor, the reconstruction and dispatch tables, the run
> table), which the ended vCPU's guest may have left half-written when `hv_vcpus_exit` stopped it, before any turn's
> start finds the run's stop flag. Every service takes the word through `take_word` (`run.rs` 685; ten calls in
> `vcpu_loop` and `take_a_thread`) or, after each exit, `host_take_runtime_word` directly (`run.rs` 1287).

### work_performed

- e780b6afb Sprint 329: a stop found at the word's take; the word let go on every path out of a thread
- 811bbcb19 Sprint 329 card: a stop found at the word's take

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; where a stopped run ends a service.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 329: its Rubber Duck Pass and close.
