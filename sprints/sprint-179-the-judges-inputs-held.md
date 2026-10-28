# Sprint 179 — The judges' inputs held

```yaml
---
id: 179
status: active
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
