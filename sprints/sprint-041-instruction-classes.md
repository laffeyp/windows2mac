# Sprint 041 — instruction classes as item runs

```yaml
---
id: 041
status: active
phase: 4
pass_kind: functional
---
```

## scope

The lifter is one file of rules, the VM compiles none of the 47 legacy-SSE classes the model gained in Sprint 039 (17,344 of 039's cases wait on it), and every instruction class has been judged inside a hand-run differential, not on its own record. This sprint makes the class the unit of work (`docs/plan.md` Phase 4).

1. **Vector state in the VM.** `vm_monitor` gets and sets Q0-Q15, FPCR and FPSR (`hv_vcpu_{get,set}_simd_fp_reg`, `HV_REG_FPCR/FPSR`). XMM i lives in V i. MXCSR lives in the monitor data page; FPCR follows it: AH=1 always (Sprint 039: x86's NaN, MIN/MAX, tininess and denormal-input behavior), RMode from RC, FIZ from DAZ, FZ from FTZ.
2. **SSE lowering** for the 47 classes: moves, logic, ADD/SUB/MUL/DIV/SQRT/MIN/MAX in SS/SD/PS/PD, the conversions (FCVTZS fixed up to the integer indefinite 0x80000000 on NaN and out-of-range, which FPCR.AH does not provide), COMISS/UCOMISS to ZF PF CF. Exception flags: FPSR's IOC DZC OFC UFC IXC IDC become MXCSR's IE ZE OE UE PE DE. When MXCSR unmasks any exception, the instruction leaves the fast path and the host executes it with `x86fp` (the model the CPU judged in Sprint 039) and writes the state back; Windows's default MXCSR 0x1F80 masks all six, and the count of slow-path instructions is on the record.
3. **The VM oracle compares XMM and MXCSR**, on every 039 case the VM compiles.
4. **One rule per class.** `src/w2m/rules/` holds one module per instruction class (its lifting rule and, where the class needs more than the shared operations, its AArch64 lowering), registered by iclass; `lift.py` dispatches to them. Behavior is unchanged: the 039 verdicts on model and VM must hold before and after.
5. **A class is accepted by an item run** (`scripts/class_run.py`): one substrate run per class with a trigger chain — the rule is built, the class's oracle cases run in the model and the VM against the recorded CPU, COMPARISON_COMPLETED per executor causes either CANDIDATE_ACCEPTED or DISCREPANCY_REPORTED → DISCREPANCY_ROUTED. A process pool runs classes in parallel.
6. **The next block:** IMUL, CMOVcc, SHL/SAR/ROL/ROR, and the LOCK-prefixed atomics (XADD, CMPXCHG, ADD/SUB/AND/OR/XOR/INC/DEC with LOCK), chosen by frequency in the game's code (Sprint 036), get rules, cases in a new case file (its class set frozen, KIT_DIARY #23), Intel runs, and item runs.

## prerequisites

- 040 (VM state, oracle in the VM), 039 (FP model, Intel results), 036 (class frequencies)

## context_files

- `src/w2m/lift.py`, `src/w2m/runtime.py`, `src/w2m/x86fp.py`, `src/w2m/vm_backend.py`, `src/w2m/a64_encoder.py`, `tools/vm_monitor/vm_monitor.c`, `scripts/vm_oracle.py`, `scripts/semantics_differential.py`, `src/w2m/record.py`
- `store/runs/capability-fp-1791593555694176000` (FPCR.AH results), Arm ARM DDI 0487: FPCR (AH, FIZ, FZ, RMode), FPSR, FCMP/FCMPE NZCV, FCVTZS/FCVTNS/FCVTMS/FCVTPS, SCVTF, FCVT, FMIN/FMAX, FSQRT, LDR/STR (SIMD&FP), INS, EOR/AND (vector)
- Intel SDM Vol 1 §10.2.3 (MXCSR), §11.5; Vol 2 pages of each class

## signal contract

### Emits

- per class item run: SESSION_INIT, CANDIDATE_GENERATED (the rule's source and its sha256), COMPARISON_COMPLETED (one per executor: model, managed_vm), CANDIDATE_ACCEPTED or DISCREPANCY_REPORTED + DISCREPANCY_ROUTED, SESSION_COMPLETED; each caused by the event before it.
- VM oracle runs: as Sprint 040, now with XMM and MXCSR.

### Invariants

- Vocabulary 0.7, no change expected. The reference is the CPU's result file.

## artifact contract

- created: `src/w2m/rules/` (package, one module per class), `scripts/class_run.py`, `tests/test_class_runs.py`, a new case file's generator entry and its Intel results in `store/probe-runs/`
- modified: `tools/vm_monitor/vm_monitor.c`, `src/w2m/vm_backend.py`, `src/w2m/a64_encoder.py`, `src/w2m/lift.py`, `src/w2m/runtime.py`, `scripts/vm_oracle.py`, `scripts/semantics_differential.py`, tests
- File count over 10 (project override): the class is the concept; the package is one file per class by design.
- `scripts/gates.sh` → 0

## observation contract

- **VM SSE:** on both Intel runs, every 039 SSE case the VM compiles agrees in XMM, MXCSR, GPRs and flags, faults included; the number of slow-path executions is stated.
- **Planted faults** (reach counted first, WORKING_AGREEMENT): FCVTZS without the indefinite fix-up (changes a GPR on the 37 register-source CVTT* cases of 039 whose input is NaN, infinite or out of range); FPCR.AH left 0 (changes XMM on the 117 register MIN/MAX cases of 039 with a NaN lane); IDC not mapped to DE (changes MXCSR on the 105 register ADD/SUB/MUL/DIV cases of 039 with a denormal lane and DAZ clear). Each caught by the CPU's results alone.
- **Rules per class:** the 039 verdicts are unchanged on model and VM after the split (0 differences on both Intel runs).
- **Item runs:** every class in the 039 set and the new block has its own record; the chain on each record is causal (`explain_producer` shows CANDIDATE_ACCEPTED caused by the comparisons); a planted wrong rule produces DISCREPANCY_REPORTED and ROUTED on its record.
- **Next block:** the new classes agree with an Intel CPU on every case of the new case file, or each difference is reported, routed and fixed.

## done criteria

Every instruction class w2m lifts is a module with its own accepted record judged by an Intel CPU, the VM executes SSE as the CPU does, and the next block by frequency is in.
