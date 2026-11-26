# SIGNAL_REPORT — Sprint 348 — A run-table entry is published by a release store of the count, read with acquire

Made by `signal_report 348` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 348.

## 1. Observed

### scope_confirmation

> Reviewer-4's eighteenth review, Low 2, recorded in the Decision "Two cards from the eighteenth review". The host's
> `threads::add_thread` (`crates/w2m_vm/src/threads.rs` 168-172) writes a new entry's `E_TID`, `E_STATUS` and `E_TEB`,
> then `RT_COUNT`. Each is a `Relaxed` atomic store through `ConcurrentGuestMemory::write` (`concurrent_memory.rs`
> 128-130).

### work_performed

- ae9782bef Sprint 348 card: a run-table entry is published by a release store of the count, read with acquire

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; the ordering of one shared word.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 348: its Rubber Duck Pass and close.
