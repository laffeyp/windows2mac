# Sprint 170 — Vocabulary 0.12: P22 and P23

```yaml
---
id: 170
status: active
phase: 6
pass_kind: functional
---
```

## scope

Decision 2026-10-24's order and reviewer-2's Low of 2026-10-27 ("P22 settled before step 4's first record"), after
Sprint 169 put the records check in Rust. Two open proposals (`signals/proposals.json`):

- **P22** (INVARIANT_PROPOSED): "T4 holds each EXECUTION_COMPLETED whose execution makes a guest trace to one
  TRACE_INGESTED or TRACE_REJECTED of it; an execution a judge runs and judges by state (the case runner's) makes no
  guest trace and is marked so on EXECUTION_COMPLETED." Its form: an optional `guest_trace` field (`bool`, absent
  meaning true) on EXECUTION_COMPLETED; T4's text and the Rust check exempt an execution with `guest_trace` false, and
  `trace_sha256` stays required: for an execution with `guest_trace` false it names the sha256 of the empty trace,
  which the tag's note in 0.12 says (0.11's note says only "its trace was collected").
- **P23** (PAYLOAD_FIELD_PROPOSED): COMPARISON_COMPLETED's `reference_ref` and `target_execution_id` may name a session
  or an external run (a GitHub Actions run id), not only an execution on the record: the tag's note changes; the
  types (`id`) do not.

`signals/0.12.json` differs from 0.11 in those, its version fields and P22's T4 text; its rationale; P22 and P23
`ratified`; `signals/rust-current.json` names 0.12; `current.json` stays 0.9 (the frozen host). The generated Rust
types follow (`vocabulary_generator`), and every literal that pins 0.11, all found by one search before the change
(2026-10-28: `crates/events/src/generated.rs`, `crates/session_events/src/main.rs` 80,
`crates/vocabulary_generator/tests/planted.rs` 96, `crates/vocabulary_samples/tests/payloads.rs` 89, and the eight
`harness/*.toml` baselines). The Rust records check (Sprint 169) honors `guest_trace`; the Python check reads 0.9 and
is unchanged, so `--all`'s comparison of the two holds while no record carries the field.

## prerequisites

- 169 (closed)

## context_files

- `signals/0.11.json` (EXECUTION_COMPLETED, COMPARISON_COMPLETED, T4), `signals/proposals.json` (P22, P23),
  `signals/0.11-rationale.md`; `crates/vocabulary_generator`; `crates/w2m_vm/src/bin/check_records.rs` (`t4`)

## signal contract

### Emits

- The samples record (`harness/vocabulary-samples.toml`): every live harness kind, and EXECUTION_COMPLETED with
  `guest_trace` false once.

### Invariants

- No Python file added or changed; the Python check's lines equal the Rust's over every record.

## artifact contract

### Files created

- `signals/0.12.json`, `signals/0.12-rationale.md`

### Files modified

- `signals/proposals.json`, `signals/rust-current.json`, `crates/events/src/generated.rs`, the literals above,
  `crates/w2m_vm/src/bin/check_records.rs`, `WORKING_AGREEMENT.md`

### Command exit codes

- `target/release/vocabulary_generator --check`, `scripts/gates.sh --all` return 0

## observation contract

- **Expected values:** the samples record replays and passes the Rust check; a record with an EXECUTION_COMPLETED
  `guest_trace` false and no ingest gives T4 no violation in the Rust check; one with `guest_trace` absent and no
  ingest gives T4 one.
- **Planted faults:** (1) the exemption reversed in the Rust check: the second record passes and the first fails; (2)
  `guest_trace` given as a string in a sample: the Rust type and substrate's schema refuse it.

## done criteria

P22 and P23 are in the vocabulary in force for Rust, and the records check honors them.

## self-review checklist

- [x] Scope names one verifiable property: 0.12 with P22 and P23, honored.
- [x] context_files cite the vocabulary, the proposals and the check.
- [x] Emits: the samples record's tags.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: the vocabulary's two proposals.
