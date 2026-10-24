# Sprint 126 — The vocabulary in Rust

```yaml
---
id: 126
status: done
phase: 6
pass_kind: architecture
---
```

## scope

Step 2 of `docs/rust-port-plan-2026-10-21.md`, its vocabulary part. A Rust binary, `vocabulary_generator`, reads the
current vocabulary (`signals/current.json`, then `signals/0.9.json`), checks it as `scripts/check_vocab.py` does, and
writes two files from it: the Rust types of all 62 tags (`crates/events/src/generated.rs`) and substrate's schema file
for the 52 harness-plane tags (`harness/schemas/harness-events.json`, which replaces the hand-written
`harness/schemas/session.json` of Sprint 125). With `--check` it writes nothing and fails when either committed file
differs from a fresh generation. `session_events` writes its two events through the generated types.

Each field's type makes a bad value unrepresentable: a `sha256`, `id`, `contract_id` or `version` is a checked
string; a harness-plane `u64` or `i64` is checked to the RFC 8785 range of 2^53 - 1; a `float` is checked finite; an
enum is a Rust enum. So a payload that builds is a payload substrate accepts, and a tag outside the vocabulary has no
type to build. Decoding (`from_payload`, `GuestLine::parse`) refuses a missing field, an unknown field, and a value
outside its type, naming the field.

Lists need substrate's kernel sprint K271 (`{"type": "list", "items": {...}}`): eleven harness-plane fields are lists.

## prerequisites

- 125 (closed)
- substrate K271 on its `w2m` branch, pinned in `substrate.commit`

## context_files

- `sdd-kit-3/AGENTS.md`; `BLACKBOARD.md` Decisions 2026-10-23 (user; how Rust w2m reaches substrate) and 2026-10-24
  (serde_json); `WORKING_AGREEMENT.md` (hand-author log, permitted dependencies)
- `signals/0.9.json` (`types`, `enums`, `tags`, `transport`); `scripts/gen_events.py` and `scripts/check_vocab.py`
  (what the Rust generator replaces, read, not run)
- substrate `docs/process-producer.md` (the schema file)

## signal contract

### Emits

- SESSION_INIT and SESSION_COMPLETED, as in Sprint 125, now built from the generated types.

### Consumes

None.

### Invariants

- No Python file is added to w2m.
- Every tag of the vocabulary has exactly one generated type; every harness-plane tag has exactly one schema entry.
- The generated files are the generator's output byte for byte (`--check` in the gates and CI).

## artifact contract

### Files created

- `crates/vocabulary_generator/` (the binary), `crates/events/` (hand-written `lib.rs`, generated `generated.rs`)
- `harness/schemas/harness-events.json` (generated; `git mv` from `session.json`)
- `harness/vocabulary-samples.toml` and `harness/vocabulary-samples-planted.toml` (the samples run and its plant)

### Content assertions

- `generated.rs` holds 62 event types and 34 enums; `harness-events.json` holds 52 kinds.
- The schema entries for SESSION_INIT and SESSION_COMPLETED equal Sprint 125's hand-written ones (git history of
  `harness/schemas/session.json`, moved by `git mv`), but for patterns ending in `\Z` where those ended in `$`.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `target/release/vocabulary_generator --check` returns 0
- `uv run substrate run --file harness/session-check.toml --strict` and `substrate replay --level 2` return 0

## observation contract

- **Planted fault 1:** one character of a committed generated file changed: `--check` exits 1 and names the file.
- **Planted fault 2:** a vocabulary with a tag whose verb is not past tense, and one whose field has an unknown type:
  the generator refuses each, naming the tag. Reach: two of the structural checks.
- **Planted fault 3:** a payload of each kind decoded with a field missing, a field extra, and a value out of range
  (a `u64` of 2^53, an enum value outside its list): each refused, naming the field.
- **Planted fault 4** (review of Sprint 126): a SESSION_INIT whose `vocab_version` ends in a newline, which the Rust
  `Version` refuses: substrate's generated schema must refuse it too, the Producer failed naming `vocab_version`.
- **Expected value:** every harness-plane kind's schema, read by substrate's `schemas_from_file`, accepts the payload
  the generated Rust type writes for a sample value of each field type, on a record that replays.

## done criteria

The vocabulary reaches Rust and substrate from one generator, and nothing hand-written repeats it.

## self-review checklist

- [x] Scope names one verifiable property: the vocabulary's one source, generated into both planes' checks.
- [x] context_files name the vocabulary, the two Python scripts replaced, and substrate's schema format.
- [x] Emits: the two session tags, unchanged.
- [x] Artifact contract gradable by command.
- [x] Observation contract: three planted faults with their reach, and an expected value.
- [x] One concept: the vocabulary's one source.

## result

- `vocabulary_generator` (check, render, `--check`): on `signals/0.9.json` it prints 62 tags, 12 categories, 102
  foreign keys (102 resolved), 16 operators, as `scripts/check_vocab.py` does. The reviewer ran 18 vocabularies, each
  breaking one rule, through both: equal exit codes and error counts on all 18. The E7, E8 and cadence checks now
  match whole UPPER_SNAKE words, where the Python matched substrings (a tag named only inside `XSESSION_COMPLETEDX`
  passed it).
- `generated.rs`: 62 event types, 34 enums, `GuestLine`, `HarnessEvent`; `harness-events.json`: 52 kinds. The two
  session kinds equal Sprint 125's hand-written ones but for `\Z` in place of `$`.
- Planted faults: (1) one character changed in either generated file: `--check` exit 1, naming it; (2) a verb not past
  tense, a field of unknown type, an unknown enum, a Rust keyword as a field, an enum value not an identifier, a tag
  named only inside a longer word: each refused by name; (3) 289 required fields left out, 308 fields given a value
  outside their type, 62 tags given an unknown field: each refused at its path; (4) `vocab_version` "0.9\n": substrate
  refused it at `$.vocab_version`, the run exit 1.
- Expected value: the 52 harness kinds at sample values, twice each (104 events, integers at 2^53 - 1, a text with a
  quote, a newline and é), accepted by substrate's generated schema; the record replays at Level 2.
- Tests: 19 in the workspace (`session_events` 4, `vocabulary_generator` 8, `vocabulary_samples` 7).
