# Sprint 039 — the floating-point and fault oracle

```yaml
---
id: 039
status: open
phase: 3
pass_kind: functional
---
```

## scope

Sprint 037's oracle compares GPRs, five flags and two memory windows; it never sees XMM registers, MXCSR, AF or DF, and every one of its cases completes. Review 2026-10-09 (progress) N3 and N4 name the gap: SSE and AArch64 differ on NaN choice in MIN/MAX, float-to-integer overflow, denormal handling and underflow detection; and a fault inside an instruction must leave no partial store. Hail to the Rainbow's code is SSE-heavy (MOVAPS, MOVSS, MOVUPS and MULSS are among its most frequent classes, Sprint 036). This sprint extends the oracle to that state and to faults, gives the model the floating-point semantics it judges, and probes the AArch64 side's FPCR.AH.

1. **Probe format 2** (`probes/semantics/semantics.c` reads formats 1 and 2): a case adds MXCSR, XMM0-15 and a 32-byte edge window at the end of the data page (0x3000_0FE0; the next page is unmapped); RFLAGS also carries AF and DF. A result adds MXCSR, XMM0-15, the edge window, and for a fault the faulting RIP and the CONTEXT's GPRs, RFLAGS, MXCSR and XMM registers, so the state at a fault is compared as well as the code. The harness saves and restores XMM6-15 and MXCSR (Windows x64 nonvolatile).
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

- harness plane: SESSION_INIT, COMPARISON_COMPLETED, DISCREPANCY_REPORTED/ROUTED per class, SESSION_COMPLETED (as 037); CAPABILITY_PROBED per FPCR check.

### Invariants

- No vocabulary change. The reference is the CPU's result file; the judge reads no model code to produce it.

## artifact contract

### Files created / modified

- created: `src/w2m/x86fp.py`, `tests/test_x86fp.py`, `scripts/probe_on_intel.py`, `probes/mac_hv_fp/hv_fp.c`, `scripts/probe_fp_control.py`
- modified: `probes/semantics/semantics.c`, `probes/semantics/cases.bin` (format 2; format 1 moves to `store/probe-runs/semantics-037-cases.bin`), `scripts/semantics_differential.py`, `src/w2m/lift.py`, `src/w2m/runtime.py`, `src/w2m/kernel32.py` (Thread), `src/w2m/x86ops.py`, `specs/x86_decode.json`, `tests/test_semantics_differential.py`

### Command exit codes

- `scripts/gates.sh` → 0

## observation contract

### Expected runtime signals

- Planted faults, each reported by the CPU's results alone: MINPS returning the first source when the second is NaN (x86 returns the second); CVTTSS2SI saturating to 0x7FFFFFFF on overflow (x86 returns 0x80000000); AF computed as undefined after ADD; a straddling store committing its first bytes before the fault.
- Sprint 037's results still agree on Intel with AF now compared (AF had been ignored).
- FPCR probe: AFP present on this Mac (the host reports `hw.optional.arm.FEAT_AFP: 1`); with AH 1, FMIN on (number, NaN) returns the NaN's second operand as x86 does, or the record says otherwise.
- Every discrepancy class is reported and routed; real model defects are fixed in this sprint and the run repeated.

## done criteria

The model agrees with an Intel CPU on every defined output of every case, including XMM, MXCSR, AF, DF and the state at faults, or each disagreement is reported, routed and fixed; the four planted faults are caught; the FPCR probe's results are on the record.

## self-review checklist

- [x] Scope concrete. [x] Sources named. [x] No new vocabulary. [x] Observation contract states outcomes.

## observed
