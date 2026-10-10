# SIGNAL_REPORT — Sprint 044

## 1. Observed

### scope_confirmation

> The launcher's C runtime start-up completes in w2m. The frontier stops at the import `UnityPlayer.dll!UnityMain` after 19,084 instructions, and every service the launcher called before it is judged against Windows on its own family's record.

### work_performed

- Families, each a no-CRT probe on the runners, a contract from its rows, guest C and an item run: process queries, modules, critical sections, thread storage, heap, console 0.2.0, code pages 0.2.0, environment (set and empty blocks), exception filter (`scripts/family_run.py`, `tools/guest_runtime/*.c`, `contracts/windows.*`).
- Instruction case sets 044, 044b, 044c, 044d, 044e, judged on Intel runners; 183 classes accepted (`scripts/class_run.py`; a class may span sets).
- VM: GS-relative operands, the guest counter, start and clock modes, the heap arena, the environment block, host requests counted by op, argument-form stops as SERVICE_UNIMPLEMENTED, per-API counters for every id.
- `scripts/class_census.py`; `scripts/model_family.py` (a cloud model writes a family in substrate's session topology); probe builds with `/Brepro`.

### signal_trace

```
store/runs/frontier-643a6283510c-1791630008722386000: SESSION_INIT, EXECUTION_STARTED, SERVICE_UNIMPLEMENTED (UnityPlayer.dll!UnityMain, 0x1400011F2, 19,084 instructions), SESSION_COMPLETED
store/runs/family-runs.json: 10 families, each SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED x runner binaries (4 to 16), CANDIDATE_ACCEPTED, SESSION_COMPLETED; all on the final runtime
store/runs/class-runs.json: 183 classes, model and managed_vm conform; RET_NEAR on 272 cases (sets 039 and 044e)
store/runs/semantics-044{,b,c,d,e}-*, vm-oracle-044*: every unit accepted
store/runs/model-family-environment-kimi-k2.7-code-cloud-1791629810134921000: 16 ModelReply, 15 ToolCall, 2 judgments; item runs family-environment_set-1791629839307013000, family-environment_empty-1791629848553471000, both CANDIDATE_ACCEPTED
```

## 2. Expected

> The card expected the stop at `UnityMain` with every earlier service served and judged, each family equal to Windows with a planted fault of named reach, hot services without host exits, and a family written by a cloud model. All held; three named reaches were wrong and are recorded.

## 3. Delta

### dual_contract_self_grade

**Signal contract:** pass (vocabulary 0.7, no new tags; one decision per item run). **Artifact contract:** gates 0 and CI on the closing commit. **Observation contract:** frontier pass; families pass (three wrong reaches and one blind spot recorded, the blind spot closed by a probe row); hot services pass (`tests/test_hot_services.py`); model family pass.

**overall:** pass.

## 4. Hypothesis

### rubber_duck_observations

**Sequence narration:** the frontier stops at a service; a scratch run with logged refusals shows the forms start-up uses; a probe records Windows on the runners; the rows become a contract; guest C answers them; plants check the judge; the item run accepts; the frontier moves.

- *Observation 1 (wrong reaches):* two plants stopped the run instead of changing rows; one changed 4 rows for 2 named; one changed nothing on a zero arena. Disposition: recorded; plants replaced; probe row added.
- *Observation 2 (silent failures in the instruments):* uncounted ids above 63, argument-form stops reported as faults, a stale heap acceptance. Disposition: resolved-here, each with a test that fails without the fix; KIT_DIARY #30.
- *Observation 3 (the model's file):* accepted and different from the agent's where the rows are silent. Disposition: recorded; KIT_DIARY #29.

### status_and_blockers

`status: complete`
