# Sprint 039 — the floating-point and fault oracle

```yaml
---
id: 039
status: closed
phase: 3
pass_kind: functional
---
```

## scope

Sprint 037's oracle compares GPRs, five flags and two memory windows; it never sees XMM registers, MXCSR, AF or DF, and every one of its cases completes. Review 2026-10-09 (progress) N3 and N4 name the gap: SSE and AArch64 differ on NaN choice in MIN/MAX, float-to-integer overflow, denormal handling and underflow detection; and a fault inside an instruction must leave no partial store. Hail to the Rainbow's code is SSE-heavy (MOVAPS, MOVSS, MOVUPS and MULSS are among its most frequent classes, Sprint 036). This sprint extends the oracle to that state and to faults, gives the model the floating-point semantics it judges, and probes the AArch64 side's FPCR.AH.

1. **Probe format 2** (`probes/semantics/semantics.c`; Sprint 037's format-1 files stay judged by `judge_format1`): a case adds MXCSR, XMM0-15 and a 32-byte edge window at the end of the data page (0x3000_0FE0; the next page is unmapped); RFLAGS also carries AF and DF. A result adds MXCSR, XMM0-15, the edge window, and for a fault the faulting RIP and the CONTEXT's GPRs, RFLAGS, MXCSR and XMM registers, so the state at a fault is compared as well as the code. The harness saves and restores XMM6-15 and MXCSR (Windows x64 nonvolatile).
2. **Model state:** `Thread` gains `xmm` (16 × 128 bits), `mxcsr` (0x1F80 at reset), `df`; flags gain `A`. AF is computed for ADD, SUB, CMP, INC, DEC (bit 4 of a ^ b ^ r, SDM Vol 1 3.4.3.1) and undefined where the SDM leaves it undefined (logic, shifts, MUL). CLD and STD are lifted.
3. **`src/w2m/x86fp.py`:** IEEE 754 binary32 and binary64 arithmetic under MXCSR, exact (rational arithmetic, then one rounding in the mode RC selects): ADD, SUB, MUL, DIV, SQRT, MIN, MAX, the conversions to and from integers and between widths, ordered and unordered compares; DAZ and FTZ; the six exception flags; SSE's NaN rules (SDM Vol 1 Table 4-7: the first source's NaN, quieted, wins; MIN and MAX return the second source on any NaN or on two zeros); the integer indefinite for out-of-range conversion; masked versus unmasked exceptions (an unmasked one faults and leaves the destination unchanged).
4. **Operand templates for SSE rows** (`specs/x86_decode.json` 0.7.1): V, W and U notation with ss, sd, ps, pd, d, q and y sizes (SDM Vol 2D Appendix A, Table A-1), transcribed from each instruction page's Opcode/Instruction column, for the legacy-SSE classes the model now lifts: MOVSS, MOVSD_XMM, MOVAPS, MOVUPS, MOVAPD, MOVUPD, MOVD, MOVQ, XORPS, XORPD, ANDPS, ANDPD; ADD, SUB, MUL, DIV, SQRT, MIN, MAX in SS, SD, PS and PD; CVTTSS2SI, CVTTSD2SI, CVTSS2SI, CVTSD2SI, CVTSI2SS, CVTSI2SD, CVTSS2SD, CVTSD2SS; UCOMISS, UCOMISD, COMISS, COMISD. `operand_differential` judges the new templates against XED.
5. **Cases:** XMM lanes drawn from floating-point boundaries (±0, smallest and largest denormal, smallest normal, largest finite, ±infinity, quiet and signalling NaNs with payloads, the integer-conversion edges ±2^31, ±2^63 and their neighbours, values that round differently per mode) and uniform bits; MXCSR with random RC, DAZ, FTZ and sticky flags, masks all set in most states and random in the rest; legacy-SSE memory operands placed both aligned and misaligned. Fault cases: one state per lifted encoding with a memory operand, its access straddling the end of the data page.
6. **The judge** compares, in addition: XMM0-15, MXCSR, AF (where the model defines it), DF, the edge window, and for faults the fault code, the faulting RIP and the whole state at the fault.
7. **Intel verdicts:** `scripts/probe_on_intel.py` dispatches `windows-probe.yml` and repeats until the runner's CPU is Intel (up to 8 dispatches), so a verdict that needs Intel's behaviour comes from Intel; every run records its CPU.
8. **FPCR.AH probe** (`probes/mac_hv_fp/hv_fp.c`): in a Hypervisor.framework guest at EL1, read ID_AA64MMFR1_EL1.AFP, set FPCR.AH and FIZ and read FPCR back, then run FMIN and FMAX on NaN and signed-zero operands, an underflowing multiply whose result rounds to the smallest normal, and FCVTZS on out-of-range values, with AH 0 and 1; record each result beside the x86 rule it must match.

