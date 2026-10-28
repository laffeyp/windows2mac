# Sprint 183 — The model's VEX step

```yaml
---
id: 183
status: active
phase: 6
pass_kind: functional
---
```

## scope

Decision "Sequence after Sprint 182": the batch run's VCVTSS2SI and VCVTSI2SS (256 of its model exits) lift since
Sprint 182 and reach `hvc #10` unconditionally (`rules/_vex.py` 57-60, 150), where the model refuses VEX ops. This
sprint gives the model its VEX step.

- **The state:** `Thread` gains YMM0-15 bits 255:128, carried in from and out to MONITOR_DATA + 0x400 by
  `model_step` as `_slow_sse` does (vm_backend 4282-4283, 4315-4316), restored on a guest fault as `_exec` restores
  `ymm_hi` (runtime.py 999-1010).
- **The step:** `runtime.py` `_vex` (732-786), `_vex_cross` (668-730) and `_fma` (788-827), with XMM 16 and 17 as
  `_vex`'s scratch and `_defer_fp`'s one check for both halves at VEX.256 (`_sse`'s FP arm with `execute(check=False)`,
  `_fp_check`'s deferral, 456-465, 983-986); VZEROUPPER clears the upper halves.
- **The arithmetic:** `x86fp.py`'s `fma` and `fma_lane` (419-467) ported into `x86fp.rs`.

## prerequisites

- 182 (closed)

## context_files

- `src/w2m/runtime.py` 398-497, 668-827, 980-1013; `src/w2m/x86fp.py` 419-467; `src/w2m/vm_backend.py` 4268-4323
- `crates/w2m_vm/src/model.rs`, `x86fp.rs`, `model_step.rs`

## signal contract

### Emits

- On four `case-runner-rust` records: SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED,
  SESSION_COMPLETED, now judging the VEX classes' model cases; FCOS's and FSIN's discrepancy tags as before.

### Invariants

- No Python file added or changed; every case judged before is judged as before.

## artifact contract

### Files modified

- `crates/w2m_vm/src/model.rs`, `x86fp.rs`, `model_step.rs`, `store/case_runner/expected.tsv`, `expected-by-class.tsv`,
  `WORKING_AGREEMENT.md`, `docs/step-4-census-2026-10-28.md` (row 4); four records

### Command exit codes

- the case runner, the run-loop judges and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** sets 049 and 049b's VEX model cases (26,733 counted after Sprint 178, VCVTSS2SI 522 and
  VCVTSI2SS 835 among them) judged against Windows with 0 difference classes, `expected.tsv`'s "model" falling by
  exactly the count stepped; what the model still refuses named by class.
- **Planted faults:** each checked against the Python and the case sets before this card's commit: (1) a VEX.128
  register write keeping its upper half (`hi[d] = 0` dropped): the case runner names a YMM upper-half difference; (2)
  FMA's 213 order taken as 231: the case runner names a VFMADD213 class. Checked: every case the case runner judges in
  sets 049 and 049b is judged with its YMM upper halves (19,679 and 24,525 at Sprint 182, `judge_ymm_x87`); set 049
  holds 75 VFMADD213SD and set 049b 97 VFMADD213SS model cases (`expected-by-class.tsv`).

## done criteria

Sets 049 and 049b's VEX model cases are judged against Windows through the Rust model.

## self-review checklist

- [x] Scope names one verifiable property: the VEX model cases judged.
- [x] context_files cite the Python to read in full first.
- [x] Emits: the case runner's records.
- [x] Artifact contract gradable by command.
- [x] Observation contract: counts and two plants.
- [x] One concept: the model's VEX step.
