# Sprint 182 — The VEX translator group

```yaml
---
id: 182
status: active
phase: 6
pass_kind: functional
---
```

## scope

Decision "Sequence after Sprint 181": the batch run's last model exits the Rust cannot serve are VCVTSS2SI 128 and
VCVTSI2SS 128 (hvc 10), VEX classes the Rust does not lift. `rules/_vex.py` puts both in `SLOW` (57-59): the Python
always sends them to the model. The C words census holds 719 VEX calls not lifted (`store/census/c-words-unlifted-
2026-10-28.tsv`).

- **The count first:** the census's VEX classes listed with `_vex.py`'s route for each: `BASE` and lowered natively
  through the base's lowering, `SLOW` or a 256-bit form outside `NATIVE_256` to `hvc #10`, `CROSS` (`vexs`) to `hvc #10`,
  FMA scalar natively behind `_fp_guard`, packed to `hvc #10`, VZEROUPPER natively.
- **The lifting:** `_vex.py`'s `rule` (67-107) in Rust: `Op::Vex { base, d, s1, s2, vl, align, imm }`, `Op::Vexs`,
  `Op::Fma`, `Op::Vzeroupper`, with `BASE`, `CROSS`, `CROSS_AT_256`, `FMA`, `L0_ONLY`, `ALIGNED`.
- **The lowering:** `_vex.py`'s `lower` (136-207) and `_lower_fma` (210-228), word for word with the C emitter; the
  model ops lowered to `hvc #10`.
- **The model:** `Op::Vex`, `Op::Vexs` and `Op::Fma` refused by name in the model's step (`CompileError` is the
  lowering's; the model's is `Fault::Refused`), so the case runner counts their cases as now; the model's VEX step is
  the next card.

## prerequisites

- 181 (closed)

## context_files

- `src/w2m/rules/_vex.py` (whole), `src/w2m/lift.py` (the `vex`, `vexs`, `fma` ops and their `commit_order` arms),
  `src/w2m/vm_backend.py` (`_temps_read` and the flag sets for the four ops)
- `crates/translator/src/lift.rs`, `rules.rs`, `lower.rs`, `crates/translator/tests/c_words.rs`

## signal contract

### Emits

- On four `case-runner-rust` records: the verdicts Sprint 181's records hold.

### Invariants

- No Python file added or changed; every call compared before compares as before.

## artifact contract

### Files modified

- `crates/translator/src/lift.rs`, `rules.rs`, `lower.rs`, `store/census/c-words-unlifted-2026-10-28.tsv`
- `crates/w2m_vm/src/model.rs` (the refusal), `store/case_runner/rust-compiled-by-class.tsv` and
  `rust-compiled-codes-by-class.tsv`, `WORKING_AGREEMENT.md`, `docs/step-4-census-2026-10-28.md`; four records

### Command exit codes

- `c_words`, the case runner and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the census's VEX calls move from "not lifted" to compared with no difference; the case runner's
  judged counts unchanged, its compiler-of-record files gaining the VEX classes the Rust now compiles.
- **Planted faults:** each checked against the Python and the record before this card's commit: (1) VCVTSS2SI, a `SLOW`
  class, lowered through CVTSS2SI's base lowering instead of to `hvc #10`: `c_words` names VCVTSS2SI (the census holds
  its one call, the site of the batch run's 128 exits; checked 2026-10-28, the record's 722 VEX calls hold no 256-bit
  form outside `NATIVE_256`, so the first draft's plant on that path could not fail); (2) the upper half not
  cleared after a 128-bit VEX write (`_clear_upper`): `c_words` names a word difference.

## done criteria

The batch run's VEX calls lift and lower word for word with the C emitter.

## self-review checklist

- [x] Scope names one verifiable property: the VEX calls compared.
- [x] context_files cite the Python to read in full first.
- [x] Emits: the case runner's records.
- [x] Artifact contract gradable by command.
- [x] Observation contract: counts and two plants.
- [x] One concept: the VEX translator group.