## prerequisites

- 037 (the oracle), 036 (spec 0.7.0)

## context_files

- `probes/semantics/semantics.c`, `scripts/semantics_differential.py`, `src/w2m/lift.py`, `src/w2m/runtime.py`, `src/w2m/x86ops.py`, `specs/guest_state.json` 0.2.0
- Intel SDM Vol 1 §4.8 (floating-point formats, Table 4-7 NaN rules), §10.2.3 (MXCSR), §11.5 (SIMD floating-point exceptions); Vol 2 instruction pages (cached at `build/sdm_pages/`, felixcloutier transcription); Vol 2D Appendix A Table A-1
- Arm ARM (DDI 0487) FPCR.AH, FPCR.FIZ, ID_AA64MMFR1_EL1.AFP; Microsoft Learn: CONTEXT (x64), AddVectoredExceptionHandler

## signal contract

### Emits

- harness plane: SESSION_INIT, COMPARISON_COMPLETED, DISCREPANCY_REPORTED/ROUTED per class, SESSION_COMPLETED (as 037); CAPABILITY_PROBED per FPCR check (six new `capability` values: vocabulary 0.7, P17, added when the probe showed what it decides).

### Invariants

- Vocabulary 0.7 (P17, an enum extension). The reference is the CPU's result file; the judge reads no model code to produce it.

## artifact contract

### Files created / modified

- created: `src/w2m/x86fp.py`, `tests/test_x86fp.py`, `scripts/probe_on_intel.py`, `probes/mac_hv_fp/hv_fp.c`, `scripts/probe_fp_control.py`
- modified: `probes/semantics/semantics.c`, `probes/semantics/cases.bin` (format 2, built on the runner from the seed by `probe.json` `generate`, sha256 recorded; not committed; format 1 moves to `store/probe-runs/semantics-037-cases.bin`), `scripts/semantics_differential.py`, `src/w2m/lift.py`, `src/w2m/runtime.py`, `src/w2m/kernel32.py` (Thread), `src/w2m/x86ops.py`, `specs/x86_decode.json`, `tests/test_semantics_differential.py`

### Command exit codes

- `scripts/gates.sh` → 0

## observation contract

### Expected runtime signals

- Planted faults, each reported by the CPU's results alone: MINPS returning the first source when the second is NaN (x86 returns the second); CVTTSS2SI saturating to 0x7FFFFFFF on overflow (x86 returns 0x80000000); AF taken from bit 3 instead of bit 4 (a model that calls a defined flag undefined is invisible to this judge, which compares only flags the model defines; recorded as a finding); a straddling store committing its first bytes before the fault.
- Sprint 037's results still agree on Intel with AF now compared (AF had been ignored).
- FPCR probe: AFP present on this Mac (the host reports `hw.optional.arm.FEAT_AFP: 1`); with AH 1, FMIN on (number, NaN) returns the NaN's second operand as x86 does, or the record says otherwise.
- Every discrepancy class is reported and routed; real model defects are fixed in this sprint and the run repeated.

## done criteria

The model agrees with an Intel CPU on every defined output of every case, including XMM, MXCSR, AF, DF and the state at faults, or each disagreement is reported, routed and fixed; the four planted faults are caught; the FPCR probe's results are on the record.

## self-review checklist

- [x] Scope concrete. [x] Sources named. [x] Vocabulary change recorded (0.7, P17). [x] Observation contract states outcomes.

