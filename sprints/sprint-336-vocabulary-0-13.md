# Sprint 336 — Vocabulary 0.13: the two-vCPU run's words

```yaml
---
id: 336
status: active
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

GATES-LINE
