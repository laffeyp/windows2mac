# Sprint 037 — the semantics oracle: single instructions on the runner's CPU judge the lifter

```yaml
---
id: 037
status: open
phase: 3
pass_kind: functional
---
```

## scope

`vm_backend.lockstep` and `state_differential` compare the AArch64 back end with `runtime.Runtime`; both execute `lift.py`'s operations, so a wrong lifting rule passes both (review 2026-10-09 H3; the 2026-10-08 decoder-theory Decision item 4, not yet done). This sprint builds the independent check: the x86 CPU on the windows-2025 runner executes each instruction from a stated state, and the model's result for the same instruction and state is compared field by field.

1. `scripts/semantics_differential.py --cases` writes `probes/semantics/cases.bin`: for every spec row whose encoding the lifter handles, the `encgen` cases (seeded) that decode and lift, each from several random states (16 GPRs drawn from width and sign boundaries and uniform bits; CF, PF, ZF, SF, OF set at random, the others 0; a 128-byte data window and a 128-byte stack window of random bytes). A memory operand's base register is set so its effective address falls in the data window (index register 0, as `state_differential` does); RIP-relative and absolute-address forms are counted and skipped. RSP points into the stack window. Control flow uses fixed encodings so both paths are observable: Jcc rel8 +14 for all 16 conditions, JMP rel8 +14, CALL rel32 +14, RET to a planted address.
2. `probes/semantics/semantics.c` (C with an assembly entry and exit): maps code at 0x2_0000_0000, data at 0x3_0000_0000, stack at 0x4_0000_0000 (the addresses the model uses); per case writes the instruction followed by `jmp [rip]` to a not-taken collector and, 14 bytes on, to a taken collector; loads RFLAGS and all 16 GPRs and jumps to the instruction; the collector stores every GPR, RFLAGS and which path ran without touching the case's stack, restores the harness, and the harness copies out both windows. A vectored handler turns a fault into a recorded outcome instead of a dead process.
3. `windows-probe.yml` gains mode `file` (input file in, result file out).
4. `scripts/semantics_differential.py --judge` runs each case's instruction in `runtime.Runtime` (one step, same addresses) and compares RIP (by path), the 16 GPRs, each flag the model defines (an undefined flag is not compared), and both windows; `--record` writes one substrate run: COMPARISON_COMPLETED (contract `x86.semantics`, relation `exact`, reference the runner's CPU), DISCREPANCY_REPORTED and DISCREPANCY_ROUTED per (iclass, field) class, routed `implementation_repair` (the CPU is the reference; the lifter is wrong).

## prerequisites

- 036 (the decoder the cases decode with)

## context_files

- `src/w2m/lift.py`, `src/w2m/runtime.py`, `src/w2m/vm_backend.py` (`state_differential`), `src/w2m/encgen.py`; Intel SDM Vol 1 3.4.3 (EFLAGS), Vol 2 instruction pages for flag effects; Microsoft Learn: VirtualAlloc, FlushInstructionCache, AddVectoredExceptionHandler

## signal contract

### Emits

- harness plane: SESSION_INIT, COMPARISON_COMPLETED, DISCREPANCY_REPORTED/ROUTED per class, SESSION_COMPLETED.

### Invariants

- No vocabulary change. The judge reads no code from `lift.py` to produce the reference: the reference is the CPU's output file.

## artifact contract

### Files created / modified

- created: `probes/semantics/semantics.c`, `probes/semantics/probe.json`, `probes/semantics/cases.bin` (derived), `scripts/semantics_differential.py`, `tests/test_semantics_differential.py`, `store/probe-runs/<run>/`
- modified: `.github/workflows/windows-probe.yml`

### Command exit codes

- `scripts/gates.sh` → 0

## observation contract

### Expected runtime signals

- Runner: every case completes (no fault outcome); results are identical across the 3 runs except where an instruction's result is undefined by the SDM.
- Judge: whatever the record shows. A planted lifter fault (SUB's carry computed as signed, Sprint 030's planted fault) must be reported by this check alone, without lockstep.
- Every discrepancy class is reported and routed; any real lifter defect found is fixed in this sprint and the run repeated.

## done criteria

Every instruction form the lifter handles has been executed on a real x86 CPU from random states and the model agrees on every defined output, or each disagreement is reported, routed and fixed.

## self-review checklist

- [x] Scope concrete. [x] Sources named. [x] No new vocabulary. [x] Observation declared before running. [x] File count recorded (7).

## observed
