# Sprint 153 — The integer rest: link 5's second group, lifted and lowered in Rust

```yaml
---
id: 153
status: active
phase: 6
pass_kind: functional
---
```

## scope

Link 5's second group (`docs/design-host-compile-path-2026-10-24.md`): the rest of `src/w2m/rules/_int.py`'s rules
whose lowering is A64 (not the model), and `rules/bmi.py`, carried into `crates/translator` with the lowering their ops
need. By the record's calls outside group 1 (Sprint 151's census), INC 8,403, IMUL 4,744, SHR 4,668, CMOVcc 3,976, SHL
3,871, SETcc 2,189, SAR 1,483, DEC 1,219, BT 1,201, MOVSX 904, NOT 750 lead.

- **The rules** (`_int.py`, read in full in Sprint 151; `bmi.py`): `inc_dec` (INC DEC), `neg`, `not_`, `cdqe`, `cqo`,
  `cwde`, `cdq`, `movsx`, `bswap`, `xchg`, `sbb`, `adc`, `bit_scan` (BSF BSR), `bit_test` (BT BTC BTR BTS), `shr`,
  `shift` (SHL SAR ROL ROR), `mul`, `imul`, `setcc` (16), `cmov` (16), `xadd`, `cmpxchg`, `div`, `idiv`, `popcnt`,
  `movbe`, `leave`, and BMI's ANDN SHLX SHRX RORX MULX: 75 classes.
- **The ops they lift** beyond group 1: `alu` shr shl sar rol ror mul_lo mul_hi smul_lo smul_hi sext8 sext16 bswap
  popcnt adc sbb bsf bsr; `flags` inc dec adc sbb mul imul bt bsf popcnt andn and the shift kinds; `set`, `sel`, `cst`,
  `cstz`, `cmov`, `div`; the `ud` op MOVBE emits for a register operand.
- **The lowering** (`vm_backend.py`, read in full in Sprint 151): `compile_insn`'s branches for those ops (1585-1641,
  1687-1731, 1761-1784), `_set_flags`' other kinds (591-650), `_shift_flags` and `_shift_flags_const` (712-812), and
  the ops' places in `_op_flags`, `_temps_read`, `_operand_forms` and `commit_order`.

Out of scope (group 3): the `_LOCK` forms (the exclusive-pair lowering), CMPXCHG16B, and the system ops (`df`,
`cpuid`, `rep`, `fence`, `pause`, `mxcsr`, `fcw`, `xgetbv`, the model's `x86` steps).

## prerequisites

- 152 (closed)

## context_files

- `src/w2m/rules/_int.py`, `src/w2m/rules/bmi.py`; `src/w2m/vm_backend.py` as above
- `crates/translator` (Sprint 151); `crates/w2m_vm/tests/case_runner.rs`, `crates/translator/tests/c_words.rs`

## signal contract

### Emits

- The `case-runner-rust` sessions, four reference profiles (`harness/case-runs-rust.toml`): CANDIDATE_GENERATED,
  COMPARISON_COMPLETED and CANDIDATE_ACCEPTED per class, with group 2's classes now the Rust translator's candidates;
  SESSION_COMPLETED completed. The judges' ledger stays empty.

### Consumes

The stencil record, the case files and Windows results, the fixture.

### Invariants

- No Python file added or changed.

## artifact contract

### Files modified

- `crates/translator/` (`lift.rs`, `rules.rs`, `lower.rs`), `WORKING_AGREEMENT.md`, `BLACKBOARD.md` (Sprint 152's CI
  run, in the tail entry); four records under `store/runs/`

### Content assertions

- Each function cites the Python lines it carries.

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values, from the C emitter:** every record call of groups 1 and 2 the C compiles has Rust words equal to
  the C's.
- **Expected values, from Windows:** the case runner with the Rust translator judges each run's committed counts with
  zero difference classes; the classes it compiles grow from 38 to 38 plus group 2's classes present in the cases.
- **Expected values, from the Python words:** the third judge (Sprint 152) holds every case code the Rust rules lift to
  the fixture's words.
- **Planted faults:** `_shift_flags_const`'s CF for SHR read from bit `m` in place of `m - 1`; the C judge and the
  case runner name SHR classes.

## done criteria

Every integer instruction class the rules lower in A64 outside the locked and system forms is lifted and lowered in
Rust, equal to the C and the Python, and agrees with Windows.

## self-review checklist

- [x] Scope names one verifiable property: group 2 in Rust, equal to the C, the Python and Windows.
- [x] context_files cite the Python carried.
- [x] Emits: the sessions' tags.
- [x] Artifact contract gradable by command.
- [x] Observation contract: three judges and a planted fault.
- [x] One concept: the second group.

## result
