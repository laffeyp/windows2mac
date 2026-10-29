# SIGNAL_REPORT — Sprint 200 — The gates run what the change reaches

Made by `signal_report 200` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 200.

## 1. Observed

### scope_confirmation

> The user's word of 2026-10-28: "running 15 minutes of tests per sprint is fucking insane", "need to test more
> efficiently". Sprints 197-199's gates took 943, 969 and 984 s. Measured on 2026-10-28 on this Mac, idle: the Rust tests
> under `cargo test` 347 s, of which `crates/w2m_vm/tests/case_runner.rs` 271 s (its two passes take turns behind a lock,
> one VM a process); the same tests under `cargo nextest`, a process a test, 188 s (the two passes side by side, 146 and
> 149 s); the Python gates' pytest 427 s (2,550 tests of the frozen Python host, the slowest 144 s). The Python gates ran in
> every sprint because `store/` is in their trigger, and every sprint changes `store/gates`, `store/batch_run` or
> `store/family_runs`, none of which the frozen Python reads; the Rust tests ran all for the same paths.

### work_performed

- 6a9f9dd3b Sprint 200: the gates run what the change reaches
- 892cdcb64 Sprint 200 card: the gates run what the change reaches

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None: the judge is `gates.sh --plan` and the gates' own run.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 200: its Rubber Duck Pass and close.
