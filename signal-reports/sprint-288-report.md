# SIGNAL_REPORT — Sprint 288 — A test refuses outputs made from other sources

Made by `signal_report 288` from 0 records under store/runs/ whose baseline names one of the sprint's 5 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 288.

## 1. Observed

### scope_confirmation

> Reviewer-3's Low 1 at 287, the class: six tests judge files the gates' batch-run step writes, and a run of the tests
> after a change and before that step judges the old files. Sprint 286's pre-commit run did, naming `create_step+0x4`
> from Sprint 285's record against the frozen `+0x0`; Sprint 284's did the same. WORKING_AGREEMENT's rule since 286 runs
> the step first; nothing checks it. `built_from`'s tree would not have caught it: its inputs leave out `tools/`, whose C
> `guest_runtime` compiles when a run starts (`crates/guest_runtime/src/lib.rs` 37-39, 155).

### work_performed

- 6d13df494 Sprint 288: a file checked by its own stamp
- ce1541d55 Sprint 288: a stamp a file, each removed with its file at a run's start
- 92ea752da Sprint 288: each tree_of call takes its own index
- f7b1a7a72 Sprint 288: a test refuses outputs made from other sources
- 61a572f77 Sprint 288 card: a test refuses outputs made from other sources; reviewer-3's Medium 1 at 287, the design's choice 6, the lock order

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a refusal, held by tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 288: its Rubber Duck Pass and close.
