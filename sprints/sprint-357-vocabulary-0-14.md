# Sprint 357 — Vocabulary 0.14: each lock's takings, hold and wait

```yaml
---
id: 357
status: done
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

- None; the card adds no emit site. A vocabulary card's new words are carried by the samples record
  (`harness/vocabulary-samples.toml`, run in the gates' samples stage), which is not kept under `store/runs/` (Sprint
  336's annotation), so the Signal Report cannot read them. Their evidence is the observation contract's: the samples
  stage's measurement lines, with each of the six new `metric` values once and each `lock` value once on a `lock_*`
  metric.
- *(Annotated 2026-11-26 at the close, by the Supervisor: the card first named the measurement tag here, from the
  samples record. Since Sprint 340 the record check fails a declared tag the Signal Report shows NOT FIRED, and the
  report reads `store/runs/` alone; a kept samples record breaks the records check (Sprint 336's annotation). The first
  form was the card's error, not the build's; the samples' counts are in the result.)*

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

- **Vocabulary 0.14** (`signals/0.14.json`, `signals/0.14-rationale.md`), written as jq edits of 0.13 (`jq --indent 1`
  reproduces 0.13's bytes): the version fields, `metric` += `lock_hold_ns_p50`, `lock_hold_ns_p99`, `lock_wait_ns_p50`,
  `lock_wait_ns_p99`, `lock_takings`, `runtime_word_takings` (P30), the new enum `lock` (`heap`, `slots`, `handles`,
  `sync_objects`) and MEASUREMENT_RECORDED's optional `lock` (`enum:lock`) with one sentence of its note (P31), and E16
  (P32). `git diff --no-index --stat signals/0.13.json signals/0.14.json`: "1 file changed, 30 insertions(+), 9
  deletions(-)". `guest_plane_compatible_versions` unchanged (no guest tag changes). P30 to P32 are in
  `signals/proposals.json`, ratified into 0.14, added by Edit at the file's end (jq would rewrite the `—` escapes of
  eleven earlier lines). `signals/rust-current.json` names 0.14; `current.json` stays 0.9. The generator wrote
  `crates/events/src/generated.rs` (`VOCAB_VERSION` "0.14", 36 enums, `events::Lock`, `MeasurementRecorded::lock`) and
  `harness/schemas/harness-events.json`; `target/release/vocabulary_generator --check` exit 0, "vocabulary 0.14: 62
  tags, 12 categories, 102 foreign keys (102 resolved), 16 operators".
- **The Decision's form and the card's:** the Decision "The vocabulary for each lock's hold and wait" names
  `runtime_word` among `lock`'s values and E16 on four `lock_*` metrics; this card names four leaf values and five
  metrics, `lock_takings` among them. The card is followed: the word keeps its own metrics and `holder`, and plant 1
  uses `runtime_word` as the value outside the enum.
- **The 0.13 literals:** one search, `git grep -n -I '0\.13'` over every tracked file before any change, 1,681 lines:
  483 files under `store/runs` (records naming the vocabulary they were written under, and numbers such as
  `elapsed_seconds` 0.138), and records of sprints, reports, BLACKBOARD, `docs/` and `store/` data (`uv 0.13.0`,
  `modes.28.0.13`, times such as 0.1309788 s, `games/.../session-2026-10-16.jsonl`'s `"t":540.133`). Moved to 0.14:
  `signals/rust-current.json`, the generated code, eleven `harness/*.toml` baselines' `vocabulary` line
  (`case-runs-planted`, `case-runs-rust`, `class-judge`, `session-check`, the four `vocabulary-samples*`,
  `w2m-vm-game`, `w2m-vm-measurements`, `w2m-vm-sleeper`), `WORKING_AGREEMENT.md` 30 and 132, and
  `crates/w2m_vm/tests/records_check.rs` 26, whose test record now takes `events::VOCAB_VERSION`. Read and left, each
  the version a word came in or a dated statement: `crates/vocabulary_samples/src/main.rs` 153,
  `crates/w2m_vm/src/bin/check_records.rs` 8, 183 and 286, `crates/w2m_vm/src/bin/run_measurements.rs` 463,
  `crates/w2m_vm/src/record.rs` 304, 473 and 480, `crates/w2m_vm/tests/batch_record.rs` 172, `records_check.rs` 3,
  `harness/w2m-vm-measurements.toml` 5, and P25-P29's text. Not this vocabulary's pin:
  `crates/signal_report/tests/check.rs` 671 and 673, a scratch root's own `rust-current.json` and `0.13.json`;
  `tools/windows_machine/SETUP.md` 179 (`uv 0.13.0`).
- **E15's metrics, a class of error found before the gates:** 0.13's check (`check_records.rs` 216) and the samples
  (`vocabulary_samples/src/main.rs` 159) took E15's metrics as the values beginning `runtime_word_`; 0.14's
  `runtime_word_takings` begins so and is not one of E15's four. The check now takes each of E15's and E16's metrics as
  the values of `enums.metric` the constraint's spec names, whole words (`metrics_named`), and the samples keep `holder`
  only on `runtime_word_hold_ns_*` and `runtime_word_wait_ns_*`. On 0.13's vocabulary both readings give the same four.
- **E16 in the records check:** `check_records.rs` applies E16 to each MEASUREMENT_RECORDED that gives a `lock` (null
  is none) or a `lock_*` metric, naming "lock L on metric M" or "metric M with no lock". `record.rs`' one writer sets
  `lock` to none; no emit site of a new word is added.
- **The samples:** each new `metric` value once in the every-metric loop, and each `lock` value once on
  `lock_hold_ns_p50`; `lock` given exactly on a `lock_*` metric (`holder_and_lock_by_metric`). Run under a dated
  scratch root, exit 0, 138 frames; MEASUREMENT_RECORDED 48 times, EXECUTION_STARTED 3, CAPABILITY_PROBED 20.
  `substrate replay --level 2`: "[OK] Level 2 replay successful.", "Frames replayed: 138". `check_records` over it:
  "E15 applied 9 violations 0 unchecked 0", "E16 applied 9 violations 0 unchecked 0", and the 18 violations Sprint
  340's annotation of 336 records for a dated samples record (T2 1, T3 3, T4 2, S2 8, S4 2, S8 2), the samples writing
  each harness tag out of a session's order. The gates' samples stage ran it (`vocabulary-samples`, 0 s).
- **The records check:** `target/release/check_records --since 2026-10-17 --show 0` over every record, built before the
  change and after, exit 0 both, the output identical by `diff`: "38698 records; 2017 violations; in records made on or
  after 2026-10-17: 0 held, 170 known". No stored record holds a `lock` or a `lock_*` metric, so E16 prints no line.
- **Plant 1** — the Rust type: "MEASUREMENT_RECORDED.lock: \"runtime_word\" is not a value of enum:lock"; substrate's
  schema: ProducerFailed "line 111: MEASUREMENT_RECORDED: Invalid enum value 'runtime_word' - at `$.lock`", run exit 1.
  The samples' lock loop set `lock` `runtime_word` and wrote the line past the type (an Edit of
  `crates/vocabulary_samples/src/main.rs`); substrate keeps no Producer standard error, so the type's refusal was read
  from the planted binary run alone, four lines, exit 0.
- **Plant 2** — the records check: "E16 applied 9 violations 4 unchecked 0", "01JSAMPLE: lock heap on metric
  instructions_per_second" (and slots, handles, sync_objects), exit 1, "22 held". The lock loop's metric made
  `instructions_per_second` (one `sed` line, 152); the type and the schema admit it, substrate run exit 0.
- **Plant 3** — the records check: "E16 applied 9 violations 4 unchecked 0", "01JSAMPLE: metric lock_hold_ns_p50 with
  no lock" four times, exit 1, "22 held". The lock loop's `lock` removed (one `sed` line, 154); substrate run exit 0.
- **Plant 4** — the enum-equality test: "assertion `left == right` failed: Lock's leaf classes (services.rs) and
  vocabulary 0.14's enum lock", left `["heap", "slots", "handles", "sync_objects", "files"]`, right `["heap", "slots",
  "handles", "sync_objects"]` (`services.rs:630`). A variant `Files` added to `Lock` by Edit.
  Each plant ran from a cp backup, restored by cp and checked by cmp; after plants 1-3 the clean samples ran again (exit
  0, replay OK, 138 frames, E15 and E16 violations 0), all before the gates.
- **The two standing tests:** `services::tests::the_vocabularys_lock_values_are_the_leaf_classes_of_lock`
  (`crates/guest_runtime/src/services.rs`) reads `Lock`'s variants from the file's text, less `RuntimeWord` and `None`,
  and the enum `lock` of the vocabulary `rust-current.json` names; it passes. `records_check::a_lock_rides_exactly_on_a_lock_metric_sprint_357`
  runs the built `check_records` on three dated records: `lock` heap on `lock_hold_ns_p50`, exit 0, "E16 applied 1
  violations 0 unchecked 0"; heap on `instructions_per_second` and `lock_takings` with `lock` null, each exit 1, "E16
  applied 1 violations 1 unchecked 0", named. It passes. Its reach, E16's check taken out (`lock_metrics: None`, one
  `sed` line from a cp backup): "panicked at crates/w2m_vm/tests/records_check.rs:131:5", left `None`, right
  `Some("E16 applied 1 violations 0 unchecked 0")`; restored by cp, cmp clean, passing again.
- **The gates:** one run of `scripts/gates.sh`, every Rust test (an input outside one crate changed) and the substrate
  samples and their plants: nextest "408 tests run: 408 passed (4 slow), 1 skipped", "gates: steps 422 s of the run's
  427 s, unplaced 5 s", "gates: all passed". `target/release/signal_report --check` and `design_check` exit 0 on the
  tree before it. Its row:

exit 0 in 427 s	default	2026-11-26T19:51:58Z	sprint 357	2e27ecee06b202d3ce62b5d48e7777fe6a5960e2	5948557c94a6f052878e38390d624a8edf7bcddb742d6d2a0eb3f2143865b33c	root=d9511a1c36d91bef59dac8ad813a6d2798ca7452	rust=all python=0 samples=1	base=051e9fa5e7a2b493ce6b8ffb1231433d5dd21c44
