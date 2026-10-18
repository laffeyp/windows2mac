# SIGNAL_REPORT — Sprint 118 — guest threads on several vCPUs

Made by `scripts/signal_report.py 118` from 528 records under store/runs/ whose baseline names one of the sprint's 2 commits. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 118.

## 1. Observed

### scope_confirmation

> Step 1, the chain's fifth sprint (docs/design-several-vcpus.md, "The host for several vCPUs, designed twice"). One
> concept: one process's guest threads run on several vCPUs at once, each vCPU served by a host thread of its own.

### work_performed

- 69e1d704b Sprint 118: the batch run and sweeps on f26cf79c; wait_handoff on two vCPUs
- f26cf79c6 Sprint 118: one process's threads on several vCPUs

### signal_trace

```
class-* 458 records: SESSION_INIT 458, CANDIDATE_GENERATED 458, COMPARISON_COMPLETED 916, CANDIDATE_ACCEPTED 456, SESSION_COMPLETED 458, REDUCTION_COMPLETED 8, DISCREPANCY_REPORTED 8, DISCREPANCY_ROUTED 8; producers failed 0; not finalised 0
family-* 69 records: SESSION_INIT 69, CANDIDATE_GENERATED 69, COMPARISON_COMPLETED 274, CANDIDATE_ACCEPTED 69, SESSION_COMPLETED 69; producers failed 0; not finalised 0
frontier-643a6283510c-1792306704577911000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 107, EXECUTION_COMPLETED 1, TRACE_INGESTED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    0.001  harness -> SESSION_INIT (contract_version=0.9, harness_version=0.0.1, profile=w2m-vm, purpose=application, vocab_version=0.9)
  t=    0.233  destination_runner -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1792306704577911000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afb
  t=   40.072  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792306704
  t=   40.072  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792306704
  t=   40.072  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792306704
  t=   40.075  destination_runner -> EXECUTION_COMPLETED (elapsed_seconds=39.521, execution_id=frontier-643a6283510c-1792306704577911000, exit_code=None, exit_status=killed, guest_line_count=10546, trace_sha256=sha256:b5dee77eae548bdbd
  t=   40.088  ingester -> TRACE_INGESTED (execution_id=frontier-643a6283510c-1792306704577911000, line_count=10546, thread_count=28, trace_sha256=sha256:b5dee77eae548bdbd45f3d8fa191285fdc36d12f..., vocab_version=0.9)
  t=   40.381  comparator -> COMPARISON_COMPLETED (comparison_id=cmp-frontier-643a6283510c-1792306704577911000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + game-ru
  t=   40.381  closer -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=40.786, outcome=completed)
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
- `REDUCTION_COMPLETED`
- `SESSION_COMPLETED`
- `SESSION_INIT`
- `TRACE_INGESTED`

## 2. Expected

> - On the wait_handoff run on two vCPUs (`family_run.py wait_handoff --run --vcpus 2`), its record's baseline naming
>   `vcpus` 2: SESSION_INIT (purpose conformance), CANDIDATE_GENERATED, COMPARISON_COMPLETED for each of its runs
>   (verdict conforms, first_divergence none), CANDIDATE_ACCEPTED, SESSION_COMPLETED (outcome completed).
> - On the batch run's record, as on Sprint 117's: SESSION_INIT, EXECUTION_STARTED, MEASUREMENT_RECORDED 107 times,
>   EXECUTION_COMPLETED (exit_status killed), TRACE_INGESTED, COMPARISON_COMPLETED (first_divergence none),
>   SESSION_COMPLETED. Per class run (458) and family run (68), as on Sprint 117's sweep: SESSION_INIT,
>   CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED (456 and 68) or for FCOS and FSIN REDUCTION_COMPLETED,
>   DISCREPANCY_REPORTED and DISCREPANCY_ROUTED, SESSION_COMPLETED.

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 525 times
- `CANDIDATE_GENERATED`: fired 527 times
- `COMPARISON_COMPLETED`: fired 1,191 times
- `DISCREPANCY_REPORTED`: fired 8 times
- `DISCREPANCY_ROUTED`: fired 8 times
- `EXECUTION_COMPLETED`: fired 1 time
- `EXECUTION_STARTED`: fired 1 time
- `MEASUREMENT_RECORDED`: fired 107 times
- `REDUCTION_COMPLETED`: fired 8 times
- `SESSION_COMPLETED`: fired 528 times
- `SESSION_INIT`: fired 528 times
- `TRACE_INGESTED`: fired 1 time

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 118: its Rubber Duck Pass and close.
