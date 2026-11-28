# SIGNAL_REPORT — Sprint 382 — The Rust host's batch stream stored as the reference past the frozen host's cut

Made by `signal_report 382` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 382.

## 1. Observed

### scope_confirmation

> Sprint 381's profile re-take moved the game's batch run from row 29, so the frozen Python host's stream
> (`store/frontier-calls/frontier-643a6283510c-1792623423346201000.tsv`) holds row for row only to row 28 of 10,505
> (`LAST_HELD_ROW`, `crates/w2m_vm/tests/batch_run.rs`), and Windows' judge, G3, alone holds the rest. G3 tolerates
> scheduling; the frozen stream held the exact order on the one-vCPU instruction clock, the project's determinism
> anchor. The Decision "The runner's strings re-taken, and the frozen host's reference after them" (Sprint 381) fires
> its revisit now: the Rust host's own stream becomes the reference past the cut.

### work_performed

- 3d48e90e0 Sprint 382 card: the Rust host's batch stream stored as the reference past the frozen host's cut

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None declared: the change is to a test and a stored reference.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 382: its Rubber Duck Pass and close.
