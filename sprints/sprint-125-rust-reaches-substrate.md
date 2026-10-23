# Sprint 125 — Rust reaches substrate

```yaml
---
id: 125
status: done
phase: 6
pass_kind: architecture
---
```

## scope

The first part of step 2 of `docs/rust-port-plan-2026-10-21.md`, by Decision 2026-10-23 (how Rust w2m reaches
substrate). Substrate, on its `w2m` branch, gains a process producer (a command whose standard output is one JSON
event a line, each checked against schemas read from a JSON file) and a topology read from a TOML file
(`substrate run --file`). w2m gains its Cargo workspace (`Cargo.toml`, `rust-toolchain.toml` pinning 1.99.1) and its
first crate, `session_events`, a binary with no dependencies that writes SESSION_INIT and SESSION_COMPLETED as lines.
A topology file in `harness/` runs it under substrate; the record substrate writes holds both events as w2m tags with
their payloads, and `substrate replay` reproduces it.

## prerequisites

- 124 (closed)

## context_files

- `sdd-kit-3/AGENTS.md`; `BLACKBOARD.md` Decisions 2026-10-22 user, 2026-10-23 user and agent-resolved (how Rust w2m
  reaches substrate); `WORKING_AGREEMENT.md` (substrate entry, permitted dependencies)
- substrate `docs/api.md` (Producer, `producer_kind`, `TopologyBuilder`, `read_record`), its CLI (`run`, `replay`)
- `signals/0.9.json` (SESSION_INIT, SESSION_COMPLETED; `transport`)

## signal contract

### Emits

- SESSION_INIT: `vocab_version` "0.9", `profile` "w2m-session-check", `contract_version` "0.0", `harness_version`
  the workspace's version, `purpose` a value of `enum:session_purpose`.
- SESSION_COMPLETED: `outcome` a value of `enum:session_outcome`, `discrepancy_count` 0, `accepted_count` 0,
  `elapsed_seconds` the binary's own time.

### Consumes

None.

### Invariants

- No Python file is added to w2m. The schema file for the two tags is written by hand this sprint (hand-author log);
  the generator from `signals/0.9.json` is Sprint 126's.
- Substrate's own tests and gates pass on its `w2m` branch; nothing is pushed to its `main`.

## artifact contract

### Files created

- w2m: `Cargo.toml`, `rust-toolchain.toml`, `crates/session_events/` (Cargo.toml, src/main.rs),
  `harness/session-check.toml` (the topology), `harness/schemas/session.json` (two tags).
- substrate (`w2m` branch): the process producer and the topology file reader, their tests, and their entries in
  `docs/api.md`.

### Content assertions

- The record's application events are exactly SESSION_INIT then SESSION_COMPLETED, each with the payload above.
- A line whose payload lacks a field, or names a kind the schema file lacks, ends the producer as failed, with
  PRODUCER_EMITTED_INVALID or PRODUCER_FAILED on the record naming the line.

### Command exit codes

- `cargo build --release` and `cargo test` at the workspace root return 0
- `uv run substrate run --file harness/session-check.toml` returns 0 and writes a record
- `uv run substrate replay <record>` returns 0
- substrate's own test suite on its `w2m` branch returns 0

## observation contract

- **Planted fault:** `session_events --plant-missing-field` writes SESSION_COMPLETED without `accepted_count`; the run
  must end with the producer failed and the record naming the field. Reach: one field of eight.
- **Expected value:** the two events' payloads on the record equal what the binary wrote, field for field.

## done criteria

A Rust binary's events reach a substrate record through substrate's own code, replay, and are refused when malformed.

## self-review checklist

- [x] Scope names one verifiable property: a Rust program's events on a substrate record.
- [x] context_files name substrate's API and the two tags.
- [x] Emits: both tags with their payload values.
- [x] Artifact contract gradable by command.
- [x] Observation contract: a planted fault with its reach, and an expected value.
- [x] One concept across two repositories: the boundary between them.
