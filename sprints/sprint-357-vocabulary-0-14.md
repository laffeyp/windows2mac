# Sprint 357 — Vocabulary 0.14: each lock's takings, hold and wait

```yaml
---
id: 357
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "The vocabulary for each lock's hold and wait" (2026-11-25, reviewer-4's fourteenth review, Medium 1)
puts a vocabulary card, 0.14, before G6 and before the first card that measures a lock other than the runtime word.
Sprint 356's Decision "A card that moves a service class between locks measures the runtime word" makes it the next
card. Change 1 has now built four leaf locks: the heap (343), the TLS and FLS slots (345), the handle table (355) and
the sync objects (356). Vocabulary 0.13 (`signals/rust-current.json`) has no word for any of them. It names
`runtime_word_hold_ns_*` and `runtime_word_wait_ns_*` alone, `holder` for the word alone, and E15 forbids `holder`
elsewhere. Its records count a lock's takings only inside a `variant` string (Sprint 356's Rubber Duck Pass,
observation 2).

This card ratifies typed proposals into 0.14, by the method of Sprints 170 and 336. It adds no emit site; card 358
writes the per-lock figures.

- **The proposals:**
  - **`metric` gains six values** (`ENUM_EXTENDED`):
    - `lock_hold_ns_p50`, `lock_hold_ns_p99`, `lock_wait_ns_p50`, `lock_wait_ns_p99` (nanoseconds);
    - `lock_takings` and `runtime_word_takings` (counts), so a record carries a lock's takings as a figure and not
      only in a `variant` string.
  - **`lock`** (`PAYLOAD_FIELD_PROPOSED`): an optional `lock` field on MEASUREMENT_RECORDED, an enum `lock` of
    `heap`, `slots`, `handles` and `sync_objects`. These are the leaf locks of the service table's `Lock` classes
    (`crates/guest_runtime/src/services.rs`), named as the classes are. `None` is no lock, and the runtime word keeps
    its own metrics. A test holds the enum equal to `Lock`'s leaf variants, so a class added later fails it until the
    vocabulary names it.
  - **E16** (`INVARIANT_PROPOSED`): `lock` is present exactly when `metric` is one of the five `lock_*` values. It is
    checked in `crates/w2m_vm/src/bin/check_records.rs`, as E15 is.

  Each proposal goes into `signals/proposals.json` with its citations, marked `ratified`.
- **The bump itself.**
  - `signals/0.14.json` differs from 0.13 only in those changes and its version fields. Its rationale is
    `signals/0.14-rationale.md`.
  - `signals/rust-current.json` names 0.14; `signals/current.json` stays at 0.9.
  - The generated Rust types and `harness/schemas/harness-events.json` follow from `vocabulary_generator`.
  - Every literal that pins 0.13 moves, found by one `git grep -n -I '0\.13'` over every tracked file, read, not
    assumed. List them in the result, as Sprint 336 did.
- **The samples** carry each new `metric` value once, and each `lock` value once on a `lock_*` metric.
- **Not here:** any emit site of the new words; the timing area's wait start per vCPU. Both are card 358's, the
  per-lock measurement, which must come after the words exist.

## prerequisites

- 356 (closed)

## context_files

- `signals/0.13.json`: MEASUREMENT_RECORDED, `enums.metric`, `enums.holder`, E14 and E15, and
  `grammar_growth.project_overrides`.
- `signals/0.13-rationale.md`, and `signals/proposals.json` (P25-P29, the form a ratified proposal takes).
- `sprints/sprint-336-vocabulary-0-13.md`, the bump before this one: its scope, its plants and its result.
- `crates/vocabulary_generator` (what it generates, and `--check`), and `crates/vocabulary_samples` with its tests and
  the planted samples `harness/vocabulary-samples-planted*.toml`.
- `crates/w2m_vm/src/bin/check_records.rs` (E15's check) and `crates/w2m_vm/tests/records_check.rs` (E15's standing
  test).
- `crates/guest_runtime/src/services.rs` (`Lock`).
- `WORKING_AGREEMENT.md`: the canonical home row "Signal vocabulary" (line 30), lines 133 and 147-148, and lines
  137-266, "Build and verification commands".
- The Decisions: `grep -n "The vocabulary for each lock's hold and wait\|A card that moves a service class between
  locks" BLACKBOARD.md`.

## signal contract

### Emits

- The samples record (`harness/vocabulary-samples.toml`, run in the gates' samples stage) carries MEASUREMENT_RECORDED
  with each of the six new `metric` values once, and with each `lock` value once on a `lock_*` metric. As Sprint 336's
  annotation records, the samples record is not kept under `store/runs/`, so the Signal Report reads it from the
  gates' samples stage; quote that stage's counts.

### Invariants

- 0.13's records still pass the Rust records check, and `check_records` over every record from 2026-10-17 prints no
  new violation.
- No Python file is added or changed.

## artifact contract

### Files created

- `signals/0.14.json`, `signals/0.14-rationale.md`

### Files modified

- `signals/proposals.json`, `signals/rust-current.json`, the generated files, the 0.13 literals the search finds, the
  samples, `check_records.rs` (E16) and its standing test, and a test of the `lock` enum against `Lock`.
  `WORKING_AGREEMENT.md` changes only where the "Signal vocabulary" row names the version.

### Command exit codes

- `target/release/vocabulary_generator --check` returns 0
- `scripts/gates.sh` returns 0 (the samples stage runs)
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The samples record replays under substrate at level 2 and passes the Rust check.
  - `git diff --no-index --stat signals/0.13.json signals/0.14.json` shows only the proposals and the version fields.
  - The records check over every record from 2026-10-17 prints the same counts as before.
- **Planted faults:**
  - (1) A `lock` value outside the enum, `runtime_word`, in a sample. The Rust type and substrate's schema both refuse
    it, each refusal quoted.
  - (2) `lock` on a non-`lock_*` metric, in a sample. The records check names it under E16.
  - (3) A `lock_*` metric with no `lock`. The records check names it under E16.
  - (4) A leaf class added to `Lock` with no vocabulary value. The enum-equality test fails.

  Before the work, read each judge's reach: the gates' samples stage and its planted samples, the samples crate's
  tests, and E15's standing test.

## done criteria

Vocabulary 0.14, holding the six metric values, the `lock` field and enum, and E16, is the vocabulary in force for
Rust. Its samples pass, and four plants are refused.

## notes

AGENTS.md hard rule 5 asks for a COMPREHENSION_AFFIRMATION after a vocabulary bump. The Supervisor writes it at this
sprint's close; the builder puts its own in its final report, as always.

## self-review checklist

- [x] Scope names one verifiable property: 0.14 in force for Rust with its ratified proposals.
- [x] context_files cite the vocabulary, the proposals, the last bump's card, the generator, the samples, the records
      check, `Lock` and the Decisions.
- [x] Emits: the samples record's new values.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the replay, the diff, the records check and four plants, their reach read.
- [x] One concept: the vocabulary's proposals (the files a bump touches are one change, as in Sprints 170 and 336).

## result

GATES-LINE
