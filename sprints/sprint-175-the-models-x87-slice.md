# Sprint 175 — The model's x87 slice

```yaml
---
id: 175
status: active
phase: 6
pass_kind: functional
---
```

## scope

Step 4's model (census row 4), its third slice. After Sprint 174 the case runner's largest block of uncounted model
cases is set 065's: 53,349 of its 54,679 cases go to the model and are counted, not judged. They are x87 classes,
which the Rust translator does not compile, so the case runner's per-class line (`crates/w2m_vm/tests/case_runner.rs`
237, which lists only the classes the Rust compiled) never showed them; the SSE classes left (4,170 cases in 40) come
after.

- **The count first:** the case runner prints each run's model cases by class for every class, so this slice's
  classes and counts are read, not inferred, before the code.
- **The lifting, the model's own:** the Python's `("x87", ic, opcode, reg, st, mem)` operation (`src/w2m/lift.py` and
  the x87 rules), lifted by the model when the translator refuses the form, as Sprint 174 did for MMX.
- **The model:** `runtime.py` `_x87` (140-163) and `src/w2m/x87.py` (696 lines, read in full before the code): the
  register stack, the control and status words, the tags, the 80-bit arithmetic and its exceptions as the Python
  computes them; an exception the control word does not mask stops the run by name, as the Python's `Unimplemented`
  does.
- **The state:** the x87 transfer Sprint 174 added, with FCW from the monitor data page.
- **The arithmetic:** `x87.py` computes with `fractions.Fraction` (exact rationals whose exponents reach 2^-16445 and
  whose sin and cos series run to 160 bits), which no fixed-width integer holds. The port uses `num-rational` 0.4.2's
  `BigRational` over `num-bigint` 0.4 (rust-num; num-rational 0.4.2 requires num-bigint ^0.4, crates.io 2026-10-28),
  so each Fraction step has a one-to-one Rust step. Python's `round()` on a Fraction rounds half to even and `%` is
  floored; the port says so where it uses them. The x87 code goes in its own file, `crates/w2m_vm/src/x87.rs`, as
  `x87.py` is its own module.

## prerequisites

- 174 (closed)

## context_files

- `src/w2m/x87.py` (whole), `src/w2m/runtime.py` 140-163, the Python's x87 lifting rules, `src/w2m/vm_backend.py`
  4268-4330
- `crates/w2m_vm/src/model.rs`, `crates/w2m_vm/src/cases.rs`, `crates/w2m_vm/tests/case_runner.rs`

## signal contract

### Emits

- On four `case-runner-rust` records: SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED,
  SESSION_COMPLETED, now judging the x87 classes' model cases.

### Invariants

- No Python file added or changed; every case judged before is judged as before.

## artifact contract

### Files modified

- `crates/w2m_vm/src/x87.rs` (new), `crates/w2m_vm/src/lib.rs`, `crates/w2m_vm/Cargo.toml`, `Cargo.lock`
- `crates/w2m_vm/src/model.rs`, `crates/w2m_vm/src/cases.rs`, `crates/w2m_vm/tests/case_runner.rs`,
  `store/case_runner/expected.tsv`, `WORKING_AGREEMENT.md`, `docs/step-4-census-2026-10-28.md` (row 4); four records

### Command exit codes

- the case runner and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** set 065's x87 model cases judged against Windows with 0 difference classes; `expected.tsv`'s
  "to the model" falling by exactly the count stepped.
- **Planted faults:** each on a path the judge reads (Sprints 173-174): (1) FLD's push not decrementing TOP: the case
  runner names the x87 status word; (2) one rounding mode of FCW's RC field taken as another: the case runner names an
  ST register. Each plant is checked against the Python's code before the card's commit, as Sprint 174's first draft
  was not. Checked: `judge_x87` (`cases.rs` 455-475) compares FSW with only C0-C3 masked, so TOP (bits 11-13) is
  judged, and compares ST0-ST7 by value; `x87.py` 123, 156 and 232 read RC 1 and 2 by the sign, and 470 records set
  065's cases in every rounding mode, so swapping RC 1 (down) and 2 (up) changes an inexact result the judge reads.

## done criteria

Set 065's x87 model cases are judged against Windows through the Rust model.

## self-review checklist

- [x] Scope names one verifiable property: the x87 model cases judged.
- [x] context_files cite the Python to read in full first.
- [x] Emits: the case runner's records.
- [x] Artifact contract gradable by command.
- [x] Observation contract: counts and two plants on paths the judge reads.
- [x] One concept: the x87 slice.