## observed
- **Operand templates:** `scripts/sse_operand_templates.py` transcribed 62 legacy-SSE rows from the cached SDM pages (spec 0.7.1). `operand_differential` over Hail to the Rainbow's three PE files: operands equal XED's at all 4,752,945 compared positions.
- **x86fp before any CPU:** 20,000 random binary64 cases against Python floats and 20,000 binary32 cases against NumPy 2.5.3 (round to nearest, finite results): bit-identical. NumPy entered as a dev dependency.
- **Case file (format 2):** 39,708 cases over 107 iclasses (Sprint 037: 20,116 over 53), 3,333 fault placements, 23,824,812 bytes. The runner builds it from the seed (`probe.json` `generate`) and records its sha256; every run's sha256 equals the local build's.
- **First runner run 38010542937 crashed** (0xC0000005, unhandled): a fault placement for `[rsp+rbp*2+disp]` moved RSP out of the stack, and Windows dispatches an exception on the faulting thread's stack. The generator now drops cases that move RSP out of the stack mapping when the model predicts a fault (41), and the probe flushes each result.
- **The CPU corrected the model three times** (AMD EPYC 7763, run 38010775429; each rule now cites the run):
  1. Windows delivers #XM as STATUS_FLOAT_INVALID_OPERATION (IE, and DE), DIVIDE_BY_ZERO, OVERFLOW, UNDERFLOW or INEXACT_RESULT, chosen by the first unmasked flag in the order IE, ZE, DE, OE, UE, PE, not as STATUS_FLOAT_MULTIPLE_TRAPS (1,056 faults).
  2. An unmasked pre-computation exception (IE, DE, ZE) leaves only the pre-computation flags in MXCSR.
  3. With OE or UE unmasked, PE reports whether rounding to the format's precision with an unbounded exponent was inexact, the IEEE 754 trapped-exception rule. A first guess that unmasked UE suppresses PE was wrong (19 cases) and was replaced.
- **Agreement:** after those rules, **Intel Xeon Platinum 8573C (run 38011862284) and Intel Xeon Platinum 8370C (run 38011858047): the model agrees with the CPU on all 39,708 cases**, comparing GPRs, CF PF AF ZF SF OF where defined, DF, MXCSR, XMM0-15, three memory windows, and for 5,485 faults the code, the faulting RIP and the state at the fault. 18 AMD runs (EPYC 7763, 9V74, 9V45): every case agrees except the 128 `0F 0D` register forms, which fault on AMD (Sprint 037's class). No case differed across the three executions of any run. Records: `store/runs/semantics-17915948*` (5 runs: 2 Intel, 3 AMD models). Full results in the private release `semantics-039` (15 files).
- **Intel draws:** 2 of 21 jobs ran on Intel, both on windows-2025; windows-2022 gave 0 of 6. `scripts/probe_on_intel.py` dispatches batches of 6 across both images.
- **Sprint 037's results with AF compared:** Intel and AMD still agree (every 10th case); the model's AF rule holds on 20,116 more cases.
- **Planted faults, caught by the CPU's results alone** (`tests/test_semantics_differential.py`): MIN returning the first source on a NaN second source (XMM differs); CVTTSS2SI saturating to 0x7FFFFFFF (GPR differs); AF from bit 3 (AF differs); a straddling store writing its mapped bytes before the fault (edge window differs).
- **Finding: a blind spot in the judge.** A flag the model calls undefined is not compared, so a model that wrongly calls a defined flag undefined passes. The card's planted fault "AF undefined after ADD" could never be caught and was replaced. The undefined set per class is to be checked against the SDM's flag table in each class's item run (Sprint 041).
- **FPCR probe** (`scripts/probe_fp_control.py`, record `store/runs/capability-fp-1791593555694176000`, M5 Max guest): FEAT_AFP present (ID_AA64MMFR1_EL1 0x100011312000, AFP 1); FPCR keeps AH, FIZ and NEP. With AH=1, x86's answer in value and flags for MIN/MAX on NaNs and signed zeros (16 runs), the default NaN 0xFFC00000 and first-operand NaN priority (4), tininess after rounding (4: AH=0 sets UFC on a result that rounds up to the smallest normal, AH=1 does not), and denormal inputs (IDC as DE; FIZ as DAZ). FCVTZS does not match under any FPCR: 0x7FFFFFFF on overflow and 0 on NaN, where x86 returns 0x80000000.
- Vocabulary 0.7 (P17: six `capability` values).
- Two items differ from the card: the probe reads only format 2 (Sprint 037's format-1 files are judged in Python by `judge_format1`), and the AF plant changed as above.
- Gates: `scripts/gates.sh` 0 (258 tests). The suite first failed under load: each format-2 test rebuilt the 39,708 cases and passed 30 s with the Windows VM on 8 cores; the tests now build them once per session (module 89 s to 13 s).
