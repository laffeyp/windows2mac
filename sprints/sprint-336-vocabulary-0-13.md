# Sprint 336 — Vocabulary 0.13: the two-vCPU run's words

```yaml
---
id: 336
status: done
phase: 6
pass_kind: functional
---
```

## scope

The order in the Decision "Step 6 restored to the user's fix (b), and the order after Sprint 332" (2026-11-24) puts
vocabulary 0.13 next, after the gates' stall (Sprints 334 and 335). The card that follows it has `w2m-vm --vcpus N`
write a substrate record its two-vCPU tests' judges read, and emits CAPABILITY_PROBED where `w2m-vm` makes its refusal
checks. Today vocabulary 0.12 (`signals/rust-current.json`) has no word for any of the following, so that record
cannot carry them:

- the run's vCPU count;
- the count of model steps of a locked form (Sprint 331's "w2m-vm: N model steps of a locked form");
- the turn's start's median and tail (`Outcome.turn_start_ns`, Sprint 324);
- which holder the runtime word is held for;
- the three capabilities step 6's refusal checks read: LSE2 (FEAT_LSE2, the two-vCPU gate in `vcpus_allowed`),
  FEAT_BBM, and guest WFE exits.

This card ratifies four typed proposals into 0.13, following the method of Sprint 170, the 0.12 bump. It adds no emit
site, which is the next card's work.

- **The four proposals:**
  - **`vcpus`** (`PAYLOAD_FIELD_PROPOSED`): an optional `vcpus` field (`int`, at least 1, absent meaning 1) on
    EXECUTION_STARTED and MEASUREMENT_RECORDED.
  - **`metric` gains three values** (`ENUM_EXTENDED`, the form of P24): `locked_model_steps` (a count),
    `turn_start_ns_p50` and `turn_start_ns_p99` (nanoseconds).
  - **`holder`** (`PAYLOAD_FIELD_PROPOSED`): an optional `holder` on MEASUREMENT_RECORDED, an enum of `service`,
    `host_request`, `compile_install`, `turn_start` and `spin`. These are the five the Decision "The holder field and
    the turn's start" names (2026-11-24).
  - **`capability` gains three values** (`ENUM_EXTENDED`): `feat_lse2`, `feat_bbm` and `guest_wfe_exit`. The port
    review's Medium 1 found step 6's probes emit no CAPABILITY_PROBED.

  Each proposal is written into `signals/proposals.json` with its citations and marked `ratified`. Each value's
  `unit`, if the vocabulary ties a metric to a unit, is chosen from what 0.12 already admits.
- **The bump itself.** `signals/0.13.json` differs from 0.12 only in those four changes and its version fields. It gets
  its own rationale, `signals/0.13-rationale.md`, and `signals/rust-current.json` names 0.13, while
  `signals/current.json` stays at 0.9 (the frozen host). The generated Rust types follow from `vocabulary_generator`,
  as does `harness/schemas/harness-events.json`. Every literal that pins 0.12 moves to 0.13. Find all of them with one
  search before the change and list them in the result. A first search gave these paths:
  - `crates/events/src/generated.rs`;
  - `crates/vocabulary_samples/src/main.rs`;
  - `crates/call_compare/src/lib.rs`;
  - `crates/w2m_vm/src/record.rs`, `guest.rs`, `bin/check_records.rs` and `tests/batch_record.rs`;
  - the tracked `harness/*.toml` baselines.

  The search must also cover tests and planted samples, by `grep -rn '0\.12'` over every tracked file, read and not
  assumed.
- **Amended 2026-11-25 (reviewer-4's tenth review):**
  - **Medium 1, `feat_bbm` becomes `feat_bbm_level2`.** CAPABILITY_PROBED's outcome is `enum:capability_outcome`
    (`present`, `absent`, `error`, `not_established`), and that enum cannot carry a level. The capability is therefore
    renamed for the property the design decides on, after Sprint 303's "at FEAT_BBM level 2 a change of block size
    alone needs no break":
    - it reads `present` when `ID_AA64MMFR2_EL1.BBM` (bits 55:52) is 2 or more, and `absent` otherwise;
    - the level read goes in `detail`;
    - P28 and the rationale are amended to match.
  - **Low 1, the `vcpus` bound.** The rationale states that E14's bound is enforced by type at the speaker's mouth: the
    next card's one emit site takes the count as `std::num::NonZeroU64`. That next card's scope carries it.
  - **Low 2, `holder` tied to the runtime word.**
    - A new INVARIANT_PROPOSED, E15: `holder` is present only when `metric` is one of the four `runtime_word_*`
      values, and `crates/w2m_vm/src/bin/check_records.rs` checks it, as Sprint 170's T4 exemption was checked.
    - The rationale gains one sentence: "`turn_start_ns` is the turn's start from before its take of the runtime word
      to after its release, the wait included (`run.rs` 1367-1389); a `runtime_word_hold_ns` with `holder`
      `turn_start` is the hold alone."
    - A third plant goes with it: a sample with `holder` on `instructions_per_second`, which the Rust records check
      names under E15.
  - **Low 3, the proposal labels.** 0.13's `grammar_growth.project_overrides` gains one line. It declares
    `ENUM_EXTENDED` an alias of `PAYLOAD_FIELD_PROPOSED` for an added enum value, and `TAG_PROPOSED` an alias of
    `NEW_TAG_PROPOSED`. The old proposals keep their labels, since proposals are append-only.
- **Not here:**
  - Any emit site of the new words (the next card: the `--vcpus N` record, CAPABILITY_PROBED in `w2m-vm`).
  - README's Status, and the census of the 21 live 0.12 tags with no Rust emit site (port-reviewer's Medium 2). Each is
    a card of its own after the record card.

## prerequisites

- 335 (closed)

## context_files

- `signals/0.12.json`: EXECUTION_STARTED, MEASUREMENT_RECORDED, CAPABILITY_PROBED, `enums.metric`,
  `enums.capability`, and the `unit` enum if one exists.
- `signals/0.12-rationale.md`, and `signals/proposals.json` (P22-P24, the form a ratified proposal takes).
- `sprints/sprint-170-vocabulary-0-12.md`, the bump before this one: its scope, its plants and its result.
- `crates/vocabulary_generator` (what it generates, and `--check`), and `crates/vocabulary_samples`, with its tests
  and the planted samples `harness/vocabulary-samples-planted*.toml`.
- `WORKING_AGREEMENT.md`: the canonical home row "Signal vocabulary" (line 30), lines 133 and 147-148, and lines
  137-266, "Build and verification commands", with "A sprint closes in its work commit".
- The Decisions: `grep -n "Step 6 restored to the user's fix (b)" BLACKBOARD.md` and `grep -n "The holder field and
  the turn's start" BLACKBOARD.md`.
- `crates/w2m_vm/src/run.rs`: `vcpus_allowed` and `Outcome` (`locked_steps`, `turn_start_ns`). Read them so each new
  word names what the code measures.

## signal contract

### Emits

- The samples record (`harness/vocabulary-samples.toml`) carries every live harness kind. It also carries
  EXECUTION_STARTED with `vcpus` 2 once, MEASUREMENT_RECORDED with each of the three new `metric` values and one
  `holder` once, and CAPABILITY_PROBED with each of the three new `capability` values once.

### Invariants

- 0.12's records still pass the Rust records check, and `check_records` over every record from 2026-10-17 prints no
  new violation.
- No Python file is added or changed.

## artifact contract

### Files created

- `signals/0.13.json`, `signals/0.13-rationale.md`

### Files modified

- `signals/proposals.json`, `signals/rust-current.json`, the generated files, the 0.12 literals found by the search,
  and the samples. `WORKING_AGREEMENT.md` changes only where the "Signal vocabulary" row names the version.

### Command exit codes

- `target/release/vocabulary_generator --check` returns 0
- `scripts/gates.sh` returns 0 (every stage the change reaches; the samples stage runs, since
  `crates/vocabulary_samples` changes)
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The samples record replays under substrate at level 2 and passes the Rust check.
  - `git diff --no-index signals/0.12.json signals/0.13.json` shows only the four changes and the version fields;
    quote its stat in the result.
- **Planted faults:**
  - (1) `vcpus` given as a string in a sample. The Rust type and substrate's schema both refuse it, each refusal quoted.
  - (2) A `holder` value outside the five, `lock`, in a sample. The schema refuses it, with "Invalid enum value"
    quoted.

  Before the work, read each judge's reach in the gates' samples stage (`scripts/gates.sh` 258-286, its planted
  samples run and refused) and in the samples crate's tests.

## done criteria

Vocabulary 0.13, holding `vcpus`, the three metric values, `holder` and the three capability values, is the
vocabulary in force for Rust. Its samples pass, and two plants are refused.

## notes

AGENTS.md hard rule 5 asks for a COMPREHENSION_AFFIRMATION after a vocabulary bump. The supervisor writes it at this
sprint's close. The builder puts its own in its final report, as always.

## self-review checklist

- [x] Scope names one verifiable property: 0.13 in force for Rust with four ratified proposals.
- [x] context_files cite the vocabulary, the proposals, the last bump's card, the generator, the samples and the Decisions.
- [x] Emits: the samples record's new kinds and values.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the replay, the diff and two plants, their reach read.
- [x] One concept: the vocabulary's four proposals (the files a bump touches are one change, as in Sprint 170).

## result

- **Vocabulary 0.13** (`signals/0.13.json`, `signals/0.13-rationale.md`): keys sorted, it differs from 0.12 in the
  version fields, `capability` += `feat_lse2`, `feat_bbm_level2`, `guest_wfe_exit`, `metric` += `locked_model_steps`,
  `turn_start_ns_p50`, `turn_start_ns_p99`, the new enum `holder` (`service`, `host_request`, `compile_install`,
  `turn_start`, `spin`), the optional `vcpus` (`u64`) on EXECUTION_STARTED and MEASUREMENT_RECORDED and `holder` on
  MEASUREMENT_RECORDED with their notes, CAPABILITY_PROBED's note, E14 (`vcpus` at least 1, part of P25), E15
  (`holder` only on the four `runtime_word_*` metrics, P29), and one `grammar_growth.project_overrides` line (the
  amendment's Low 3: ENUM_EXTENDED an alias of PAYLOAD_FIELD_PROPOSED for an added enum value, TAG_PROPOSED of
  NEW_TAG_PROPOSED). `git diff --no-index --stat signals/0.12.json signals/0.13.json`: "1 file changed, 49
  insertions(+), 15 deletions(-)". P25 to P29 are ratified into it (`signals/proposals.json`). The amendment of
  reviewer-4's tenth review: `feat_bbm` became `feat_bbm_level2` (present at BBM level 2 or more, absent otherwise, the
  level in `detail`) in 0.13, P28 and the rationale; the rationale states E14 enforced by type at the next card's one
  emit site, `std::num::NonZeroU64`, and carries the quoted sentence on `turn_start_ns` (`run.rs` 1367-1389). Units: the vocabulary ties no metric to a
  unit (`unit` is `text`); the notes name `count` for `locked_model_steps` and `ns` for the two turn-start values,
  units the Rust records already write for `wfe_exits` and `exit_ns_*`. `signals/rust-current.json` names 0.13;
  `current.json` stays 0.9. The generator wrote `crates/events/src/generated.rs` (`VOCAB_VERSION` "0.13", 35 enums,
  `ExecutionStarted::vcpus`, `MeasurementRecorded::vcpus` and `::holder`) and `harness/schemas/harness-events.json`;
  `target/release/vocabulary_generator --check` exit 0, "vocabulary 0.13: 62 tags, 12 categories, 102 foreign keys
  (102 resolved), 16 operators".
