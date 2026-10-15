# Sprint 081 — the small review of Sprint 080: the acceptance count, the compiler's contract, the residuals named

```yaml
---
id: 081
status: closed
phase: 5
pass_kind: functional
---
```

## scope

The reviewer's small review of Sprint 080 (7c089ed8..0e760a2c): sound; the table re-derives byte for byte on HEAD
(the reviewer's own run, 78 s, `cmp` identical). Its findings:

- **1 (medium).** `record.one_step_run` wrote `accepted_count=0` after a CANDIDATE_ACCEPTED. It counts the run's
  CONTRACT_ACCEPTED, CANDIDATE_ACCEPTED and ARTIFACT_ACCEPTED, as the tag's note says. (`vm_backend.py`'s
  compilation session emits no acceptance, so its 0 is right; it is not changed.)
- **2 (medium).** The judge cited `w2m.vm_compiler.stencil_table@0.80.0` with no contract on disk.
  `contracts/w2m.vm_compiler.stencil_table/0.80.0.json` states it, with a new acceptance form in `w2m.contract`,
  `exact_words`: where the target emits, its words equal the reference compiler's; the refusals it may give instead;
  its judges and whether each one's source is outside the target's sources. The loader refuses an `exact_words`
  contract with no such judge (KIT_DIARY #59), or with no observed evidence.
- **3 (low).** `_hole`'s docstring and the contract's open questions name what the alternatives do not cover: a fit
  at another position, a function of two values or outside EXPRESSIONS, and expressions that agree on the probe values
  but differ elsewhere. BLACKBOARD Decisions are not edited (WORKING_AGREEMENT); the residual is recorded here and in
  the sprint tail.
- **4 (low).** For the sprint that puts the C in the VM: its measure counts blocks cut or sent to the host by a guard
  or an alternative (`VMC_GUARDED`) apart from those with no stencil (`VMC_NO_STENCIL`), so Decision 2026-10-15's
  revisit condition shows which one costs.
- **5 (low, trade-off).** `rules.COMPILER` takes all of `vm_backend.py` (4,500 lines: the monitor, scheduler and
  request server too) into every class identity, so a host edit re-runs 458 classes (61 to 75 s). Kept; narrowed to
  compile_insn's sources if sweeps start costing (recentering step 4's revisit condition).
- **6 (low).** The record's producing commit is inferred (4394114c); the table re-derives on HEAD's compile_insn, so
  it affects only which instructions were reached. No action.

## prerequisites

- 080 (closed, on CI)

## signal contract

### Emits

- None new; SESSION_COMPLETED.accepted_count now holds the run's acceptances.

### Invariants

- Vocabulary unchanged. The compiler unchanged.

## artifact contract

- `src/w2m/record.py`, `src/w2m/contract.py`, `contracts/w2m.vm_compiler.stencil_table/0.80.0.json`,
  `scripts/stencil_table.py` (docstring); tests in `tests/test_record_provenance.py`, `tests/test_contract.py`
- `scripts/gates.sh` exit 0; CI `success` on the closing commit

## observation contract

- A session that accepts reports 1; the compiler's contract loads; one with no outside judge is refused.

## done criteria

1-3 done, 4-6 recorded; gates and CI pass.

## observed
- **1.** `one_step_run` counts CONTRACT_ACCEPTED, CANDIDATE_ACCEPTED and ARTIFACT_ACCEPTED in `done`;
  `test_a_session_that_accepts_counts_it` holds a run that accepts at 1.
- **2.** `contracts/w2m.vm_compiler.stencil_table/0.80.0.json` loads under the new `exact_words` form; the cases and
  spec judges are marked outside the target's sources; with every judge marked inside, the loader refuses it
  (`test_the_compiler_contract_names_a_judge_outside_its_sources`).
- **3.** Named in `_hole`'s docstring and the contract's open questions.
- **Gates:** `scripts/gates.sh` exit 0, 2,440 passed; no family or class identity changed (record.py and contract.py
  are in neither).
- **Closed** on 1753eca3 (CI run 38810199578, success).
