# Sprint 131 — Vocabulary 0.10: the 24 tags no code fires, deprecated

```yaml
---
id: 131
status: active
phase: 6
pass_kind: architecture
---
```

## scope

Fix (e) of the direction of 2026-10-24 (BLACKBOARD Decision 2026-10-24 user), step 2 of
`docs/rust-port-plan-2026-10-24.md`. Vocabulary 0.10 deprecates, by one typed proposal (TAG_DEPRECATION_PROPOSED,
`signals/proposals.json` P21), the 24 tags of 0.9 that no code fires: 21 named only in generated code
(`src/w2m/events.py`) and GUEST_CLOCK_READ, GUEST_MEMORY_OBSERVED and GUEST_THREAD_EXITED, named only in tests. Their
1,080 occurrences each under `store/runs/` are the schema copied into each record's baseline and Sprint 126's samples
run, not firings. A deprecated tag keeps its entry, marked with the date, the proposal and the reason (TECHNIQUES.md
#45): records that carry it still replay under their own baselines. The Rust generator carries it no further: no Rust
type, no entry in `harness/schemas/harness-events.json`, and a guest line with its tag is refused by name. The frozen
Python speaks 0.9 (`src/w2m/__init__.py` pins `VOCAB_VERSION = "0.9"`; `gen_events.py`, `check_records.py` and
`tests/test_events.py` read `signals/current.json`), so `current.json` keeps naming 0.9, the oracle's vocabulary,
and a second pointer, `signals/rust-current.json`, names the version Rust code speaks: 0.10. No Python file and no
Python gate changes; at plan step 4, when the Python host moves under `oracle/`, the two pointers become one.

## prerequisites

- 130 (closed)

## context_files

- `signals/0.9.json`, `signals/0.9-rationale.md`, `signals/proposals.json` (P11, the precedent of a ratified
  deprecation)
- `crates/vocabulary_generator/` (check, render_rust, render_schema), `crates/vocabulary_samples/`,
  `crates/events/src/lib.rs`
- `scripts/gates.sh` (the Python vocabulary gates), `scripts/check_vocab.py` and `scripts/gen_events.py` (read, not
  changed)
- `sdd-kit-3/grammar/PRINCIPLES.md` (TAG_DEPRECATION_PROPOSED), `sdd-kit-3/TECHNIQUES.md` #45

## signal contract

### Emits

The vocabulary samples run (Sprint 126) fires each harness kind 0.10 keeps, twice, through its Rust type; the 19
deprecated harness kinds no longer appear on it. Its card lists the counts.

### Consumes

`signals/0.10.json`.

### Invariants

- No tag is removed from the file; no record under `store/runs/` changes; every record replays.
- No Python file is added or changed.

## artifact contract

### Files created

- `signals/0.10.json`, `signals/0.10-rationale.md`

### Files modified

- `signals/rust-current.json` (created), `signals/proposals.json`, `crates/vocabulary_generator/`,
  `crates/events/src/generated.rs`, `crates/signal_report/`, `harness/schemas/harness-events.json`,
  `WORKING_AGREEMENT.md`

### Content assertions

- Each of the 24 tags in `signals/0.10.json` carries `deprecated` with `2026-10-24`, `P21` and its reason.
- `crates/events/src/generated.rs` names none of the 24; `harness/schemas/harness-events.json` holds 33 kinds.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `target/release/vocabulary_generator --check`, `scripts/gates.sh` return 0

## observation contract

- **Expected value:** the samples run writes 2 events of each of 33 harness kinds and replays at Level 2.
- **Planted fault 1:** a guest line tagged GUEST_THREAD_EXITED is refused, naming the tag as deprecated.
- **Planted fault 2:** a deprecated tag given an emit site (a tag named in a hand-written crate) fails the check.

## done criteria

The vocabulary the Rust code speaks names only tags some code fires, and the deprecated ones stay readable on the
records that carry them.

## self-review checklist

- [x] Scope names one verifiable property: no Rust type or schema entry for a tag no code fires.
- [x] context_files name the precedent, the generator and the gates touched.
- [x] Emits: the samples run's kinds, with counts.
- [x] Artifact contract gradable by command.
- [x] Observation contract: an expected value and two planted faults.
- [x] One concept: the deprecation.

## result

- `signals/0.10.json`: 0.9 with its version metadata and a `deprecated` mark (date, P21, reason) on 24 tags, 19 harness
  and 5 guest; `diff` against 0.9 shows nothing else. `signals/0.10-rationale.md`; proposal P21 in
  `signals/proposals.json` (40 lines added, in the file's own format; a first jq rewrite of the whole file, 680 lines
  of churn, was undone). The precedent the card named, P13, is not a deprecation; P11 is.
- `signals/rust-current.json` names 0.10 for Rust; `signals/current.json` keeps naming 0.9 for the frozen Python
  host, which pins it in `src/w2m/__init__.py` (BLACKBOARD Decision 2026-10-24, two pointers until step 4). No Python
  file or Python gate changed.
- The generator: a tag's `deprecated` mark parsed and checked; only live tags get a type, a schema entry or a sample;
  `DEPRECATED_TAGS` in the generated code, and `GuestLine::from_tag` and `HarnessEvent::from_kind` refuse those by
  name ("... is deprecated in vocabulary 0.10"); `--check` fails a hand-written crate under `crates/*/src` that names
  one. `crates/events/src/generated.rs` is 910 lines shorter; one of its doc comments (SERVICE_UNIMPLEMENTED's note)
  still mentions a deprecated tag in prose, and no type does. `harness/schemas/harness-events.json` holds 33 kinds.
- Expected value: `store/runs/vocabulary-samples-20261025T004412` holds 2 events of each of 33 kinds and replays at
  Level 2; `store/runs/session-check-20261025T004412` carries SESSION_INIT `vocab_version` "0.10" and replays.
- Planted faults: every deprecated tag refused by name on its plane (all 24, a test); `GUEST_THREAD_EXITED` named in a
  planted crate fails `--check` naming file and tag, and `GUEST_THREAD_EXITEDX` does not; a deprecation whose
  proposal is not `Pn` refused by name.
- Tests: generator 10, samples 8; the workspace's all pass; the gates pass.
