# Sprint 175 — The model's x87 slice

```yaml
---
id: 175
status: done
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

## result

- **The count first:** the case runner's new line "to the model, not judged" listed set 065's 53,349 model cases in 15
  x87 classes (FADD 6,835, FMUL 6,540, FSTP 5,675, FLD 5,129, FISTTP 4,459, FISTP 4,360, FNSTSW 3,634, FIADD 3,379,
  FIDIVR 3,283, FSQRT 1,961, FCOS 1,728, FWAIT 1,680, FNCLEX 1,632, FSIN 1,584, FLDCW 1,470) and set 065c's FDIVR 1,323
  and FIMUL 1,617: 56,289 cases, the 17 classes of `rules/x87.py`'s `ICLASSES`, read before the code.
- **The slice:** `crates/w2m_vm/src/x87.rs` carries `x87.py` function for function on `num-rational` 0.4.2's
  `BigRational` over `num-bigint` 0.4 (Python's half-to-even `round()` and floored `%` written out where `trig` and
  `_pi66` use them); `model.rs` gains `lift_x87` (`rules/x87.py` `rule`: the last `ST:` operand is ST(i), any other the
  memory one, as the Python's decoder lists only the explicit ST(i)) and `step_x87` (`runtime.py` `_x87` 140-163: the
  x87 state written back only after `execute` returns). Two unit tests hold `_pi66` and `round80(1/3)` to the values
  `x87.py` itself prints (0x3243f6a8885a308d3; 0x3ffdaaaaaaaaaaaaaaab with PE and C1).
- **Judged against Windows:** all 56,289 cases. Set 065 is judged whole, 54,679 of 54,679, its 679 Windows faults all
  judged at the VM's fault (30 before); set 065c judges 6,732 of 7,554. Every class conforms but FSIN and FCOS, which
  differ in the last bit of 18 results: "FCOS st0" 10, "FCOS FSW" 4, "FSIN st0" 8, "FSIN FSW" 4, exactly the counts
  `tests/test_class_runs.py` 33 pins for the Python model (Sprint 065's open pair, Decision 2026-10-12: within 0.08 ulp
  of a rounding boundary; Intel documents the 66-bit pi, not the polynomial). The case runner now holds those four at
  their pins, as the Python test does, and fails on any other difference. `expected.tsv`'s "model" fell by exactly the
  count stepped: set 065 from 53,349 to 0, set 065c from 3,762 to 822 (MPSADBW, an SSE class). Set 065's run takes
  53.4 s, from 13.1 s, in exact arithmetic.
- **Plant 1** (a push that leaves TOP, `Fpu::push`): the case runner failed, "5071 FLD FSW: case 27146: [67, d9, 84,
  47, a3, 88, 5c, e5] model 0x7a61, cpu 0x7261 (in 0x7a20)", with FLD FTW and st1-st7. Restored byte for byte.
- **Plant 2** (RC 1, down, read as 2, up, and 2 as 1, `Fpu::rc`): the case runner failed, "1156 FADD st0: case 3103:
  [67, 4e, d8, 04, ef] model c05cdc9e860000000000, cpu c05cdc9e870000000000", with "1270 FADD FSW". Restored byte for
  byte.
- **Records:** four `case-runner-rust` records (1793187102199422000, -230029000, -258242000, -289222000) hold 545
  class verdicts: 235, 177, 107 and 26. The first profile (8573C, Server 2022, which holds set 065) accepts 233 and
  rejects FCOS and FSIN, each with REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED;
  `case_runs --write-ledger` opened their two entries in `store/case_runner/discrepancy-ledger.json`. Each record passes
  `check_records` with 0 violations.
- **Departures from the card:** the card expected 0 difference classes and named no discrepancy tags; the Python's own
  pinned pair is reproduced instead, so the first record carries the three discrepancy tags (all in vocabulary 0.12)
  and the ledger file changed, which the card's file list did not name. `cases.rs`'s model-step arm was edited by an
  inline Python rewrite, and this result section was first appended by a heredoc: two multi-line edits outside the
  Edit tool. `cargo fmt` reflowed `x87.rs` after the first gates
  run stopped at the format check. The second gates run stopped at the planted discrepancy chain: "gates: the planted
  chain holds 34 DISCREPANCY_REPORTED, not 32"; the 34 are the plant's 32 CF classes and FCOS and FSIN, which this
  profile now reports unplanted, so `scripts/gates.sh` and WORKING_AGREEMENT's count became 34, files the card did not
  name.
- **Left:** the SSE and VEX model cases, 35,663 in 142 classes; the VM's in-VM x87 paths (`_x87_fast`: FLD m64, FSTP
  m64, FSQRT) are the Python lowering's words, not ported.
