# SIGNAL_REPORT — Sprint 108 — vocabulary 0.9: the batch run's numbers on its record

Made by `scripts/signal_report.py 108` from 527 records under store/runs/ whose baseline names one of the sprint's 3 commits. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 108.

## 1. Observed

### scope_confirmation

> H1 of the SDD hardening order (docs/plan.md; Decision 2026-10-17; `docs/review-2026-10-17-sdd-hardening.md` item 1,
> which takes in design sprint D1). One concept: the numbers the batch run is judged by reach its record. Today the
> runner (`scripts/run_frontier.py`) gathers them into `state` and prints them to stderr; the record holds only the VM
> compiler's counts (Sprint 101). Sprint 094's table was rebuilt from logs in `build/sprint-094/` for that reason.

### work_performed

- 4a8f0eb15 Sprint 108 closed: the sweeps on e1cbc46f, the batch run's record, BLACKBOARD; Sprint 097 archived
- e1cbc46f7 Sprint 108: scripts/run_numbers.py prints a run's numbers from its record and what does not add up
- 018422051 Sprint 108: vocabulary 0.9, the batch run's numbers on its record

### signal_trace

```
class-* 458 records: SESSION_INIT 458, CANDIDATE_GENERATED 458, COMPARISON_COMPLETED 916, CANDIDATE_ACCEPTED 456, SESSION_COMPLETED 458, DISCREPANCY_REPORTED 8, DISCREPANCY_ROUTED 8; producers failed 0; not finalised 0
family-* 68 records: SESSION_INIT 68, CANDIDATE_GENERATED 68, COMPARISON_COMPLETED 273, CANDIDATE_ACCEPTED 68, SESSION_COMPLETED 68; producers failed 0; not finalised 0
frontier-643a6283510c-1792267359516238000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 107, EXECUTION_COMPLETED 1, TRACE_INGESTED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    0.001  harness -> SESSION_INIT (contract_version=0.9, harness_version=0.0.1, profile=w2m-vm, purpose=application, vocab_version=0.9)
  t=   41.012  destination_runner -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1792267359516238000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afb
  t=   41.012  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792267359
  t=   41.012  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792267359
  t=   41.012  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792267359
  t=   41.016  destination_runner -> EXECUTION_COMPLETED (elapsed_seconds=None, execution_id=frontier-643a6283510c-1792267359516238000, exit_code=None, exit_status=killed, guest_line_count=10546, trace_sha256=sha256:882c1928d54fa328f0d
  t=   41.029  ingester -> TRACE_INGESTED (execution_id=frontier-643a6283510c-1792267359516238000, line_count=10546, thread_count=28, trace_sha256=sha256:882c1928d54fa328f0df19d5800a1cbec13f7d94..., vocab_version=0.9)
  t=   41.372  comparator -> COMPARISON_COMPLETED (comparison_id=cmp-frontier-643a6283510c-1792267359516238000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + game-ru
  t=   41.373  closer -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=41.804, outcome=completed)
  ... MEASUREMENT_RECORDED 104 more
```

### signal_trace_tags

- `CANDIDATE_ACCEPTED`
- `CANDIDATE_GENERATED`
- `COMPARISON_COMPLETED`
- `DISCREPANCY_REPORTED`
- `DISCREPANCY_ROUTED`
- `EXECUTION_COMPLETED`
- `EXECUTION_STARTED`
- `MEASUREMENT_RECORDED`
- `SESSION_COMPLETED`
- `SESSION_INIT`
- `TRACE_INGESTED`

## 2. Expected

> - MEASUREMENT_RECORDED from the batch run's runner, after the compile counts, for each row above: phase `discovery`,
>   backend `managed_vm`, artifact_state as Sprint 101's. Expected on the closing batch run:
>   - `instructions_retired` `process` equal to the stop line's instruction count, and the `thread <id>` values summing
>     to it;
>   - `vm_exits` summing to the exits the progress line counts;
>   - `run_seconds` `vm` plus the `host hvc` values within 1% of the runner's wall time around `machine.run`;
>   - `thread_switches` `in the VM` equal to the printed "switches in the VM".

## 3. Delta

### emits_check

- `MEASUREMENT_RECORDED`: fired 107 times
- fired, not named in Emits: `CANDIDATE_ACCEPTED` (524), `CANDIDATE_GENERATED` (526), `COMPARISON_COMPLETED` (1190), `DISCREPANCY_REPORTED` (8), `DISCREPANCY_ROUTED` (8), `EXECUTION_COMPLETED` (1), `EXECUTION_STARTED` (1), `SESSION_COMPLETED` (527), `SESSION_INIT` (527), `TRACE_INGESTED` (1)

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 108: its Rubber Duck Pass and close.
