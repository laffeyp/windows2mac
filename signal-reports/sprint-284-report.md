# SIGNAL_REPORT — Sprint 284 — The guest's dispatch writers store whole pairs

Made by `signal_report 284` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 284.

## 1. Observed

### scope_confirmation

> Reviewer-3's Medium 1 and Low 1 at 283 (BLACKBOARD Surfaced). The dispatch table has three writers. Sprint 276 made the
> host's (`run::dispatch_insert`) store each pair by one `STP` after a release fence. The other two still store a pair
> word by word: the in-VM C compiler (`tools/guest_runtime/block_compiler.c` 145-161: on eviction `p[2] = p[0]; p[3] =
> p[1]; p[1] = code; p[0] = target;`) and the installer (`crates/w2m_vm/src/monitor_code.rs` 696-700: `str` the code,
> then `str` the target, and the way-1 move as two `str`s). The dispatcher reads a pair by one `LDP`, single-copy atomic
> with FEAT_LSE2 (DDI 0487 B2.2.1.1; Sprint 276), so on two vCPUs a lookup between two of those stores can match a target
> with the wrong block. On one vCPU the writer runs on the only vCPU, and nothing can read between its stores.

### work_performed

- 6f40f25fa Sprint 284: the guest's dispatch writers store whole pairs
- b067238dd Sprint 284 card: the guest's dispatch writers store whole pairs

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a store's form, held by tests and the one-vCPU runs.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 284: its Rubber Duck Pass and close.
