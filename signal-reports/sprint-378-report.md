# SIGNAL_REPORT — Sprint 378 — A frozen family held to a contract re-taken on the laptop

Made by `signal_report 378` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 378.

## 1. Observed

### scope_confirmation

> The Decision "Which Windows w2m answers as, the reference being the laptop" (Sprint 374) sets w2m to answer as the
> laptop, and names tables still taken from GitHub's Server runner, `tools/guest_runtime/version.c` and `system_info.c`
> first. Re-taking a table moves the families that read it (`version`, `system_info`, `files_more`): their judge,
> `crates/w2m_vm/tests/family_runs.rs`, holds each run to the contract version in
> `store/family_runs/known-2026-10-29.tsv`, and for a frozen family `crates/w2m_vm/src/bin/family_known.rs` (`contract`,
> 165-193) takes that version from the family record's first COMPARISON_COMPLETED, the runner's 0.1.0. A joined family
> already moves by its row in `store/family_runs/joined-2026-11-27.tsv` (`named_sync_objects` at 0.2.0, Sprint 374); a
> frozen family has no such route.

### work_performed

- 750e5e35e Sprint 378 card amendment: the route by which a frozen family is held to a contract re-taken on the laptop, alone; the table re-takes each a card of its own
- ed3ea8cbc Sprint 378 card: the version and system-info tables re-taken from the laptop's stored rows

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None: the route changes which contract version a judge reads; no run is recorded.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 378: its Rubber Duck Pass and close.
