# SIGNAL_REPORT — Sprint 311 — The run's shared state behind the lock

Made by `signal_report 311` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 311.

## 1. Observed

### scope_confirmation

> The run-loop design's card 4c, first part (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`, card 4; split at this
> card into 4c1, this, and 4c2, the threads). Sprint 310 put `Guest` behind `GuestLock`, but the state the design's "The
> run loop's own state" calls Served stays outside it: `HostCompiler`'s tables, reached at `hvc #1` and `#13`, and the
> records `run` writes into `Outcome` (`bridged`, `blocks` and `translations` at `#1`, `created` and `thread_requests`
> at `#8`, `handovers` at `#12`). At every exit, too, `vm_compiled` drains the in-VM compiler's log under the word
> (Sprint 275) into `blocks`, `translations`, `vm_compile_starts`, `instructions_at` and the host compiler, and
> `exit_count` then reads the host compiler's `tails` holding nothing (`run.rs` 864-884); Sprint 309's reading of the
> loop listed neither. Two vCPU threads would race on all of it. And `turn` waits on the other processes with the guard
> held (Sprint 310's Observation 2), the inversion the guest-memory design's choice 6 forbids.

### work_performed

- 6ed36976a Sprint 311: the run's shared state behind the lock
- 5d7b635b0 Sprint 311 card: the run's shared state behind the lock

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; the lock's contents.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 311: its Rubber Duck Pass and close.
