# SIGNAL_REPORT — Sprint 125 — Rust reaches substrate

Made by `signal_report 125` on 2026-10-24, after the sprint's close (hand-author log 2026-10-24), from 1 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 125.

## 1. Observed

### scope_confirmation

> The first part of step 2 of `docs/rust-port-plan-2026-10-21.md`, by Decision 2026-10-23 (how Rust w2m reaches
> substrate). Substrate, on its `w2m` branch, gains a process producer (a command whose standard output is one JSON
> event a line, each checked against schemas read from a JSON file) and a topology read from a TOML file
> (`substrate run --file`). w2m gains its Cargo workspace (`Cargo.toml`, `rust-toolchain.toml` pinning 1.99.1) and its
> first crate, `session_events`, a binary with no dependencies that writes SESSION_INIT and SESSION_COMPLETED as lines.
> A topology file in `harness/` runs it under substrate; the record substrate writes holds both events as w2m tags with
> their payloads, and `substrate replay` reproduces it.

### work_performed

- bd5df2e9d Sprint 125: Rust reaches substrate

### signal_trace

```
session-check-20261023T131400: SESSION_INIT 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    0.008  session_events -> SESSION_INIT (contract_version=0.0, harness_version=0.1.0, profile=w2m-session-check, purpose=conformance, vocab_version=0.9)
  t=    0.008  session_events -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=3.0708e-05, outcome=completed)
```

### signal_trace_tags

- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - SESSION_INIT: `vocab_version` "0.9", `profile` "w2m-session-check", `contract_version` "0.0", `harness_version`
>   the workspace's version, `purpose` a value of `enum:session_purpose`.
> - SESSION_COMPLETED: `outcome` a value of `enum:session_outcome`, `discrepancy_count` 0, `accepted_count` 0,
>   `elapsed_seconds` the binary's own time.

## 3. Delta

### emits_check

- `SESSION_COMPLETED`: fired 1 time
- `SESSION_INIT`: fired 1 time

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 125: its Rubber Duck Pass and close.
