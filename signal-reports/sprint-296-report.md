# SIGNAL_REPORT — Sprint 296 — One LOCK_OWNER, one reader

Made by `signal_report 296` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 296.

## 1. Observed

### scope_confirmation

> Two debts named at Sprint 294's close, before `Guest` is divided on them:
> - **The owner's read sits in the run loop.** `threads::schedule`'s assertion (Sprint 294) calls `crate::run::own_word`,
>   the scheduler calling up into the loop that calls it; the drain reads the same word itself (`run.rs` 470).
> - **`LOCK_OWNER` is defined twice**, both 0xB0: `guest_runtime/src/layout.rs` 26, whose value the C runtime's header
>   takes as `W2M_LOCK_OWNER_OFF` (`layout.rs` 192), and `w2m_vm/src/run.rs` 34, which `cases.rs` and `tests/litmus.rs`
>   import. A change to one would leave the host reading a word the runtime does not write.

### work_performed

- 50eea43e1 Sprint 296: one LOCK_OWNER, one reader
- 2ddba4a0d Sprint 296 card: one LOCK_OWNER, one reader

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; one definition and one reader, held by tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 296: its Rubber Duck Pass and close.
