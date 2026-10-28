# Sprint 178 — The model's legacy SSE slice

```yaml
---
id: 178
status: done
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

## result

- **The routes:** read from `rules/_sse.py` before the code (the scope's note): every legacy route to the model is
  `hvc #10`, a lowered class leaving its fast path while MXCSR unmasks an exception, or a class the lowering always
  sends there.
- **The slice:** `crates/w2m_vm/src/x86fp.rs` carries `x86fp.py` function for function on `BigRational` (`fma` and
  `fma_lane` come with the VEX slice); two unit tests hold 1/3 in binary32 (0x3EAAAAAB, PE) and DAZ to the values
  `x86fp.py` prints. `model.rs` steps `Op::Sse` as `_sse` does, with `_sse_049b`, `_sse_050` and `_sse2_int`, the
  Python's `SSE2_INT` (62 classes), `SSE_049B`, `SSE_050_NEW`, `LANE_ARITH` and `INTERLEAVE`; an unmasked exception
  merges its flags into MXCSR and is the guest's #XM with the code `xm_status` picks (`vm_backend.py` 4301-4302).
  PCMPISTRI and the RCP and RSQRT approximations are refused by name. `model_step` no longer refuses an `sse` op and
  writes XMM back after a step, as `_slow_sse` sets the vectors (4318).
- **Judged against Windows:** set `semantics`' model cases, 2,085 in 40 classes in each of its two runs, 4,170 in all,
  with 0 difference classes; both runs now judge all 39,708 cases, their 5,485 Windows faults all at the VM's fault.
  `expected.tsv`'s two rows fell from 2,085 to 0 model cases; `expected-by-class.tsv` changed in those 80 rows.
- **Not reached, against the card:** the card said the translator lifts these classes to `Op::Sse`; it lifts group
  4's 52 classes (`SSE_FLOAT_CORE`) and group 5's integer block, not the other 27 of the card's 67: CMPSD_XMM 367,
  CMPSS 372, MOVHPD 636, MPSADBW 822, PSLLW 328, PINSRD 220 and 21 more, 4,760 cases over sets 049, 050 and 065c. They
  do not lift, so the model never sees them. Lifting them means porting their lowering too, which the C words judge
  compares: they are the batch run's 848 SSE calls the census holds unlifted.
- **Plant 1** (DAZ ignored, `Env::src`): the case runner failed, "2 ADDPS fault code: case 26004: [2e, 44, 0f, 58,
  be, 16, 60, 7c, 6b] vm 0xc0000090, cpu 0xc000008f", with ADDPS's fault MXCSR and ADDSS. Restored byte for byte.
- **Plant 2** as the card wrote it (CMPSS's LT for LE) reached no judge: CMPSS does not lift, so no CMPSS case reaches
  the model. Replaced by UCOMISS and UCOMISD signalling on a QNaN as COMISS does: the case runner failed, "3 UCOMISS vm
  fault, cpu completed: case 21671: [67, 4c, 0f, 2e, 65, 35] vm 0xc0000090", with "1 UCOMISS MXCSR". Restored byte
  for byte.
- **Records:** four `case-runner-rust` records (1793193981524123000, -557203000, -584542000, -614746000), the
  verdicts Sprint 177's held (235, 177, 107, 26; FCOS and FSIN rejected on the first), `check_records` 0 violations
  each; the ledger holds 6 open entries, two for each record that reports the pair.
- **Departures:** the 27 classes above; `crates/w2m_vm/src/lib.rs`, `cases.rs` and `store/case_runner/discrepancy-
  ledger.json` changed, files the card did not name; a `sed` inserted three lines (`LaneFn` and its comment), a
  multi-line edit outside the Edit tool.
