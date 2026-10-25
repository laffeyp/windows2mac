# Sprint 148 — The vector and x87 state: MXCSR, XMM, the YMM upper halves and the x87 registers judged

```yaml
---
id: 148
status: done
phase: 6
pass_kind: functional
---
```

## scope

The case runner's last judged fields (`docs/design-host-compile-path-2026-10-24.md`, link 3). Sprints 145 and 147
judged completed cases without their vector and x87 state, and faults without the YMM upper halves and the x87 state.
This sprint carries the rest of `scripts/vm_oracle.py` `judge_done` (143-155) and `judge_fault` (116-121):

- a completed case's MXCSR: the stored word with FPSR's cumulative flags folded in (`Vcpu.mxcsr`,
  `mxcsr_flags_from_fpsr`, `FPSR_TO_MXCSR` vm_backend 1117, 1146-1147);
- its XMM0-15, read from the vCPU (`Vcpu.vectors`; `hypervisor::Vcpu::simd`, Sprint 145);
- the YMM upper halves in the monitor data's slots, for format 3 results (`Vcpu.ymm_hi`), at completion and at a fault;
- the x87 state for format 4 results (`Vcpu.x87`): FCW from the monitor data, FSW, the abridged tags and ST0-ST7 from
  the state block, judged as `semantics_differential.judge_x87` (1009-1024) judges them, with FSW's condition codes
  that the class leaves undefined masked (`X87_UNDEFINED`, `C0`-`C3`, 997-1001); at completion and at a fault.

## prerequisites

- 147 (closed)

## context_files

- `scripts/vm_oracle.py` 116-157; `scripts/semantics_differential.py` 997-1024; `src/w2m/vm_backend.py` 1117,
  1146-1147, the `Vcpu` methods `mxcsr`, `ymm_hi`, `x87`, `vectors`
- `crates/w2m_vm/src/cases.rs`, `crates/cpu_cases`, `crates/hypervisor`

## signal contract

### Emits

None yet: the record comes at the skeleton's gate.

### Consumes

The case files and the downloaded Windows results (Sprint 144).

### Invariants

- No Python file added or changed.

## artifact contract

### Files created

- none

### Files modified

- `crates/w2m_vm/src/cases.rs`, `crates/w2m_vm/tests/case_runner.rs`, `WORKING_AGREEMENT.md`, `BLACKBOARD.md` (Sprint
  147's CI run, in the tail entry)

### Content assertions

- Each function cites the Python lines it carries.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values, from Windows:** in the 22 Intel runs, every judged case (completed or faulted) is also judged on
  the fields above, with zero difference classes. The Python class runs of 2026-10-21 accepted every class but FCOS
  and FSIN, whose differences were the lowering's; the run reports whether any FCOS or FSIN case reaches the judge or
  leaves to the model.
- **Planted fault:** FPSR's IDC folded into MXCSR's bit 2 in place of bit 1: the run names an MXCSR class.

## done criteria

Every case the runner judges agrees with Windows on its whole recorded state: GPRs, flags, MXCSR, XMM, the YMM upper
halves, the x87 registers and memory.

## self-review checklist

- [x] Scope names one verifiable property: the vector and x87 state agrees with Windows.
- [x] context_files cite the Python carried.
- [x] Emits: none yet, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: expected values from Windows, and a planted fault.
- [x] One concept: the vector and x87 state.

## result

- `crates/w2m_vm/src/cases.rs`: `mxcsr_flags_from_fpsr`, `vm_ymm_hi`, `vm_x87`, `x87_undefined`, `judge_x87` and
  `judge_ymm_x87`; `judge_done`'s part now judges MXCSR (FPSR read from the vCPU) and XMM0-15 (`Vcpu::simd`), and both
  judges the YMM upper halves and the x87 state when the Windows result holds them. The tally counts the cases judged
  whole, by iclass, and those with each format's state.
- Expected values (`tests/case_runner.rs`, the 22 Intel runs): zero difference classes on every run, with MXCSR and
  XMM judged on all 173,776 cases, the YMM upper halves on 43,919 (sets 049 and 049b), and the x87 state on 5,960
  (sets 065, 065b, 065c). No FCOS or FSIN case reaches the judge: each leaves to the model, counted, not judged.
- Planted fault: FPSR's IDC folded into MXCSR bit 2 in place of bit 1. The run named MXCSR classes, the first "ADDPD
  MXCSR: case 25928: vm 0x1fa4, cpu 0x1fa2 (in 0x1f80)". Restored, the test passes.