- **The 0.12 literals:** one search, `git grep -n -I '0\.12'` over every tracked file before any change, gave 605
  files under `store/runs` (records, each naming the vocabulary it was written under) and the rest in records of
  sprints, reports and data. The pins, all moved to 0.13: `signals/rust-current.json`, the generated code, eleven
  `harness/*.toml` baselines (`case-runs-planted`, `case-runs-rust`, `class-judge`, `session-check`, the four
  `vocabulary-samples*`, `w2m-vm-game`, `w2m-vm-measurements`, `w2m-vm-sleeper`), and `WORKING_AGREEMENT.md` 30 and
  132. Read and left, each the version a word came in or a dated statement: `crates/call_compare/src/lib.rs` 1076,
  `crates/vocabulary_samples/src/main.rs` 55, `crates/w2m_vm/src/bin/check_records.rs` 11 and 317,
  `crates/w2m_vm/src/guest.rs` 1287, `crates/w2m_vm/src/record.rs` 14, 212 and 409,
  `crates/w2m_vm/tests/batch_record.rs` 13, `scripts/gates.sh` 356, the planted guest_trace baseline's comment, and
  P22's and P23's `"into": "0.12"`. Not the version: `contracts/windows.dxgi.dxgi/0.2.0.json` 335 ("modes.28.0.12")
  and `tools/guest_runtime/time_and_error_mode.c` 9 ("13:45:30.125"). No test pins the version: the samples' and the
  generator's tests read it from `events::VOCAB_VERSION` or `signals/rust-current.json` since Sprint 170.
