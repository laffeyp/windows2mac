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

## result

- **The state:** `Thread` gains `ymm_hi` and two more XMM slots, 16 and 17, `_vex`'s scratch, zeroed after every
  step; `model_step` reads YMM bits 255:128 from MONITOR_DATA + 0x400 and writes them back after a step, leaving the
  page as it was on a fault (vm_backend 4282-4283, 4315-4316); `step` restores `ymm_hi` with the rest of the thread.
- **The step:** `Step::vex`, `vex_cross` and `fma` port `_vex` (732-786), `_vex_cross` (668-730) and `_fma` (788-827);
  `defer_fp` and `deferred_flags` carry `_defer_fp`, which `fp_check` and the FP arm's `execute_unchecked` honour;
  VZEROUPPER clears `ymm_hi`. `_sse`'s CMPSD_XMM_VEX arm (predicate imm8[4:0]) was missing from `sse` and was added.
- **The arithmetic:** `x86fp.rs` gains `fma`, `fma_lane` (419-467) and `execute_unchecked` (`execute`'s
  `check=False`), which `execute` now calls.
- **The case runner:** set 049's model count fell from 4,655 to 367 and set 049b's from 22,445 to 1,258, 25,475 VEX
  model cases judged against Windows with 0 difference classes; the model steps 111,604 cases in all. Still counted,
  not judged, by class: CMPSD_XMM 367 (set 049; the Rust rules do not lift it), VRCPPS 648 and VRSQRTPS 610 (set
  049b; the model refuses RCPPS and RSQRTPS until `approximate.py` is ported), and set 050's 15 classes and
  MPSADBW as before. The first run panicked in `xread` on a 32-byte VEX.256 memory source; it now reads all 32 bytes
  and keeps the low 16, as `_xread` and the 128-bit step do.
- **Plant 1** (`hi[d] = 0` dropped after a VEX.128 register write): the case runner failed, "7 VADDSD ymm0.hi: case
  2155: [c4, a1, 27, 58, 83, a1, b9, 9f, 0d] vm 3ef99a0a0e4d21480000000000000000, cpu
  00000000000000000000000000000000". Restored byte for byte.
- **Plant 2** (213's operands taken as 231's): the case runner failed, "6 VFMADD213SD xmm0: case 10994: [c4, a2, f9,
  a9, 85, 02, d3, 79, 47] vm cc08bf99007fffffbd96503cb31fffff, cpu cc08bf99007fffff4004000000000001". Restored byte
  for byte.
- **Records:** four `case-runner-rust` records (1793203177000000000, -030000000, -060000000, -090000000), 0
  violations each; the ledger holds 14 entries. A first set of four was written by a `case_runs` binary that plant 2's
  test build had relinked; those records were removed before any commit and the ledger restored.
- **Departures:** `sse`'s CMPSD_XMM_VEX arm and `xread`'s 32-byte read; every multi-line edit used the Edit tool.
