# Sprint 178 — The model's legacy SSE slice

```yaml
---
id: 178
status: active
phase: 6
pass_kind: functional
---
```

## scope

Step 4's model (census row 4), its fourth slice. After Sprint 175 the case runner counts, and does not judge, 35,663
model cases in 142 classes, every one an `sse`, `vex`, `vexs` or `fma` operation, which `model_step` refuses. This
slice takes the legacy-encoded ones, 6,845 cases: set `semantics`' 40 classes (2,085 a run, two runs), set 049's
CMPSD_XMM, CVTPD2DQ and CVTTPD2DQ (425), set 050's 23 classes (3,513) and set 065c's MPSADBW (822). The VEX-encoded
26,733 (sets 049 and 049b) come after.

- **The count and the route first:** `store/case_runner/expected-by-class.tsv` gives the 67 legacy classes counted to
  the model (8,930 cases over every run, 6,845 with set `semantics` once); each class's route to the model is read
  from `rules/_sse.py` before the code, and the card's result lists them by route (reviewer-2's High 2, fix 3). Read
  2026-10-28: every legacy route is `hvc #10` (`_slow_sse`), of two kinds: a lowered class whose words leave the fast
  path while MXCSR unmasks an exception (`FP_CLASSES`, `CMP_LOWERED`, the four packed conversions; `_sse.py` 24-28,
  330-335), and a class the lowering always sends (CMPSD_XMM, HADDPD, MPSADBW, `SSE_050`, PCMPISTRI, the RCP and RSQRT
  approximations; 182-186, 425-426). The first kind serves the batch run only while a guest unmasks; the second always.
- **The model:** `runtime.py` `_sse` (398-497) and what it calls for these classes (`_sse_049b` 498-573 for PALIGNR,
  CVTDQ2PS, CVTPS2PD and CVTTPS2DQ, `_sse_050` 574-667, `_sse2_int`
  829-, `_xread`, `_xwrite`), `src/w2m/x86fp.py` (560 lines, read in full before the code: the IEEE operations on bit
  patterns, MXCSR's rounding, DAZ, FTZ and its six flags, and `SimdFault` for an unmasked exception, which `_exec`
  turns into a guest fault), `approximate.py` and `string_compare.py` where a class reaches them.
- **The arithmetic:** x86fp computes in `Fraction` (read 2026-10-28: `value`, `Env.round`, `_sqrt_round`), so the port
  uses the `BigRational` Sprint 175 brought, as `x87.rs` does.
- **The lifting:** the translator lifts these classes to `Op::Sse` today, and `model_step` refuses that op; the model
  steps it instead.

## prerequisites

- 177 (closed)

## context_files

- `src/w2m/x86fp.py` (whole), `src/w2m/runtime.py` 398-497, 574-667, 829-, the `_exec` fault path (995-1013)
- `crates/w2m_vm/src/model.rs`, `crates/w2m_vm/src/x87.rs`, `crates/w2m_vm/src/cases.rs`, `crates/translator/src/lift.rs`
  (`Op::Sse`)

## signal contract

### Emits

- On four `case-runner-rust` records: SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED,
  SESSION_COMPLETED, now judging the legacy SSE classes' model cases; FCOS's and FSIN's CANDIDATE_REJECTED,
  REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED as Sprint 175 left them.

### Invariants

- No Python file added or changed; every case judged before is judged as before; FSIN and FCOS stay at their pins.

## artifact contract

### Files modified

- `crates/w2m_vm/src/model.rs`, `crates/w2m_vm/src/cases.rs`, `crates/w2m_vm/src/x86fp.rs` (new, as `x86fp.py` is its own module),
  `store/case_runner/expected.tsv`, `store/case_runner/expected-by-class.tsv`, `WORKING_AGREEMENT.md`, `docs/step-4-census-2026-10-28.md` (row 4),
  four records

### Command exit codes

- the case runner and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the 6,845 legacy SSE model cases judged against Windows with 0 difference classes;
  `expected.tsv`'s "model" falling by exactly the count stepped.
- **Planted faults:** each on a path the judge reads, each checked against the Python's code before this card's commit:
  (1) MXCSR's DAZ ignored on a source operand; (2) one comparison predicate of CMPSS taken as another (LT for LE).
  Each must fail the case runner and name its class. Checked: `scripts/semantics_differential.py` 163-165 sets DAZ in
  a quarter of the cases' MXCSR and 177 seeds the operands with edge values, denormals among them, and the judge reads
  each XMM register and MXCSR (975-976); LT and LE differ on equal operands, which the edge values give.

## done criteria

The legacy-encoded SSE model cases are judged against Windows through the Rust model.

## self-review checklist

- [x] Scope names one verifiable property: the legacy SSE model cases judged.
- [x] context_files cite the Python to read in full first.
- [x] Emits: the case runner's records, the open pair unchanged.
- [x] Artifact contract gradable by command.
- [x] Observation contract: counts and two plants on paths the judge reads.
- [x] One concept: the legacy SSE slice.
