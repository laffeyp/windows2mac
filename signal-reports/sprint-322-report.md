# SIGNAL_REPORT — Sprint 322 — The tree-wide rules run on every change

Made by `signal_report 322` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 322.

## 1. Observed

### scope_confirmation

> Deferred (Sprint 321): the gates select the Rust tests by the crates a change reaches (`scripts/gates.sh` 196-197,
> `rdeps()`), and three tests read source beyond their own crate: `hypervisor/tests/no_wall_clock_bound.rs` (every
> crate), `w2m_vm/tests/layout_constants.rs` (every crate's `src`) and `signal_report/tests/no_python.rs` (the repo).
> Each is skipped when no crate it depends on changes, and Sprint 320's two upper time bounds in `w2m_vm` passed its
> gates and failed CI on its close (101ce0f55). With the card's commit, WORKING_AGREEMENT gains the rule the Drift
> watchlist's "Command descriptions that misstate the command" set at a third and did not get: a description names what
> the command changes, a command that edits never described as a test, a showing or a check (Sprints 276, 278, 304,
> 319, 321).

### work_performed

- df1283cb8 Sprint 322: the tree-wide rules run on every change
- f44bbcfbc Sprint 322 card: the tree-wide rules run on every change

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; the gates' selection.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 322: its Rubber Duck Pass and close.
