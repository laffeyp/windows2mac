# Sprint 179 — The judges' inputs held

```yaml
---
id: 179
status: done
phase: 6
pass_kind: functional
---
```

## scope

reviewer-2's review of Sprints 175-177 (BLACKBOARD Surfaced and the Decision "Sequence after Sprint 178"): High 2 and
Mediums 1 and 3. Each item is an input a judge takes without holding it.

- **The compiler of record** (High 2): the case runner's hybrid compiler takes the Python's words for a class the Rust
  does not lift (`case_inputs.rs` 222) and, from Sprint 177, for a lowering refused as a later group (240, by
  substring); `case_runner.rs` 81 and 324 hold the Rust-compiled map only non-empty. The map, 306 classes and 189,268
  cases at Sprint 177, is committed as `store/case_runner/rust-compiled-by-class.tsv` and asserted in both judging
  tests, naming each class that moved; the refusal becomes a typed `CompileError` variant matched in `case_inputs.rs`
  and `c_words.rs`.
- **Zero-halfword addresses** (Medium 1): the C stencil's branch past a runtime call is fixed for a three-word address,
  and `a64_mov_imm` skips zero halfwords; `w2m_approximate` sits at 0x7f0004060000. A second `c_words` pass gives
  x87_fast.c's five a zero-halfword address and holds the number of calls whose C words differ from the Rust's to a
  committed count; a check over the record header's symbols names each with a zero halfword below bit 48, held to a
  committed list.
- **The gates' last exit** (Medium 3): `scripts/gates.sh` writes `build/gates-last.txt` (exit, mode, the sha256 of
  `git diff HEAD`) on every exit; `signal_report --check` fails a `done` card whose tail entry is not headed closed, and
  a closed entry's gates line must name the exit the file holds for that sprint's work commit.

## prerequisites

- 178 (closed)

## context_files

- `crates/w2m_vm/src/case_inputs.rs` (`Hybrid`), `crates/w2m_vm/tests/case_runner.rs`, `crates/translator/src/lower.rs`
  (`CompileError`), `crates/translator/tests/c_words.rs`, `tools/vm_compiler/a64_encoder.c` 25-32
- `crates/signal_report/src/main.rs`, `scripts/gates.sh`

## signal contract

### Emits

- None: no producer runs; the judges are tests and checks.

### Invariants

- No Python file added or changed; every count the judges held before holds as before.

## artifact contract

### Files modified

- `crates/w2m_vm/src/case_inputs.rs`, `crates/w2m_vm/tests/case_runner.rs`, `store/case_runner/rust-compiled-by-class.tsv`
  (new)
- `crates/translator/src/lower.rs`, `crates/translator/tests/c_words.rs`, `store/census/` (the zero-halfword count and
  the symbol list, new)
- `crates/signal_report/src/main.rs`, `crates/signal_report/tests/check.rs`, `scripts/gates.sh`, `WORKING_AGREEMENT.md`

### Command exit codes

- `c_words`, the case runner, `signal_report --check` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** `rust-compiled-by-class.tsv` holds the classes and counts the case runner printed at Sprint 177
  (306, 189,268) or names each change since; the zero-halfword pass holds a count and the symbol list holds
  `w2m_approximate`.
- **Planted faults:** each on its judge's path, quoted from the judge's output: (1) `lift` refusing ADD_LOCK: the case
  runner names ADD_LOCK moved from the Rust's words; (2) the C's zero-halfword words taken as equal by the second pass:
  `c_words` names the count moved; (3) a `done` card whose tail is headed "gates running": `--check` fails on it.

## done criteria

Each input the review named is held by its judge, and each plant fails it.

## self-review checklist

- [x] Scope names one verifiable property: the judges' inputs held.
- [x] context_files cite the code each check lives in.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants on judges' paths.
- [x] One concept: the judges' inputs held.

## result

- **The compiler of record:** `CompileError` is an enum, `Refused(String)` and `LaterGroup { class, why }`; the MMX
  lowering's refusal is `LaterGroup`, matched by variant in `case_inputs.rs` and `c_words.rs`, no string. The case
  runner holds the Rust-compiled map in both tests that have it: `store/case_runner/rust-compiled-by-class.tsv` (306
  classes, 189,268 cases, the judging run, equal to Sprint 177's printed count) and `rust-compiled-codes-by-class.tsv`
  (306 classes, 16,824 distinct codes); the Python-words judging test has no Rust map to hold.
- **Plant 1** (`lift` refusing ADD_LOCK): the case runner failed, "the Rust-compiled classes moved from
  rust-compiled-by-class.tsv: ADD_LOCK: held Some(740), now None", beside expected.tsv's and expected-by-class.tsv's
  rows. Restored byte for byte.
- **Zero-halfword addresses:** a second `c_words` test gives x87_fast.c's five a zero-halfword address
  (`0x7F01_0000_1000 + 0x100 * i`): 10 of the 24 x87 calls' C words then differ from the Rust's, FSTP m64's 8 and
  FSQRT's 2, whose stencil branches past a runtime call; FLD m64 branches before its only call. Held in
  `store/census/c-words-zero-halfword-2026-10-28.tsv`; and the record's one runtime symbol with a zero halfword below
  bit 48, `w2m_approximate` at 0x7f0004060000, in `runtime-symbols-zero-halfword-2026-10-28.tsv`.
- **Plant 2** (the pass counting no C word as differing): `c_words` failed, "c-words-zero-halfword-2026-10-28.tsv
  holds \"x87_calls\tc_words_differing\n24\t10\n\", now \"x87_calls\tc_words_differing\n24\t0\n\"". Restored byte for
  byte.
- **The gates' last exit:** `scripts/gates.sh` appends each passing run to `build/gates-passes.txt` ("exit 0 in N s",
  its mode, its time); `signal_report --check` fails a `done` card whose tail entry is not headed closed, and a closed
  entry from Sprint 179 on whose "exit 0 in N s" the file does not hold (skipped where the file is absent, as on CI).
- **Plant 3** (a done card under a "gates running" heading, and a closed entry naming a run never kept): the
  integration test `a_done_card_needs_a_closed_tail_naming_a_kept_gates_run` holds `--check`'s output: "sprint 201: its
  card is done and its tail entry is not headed closed" and "sprint 200: its tail entry names \"exit 0 in 912 s\",
  which build/gates-passes.txt does not hold".
- **Departures from the card:** the card had the gates write their last run with the sha256 of `git diff HEAD`, to
  match the sprint's work commit; the gates run on a tree that later splits into a work commit and a close commit, and
  a sprint's gates may run more than once, so the file keeps every passing run and the rule starts at Sprint 179, whose
  earlier entries quote times no file held. `crates/w2m_vm/src/compile.rs` changed (two reads of `CompileError`'s
  string), and the plants' one-line `sed`s are the only edits outside the Edit tool.