- **The two writers:** `record.rs`'s ExecutionStarted and `measurement_line` set `vcpus` and `holder` to none
  (absent, one vCPU): the batch run takes the instruction clock, which `vcpus_allowed` holds to one vCPU, and
  `run_measurements` passes no `--vcpus`. No emit site of a new word is added.
- **E15 in the records check:** `crates/w2m_vm/src/bin/check_records.rs` applies E15 to each MEASUREMENT_RECORDED
  with a `holder` (absent or null is left out) when the vocabulary holds E15, its metrics `enums.metric`'s
  `runtime_word_*` values, and names a holder on any other metric.
- **The batch record test:** gates run 1 failed at `batch_record::the_batch_runs_record_is_the_frozen_hosts_in_form_and_its_own_stop_in_count`,
  "EXECUTION_STARTED vcpus: null where the reference has nothing": the Rust writer writes a field left out as null
  (`events/src/lib.rs` 361), the reader takes null and absent alike (393-397), and the reference is a 0.9 record
  without the key. The test's field-for-field comparison now drops a null as a field left out (`batch_record.rs`
  170-182); MEASUREMENT_RECORDED is held only on its four CONVENTIONS keys, and no other test holds a written line.
- **The samples:** the record holds EXECUTION_STARTED with `vcpus` 2 once, each of `locked_model_steps`,
  `turn_start_ns_p50`, `turn_start_ns_p99` on MEASUREMENT_RECORDED (the every-metric loop of Sprint 162), each
  `holder` value once on `runtime_word_hold_ns_p50`, and each `capability` value once (two new loops), 128 frames; a
  sample's `holder` is left out unless its metric is `runtime_word_*` (`runtime_word_holder_only`). `substrate replay
  --level 2`: "[OK] Level 2 replay successful." `check_records` over it exit 0, "E15 applied 9 violations 0".
- **The records check:** `target/release/check_records --since 2026-10-17` over every record, exit 0, "38695 records;
  2017 violations; in records made on or after 2026-10-17: 0 held, 170 known"; the same binary with
  `signals/rust-current.json` set back to 0.12 (cp backup, restored by cp, cmp clean) printed the same lines, and
  they equal the lines before E15 was added (no stored record carries a `holder`).
- **Plant 1** (`vcpus` the string "2" on the samples' EXECUTION_STARTED, written past the Rust type): the Rust type
  refused it, "EXECUTION_STARTED.vcpus: expected an integer in [0, 2^64), got \"2\""; substrate's schema refused the
  line, ProducerFailed "line 99: EXECUTION_STARTED: Expected `int | null`, got `str` - at `$.vcpus`", run exit 1.
- **Plant 2** (`holder` "lock" on the samples' MEASUREMENT_RECORDED, written past the Rust type): the Rust type
  refused it, "MEASUREMENT_RECORDED.holder: \"lock\" is not a value of enum:holder"; substrate's schema refused the
  line, ProducerFailed "line 100: MEASUREMENT_RECORDED: Invalid enum value 'lock' - at `$.holder`", run exit 1.
- **Plant 3** (the holder loop's metric `instructions_per_second`, five samples, admitted by the type and the schema,
  substrate run exit 0): the Rust records check named each under E15, "E15 applied 9 violations 5", "holder service on
  metric instructions_per_second"; over the record in a dated directory (`plant3-1795654354000000000`), exit 1, "23
  held", "held: E15 plant3-1795654354000000000: 01JSAMPLE: holder service on metric instructions_per_second".
  Each plant was an edit of `crates/vocabulary_samples/src/main.rs` from a cp backup, restored by cp and checked by
  cmp, the binary rebuilt and the clean samples run again (exit 0, replay OK, E15 violations 0) before the final
  gates. Plants 1 and 2 ran before and again after the amendment, with the same refusals.
- **E15's standing test:** `records_check::a_holder_rides_only_on_a_runtime_word_metric_sprint_336`
  (`crates/w2m_vm/tests/records_check.rs`, a module of `tests/all.rs`) runs the built `check_records` on two dated
  records written for it, each SESSION_INIT, one MEASUREMENT_RECORDED with `holder` `service`, SESSION_COMPLETED: on
  `runtime_word_hold_ns_p50`, exit 0 and "E15 applied 1 violations 0 unchecked 0"; on `instructions_per_second`, exit
  1, "E15 applied 1 violations 1 unchecked 0" and "...: e15-sample: holder service on metric instructions_per_second".
  It passes. Its plant, E15's check taken out (`check_records.rs` 288, `if let Some(metrics) = None::<&Vec<String>>`,
  one `sed` line from a cp backup): the test failed, "panicked at crates/w2m_vm/tests/records_check.rs:89:5:
  assertion `left == right` failed", the first record's output holding no E15 line, and the second record alone
  passed the check, exit 0, "1 records; 0 violations; in records made on or after 2026-10-17: 0 held, 0 known".
  Restored by cp, cmp clean, rebuilt, the test passing again.
- **The gates:** four runs of `scripts/gates.sh`, each of every stage the change reaches (Rust tests all, an input
  outside one crate changed; the substrate samples and their plants). Run 1 (00:37:12Z) failed at the batch record
  test above and wrote no pass row. Run 2 passed, "exit 0 in 362 s" at 00:48:45Z, before the amendment. Run 3 passed,
  "exit 0 in 366 s" at 00:59:06Z, on the amended tree after the three plants. Run 4, with E15's standing test and
  after its plant, passed, "gates: steps 355 s of the run's 360 s, unplaced 5 s", "gates: all passed"; its row:

exit 0 in 360 s	default	2026-11-26T01:08:54Z	sprint 336	a420a5b481be226202595f562f7c14f001873590	c4a3f1e363750bfce5c34b8ccc2c778893fea9aaaf39b4945ac767ea871e2a77	root=86bf520c91bceec89a75d64b2b69340a9a127a5f	rust=all python=0 samples=1	base=ad95f8888ad283a6f2b4bef0e59557abbce49964
