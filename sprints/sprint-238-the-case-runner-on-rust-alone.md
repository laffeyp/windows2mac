# Sprint 238 — The case runner on Rust alone, the Python words fixture retired

```yaml
---
id: 238
status: active
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's direction at 235 and the split of card 237: since Sprint 237 the Rust translator compiles every case code
and every conforming class is judged on its words, so no judge needs the frozen Python lowering's words.

- **The fixture's readers, by search (2026-10-30; WORKING_AGREEMENT, a card lists its readers first):**
  - `crates/w2m_vm/src/case_inputs.rs`:
    - `Fixture` (load, parse, the pin `store/case_runner/python-words.sha256`, `FIXTURE_ABSENT`);
    - `Hybrid`, which compiles by Rust and compares with the fixture's words, falling back to them;
    - the header's block base and exit, which the callers pass to `compile`;
    - the symbol and layout checks against the header (367-379).
  - `crates/w2m_vm/src/bin/case_runs.rs` 176-253 and `class_judge.rs` 245-333: each loads the fixture, compiles
    through it, and writes its sha256 into the record's baseline as part of the implementation's name.
  - `crates/w2m_vm/tests/case_runner.rs`:
    - `the_rust_words_equal_the_python_words_on_every_case_code`;
    - `every_judged_case_agrees_with_windows`, the Python words run by the Rust case runner;
    - `every_judged_case_agrees_with_windows_with_the_rust_translator`.
  - `harness/case-runs.toml`, `harness/case-runs-rust.toml` and `harness/class-judge.toml`: the baselines'
    `implementation` lines and the comments that name the fixture.
  - `scripts/fetch-python-words.sh`, `store/case_runner/python-words.sha256`, WORKING_AGREEMENT 148.
  - Stored records: 47 files under `store/runs/` name the fixture in their baselines. They stay as written; they
    record what ran then.
  - Not this card's readers: the run-loop, exit-ledger and batch-record fixtures (`qpc-python.json` and others),
    other frozen-host records.
- **The work:**
  - The block base and exit come from the guest layout or the translator, as the batch run takes them, held equal to
    the fixture's header by a test while the fixture still exists, then the test retired with it.
  - `Hybrid` becomes the Rust compile alone.
  - The two binaries and the harnesses name the Rust translator as the implementation and record its sha256
    (`translator_sha`) where they recorded the fixture's.
  - The Python-words test and judge leave `case_runner.rs`; the Rust-translator judge stays.
  - The fixture's script, pin and WORKING_AGREEMENT entry go; the release `python-words-2026-10-29` stays for reading.
  - The two binaries' start check, `w2m_vm::built_from`, stamps `specs/` too, which `approximate.rs` compiles in
    (Deferred, Sprint 237; reviewer-3's Low 1 of its review at 237); a test fails on an `include_str!` or
    `include_bytes!` outside `tests/` that names a path `built_from` does not stamp.
- **Not here:** FCOS and FSIN.

## prerequisites

- 237 (closed)

## context_files

- the files listed above; `crates/w2m_vm/src/built_from.rs` (Deferred, Sprint 237: `specs/` outside its reach)

## signal contract

### Emits

- The class judge's session, its baseline naming the Rust translator only.

### Invariants

- 456 of 458 classes conform; every case set's counts as `expected.tsv` holds them.

## artifact contract

### Files modified

- the files listed above; `scripts/fetch-python-words.sh` and the pin removed

### Command exit codes

- `scripts/gates.sh` returns 0; the class judge's session completes and `check_records` passes it

## observation contract

- **Expected values:** the case runner's counts unchanged; the class index unchanged but its baseline; no file of the
  case runner opens `build/case_runner/python-words.txt` (a search).
- **Planted faults:**
  1. The block base off by one page: the case runner names a run's first case.
  2. The fixture file absent with no other change: every case runner test still runs.
  3. A record's baseline naming the fixture: the record test names it.

## done criteria

The case runner, the case-runs judge and the class judge read no Python output.

## self-review checklist

- [x] Scope names one verifiable property: no judge reads the Python words.
- [x] context_files cite every reader found by search.
- [x] Emits: the class judge's session.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the Python words fixture's retirement.

## result
