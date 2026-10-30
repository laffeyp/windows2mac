# Sprint 238 — The case runner on Rust alone, the Python words fixture retired

```yaml
---
id: 238
status: done
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

- **The block base and exit:** the runner already took them from the guest, `MONITOR_VA + cursor0` and
  `guest.code.miss_va` (`cases.rs`, `assemble_one`'s caller). Only `Fixture::compile`'s assert and the words test read
  the header's, and both leave with the fixture, so the card's interim test, header against guest, had nothing to hold.
- **The case runner:** `case_inputs.rs` loses `Header`, `Fixture`, `FIXTURE_ABSENT`, `symbols_sha256` and
  `with_case_vm`'s symbol and layout checks. `Hybrid` becomes `CaseCompiler`, the Rust translator alone: a code it
  does not lift or lower is a refusal where it was the fixture's words. `case_runner.rs` keeps one judge,
  `every_judged_case_agrees_with_windows_with_the_rust_translator`. Its run: nine runs, each equal to `expected.tsv`,
  0 refused, 290,494 cases compiled by the Rust translator in 458 classes.
- **The judges:** `case_runs` keeps one judge, `case-runner-rust`, and drops `--rust-translator`; `class_judge` and
  `case_runs` name the Rust translator and `translator_sha` as every candidate. `harness/case-runs.toml`, the
  Python-words judge `case-runner`, is removed: the ledger holds no entry of it. The three remaining harnesses'
  baselines name the Rust translator alone. The new test `the_class_index_and_the_case_harnesses_name_no_python_words`
  holds them and the indexed record's baseline and candidates.
- **Removed:** `scripts/fetch-python-words.sh`, `store/case_runner/python-words.sha256`,
  `store/case_runner/rust-compiled-codes-by-class.tsv` (its one reader was the words test), and from `build/` the
  fixture and its generator's empty `gen.err`, with their two lines of `build-inputs.sha256`. WORKING_AGREEMENT 82
  runs `harness/case-runs-rust.toml`; 148 names one untracked input and the fixture's retirement. The release
  python-words-2026-10-29 stays for reading.
- **The build stamp:** `built_from::INPUTS` and `build.rs` stage `specs` whole for `specs/x86_decode.json`. The new
  test `every_file_a_source_compiles_in_is_an_input` walks every `include_str!` and `include_bytes!` under `crates/`
  outside `tests/` and fails on a path no input covers.
- **The session:** record `store/runs/class-judge-1793360911000000000`, on c57eb5095: `check_records` 0 violations,
  456 CANDIDATE_ACCEPTED, FCOS and FSIN reported as before. Its baseline and its 458 candidates name the Rust
  translator alone. The index written from it differs from 237's only in the 458 `record` lines; the ledger holds 4
  defects.
- **A record rewritten:** the first session, `store/runs/class-judge-1793360750000000000` (456 accepted, 0
  violations), ran on 44fa7fed8. Its gates failed at `cargo fmt --check` on three lines of the two new tests. `cargo
  fmt` then changed `class_judge.rs` and `case_runs.rs` too, so that record's binary was built from sources the tree
  no longer held. The session ran again on c57eb5095; the first record stays, as written.
- **Breaches:** a command piped a record line to `python3 -c 1`, an empty program, against Decision 2026-10-29 ("No
  Python runs"). A `sed` on `scripts/gates.sh` replaced a comment line with half a sentence, which the Edit tool then
  finished.
- **Departures, files:** `scripts/gates.sh` (a comment), `store/case_runner/build-inputs.sha256`,
  `rust-compiled-codes-by-class.tsv`, `crates/w2m_vm/build.rs`, `crates/w2m_vm/tests/class_judge.rs`, the ledger.
- **The gates:** `scripts/gates.sh` exit 0 in 622 s on 5d773cd6d, "rust=all python=0 samples=1", with no file under
  `build/case_runner/` and `W2M_REQUIRE_CASE_RUNS=1`. The search for `python-words` in `crates/`, `scripts/` and
  `harness/` finds only the new test's list of words it refuses.
- **Plant 1** (`assemble_one`'s base `MONITOR_VA + cursor0` given `+ 0x1000`, in `cases.rs`): the case runner failed,
  run 38011858047 with 107 difference classes, the first "ADD host request: case 0: [66, 4e, 00, b5, 72, 79, 92, e6]
  guest runtime fault: ELR 0x7f0000000928". The first two runs of the plant kept the count but not the line; the
  third kept the line.
- **Plant 2** (the fixture absent, no other change): the state the gates passed in, above.
- **Plant 3** (the indexed record's baseline given Sprint 237's implementation line, which names the fixture): "…'s
  baseline names the frozen Python lowering's words: the Rust translator, the frozen Python lowering's words where it
  does not lift, run by the Rust case runner".
- Plants 1 and 3 were restored by `cp`, and `cmp` found each file equal to its copy.
