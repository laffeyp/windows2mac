# SIGNAL_REPORT — Sprint 119 — a thread suspended while another vCPU runs it

Made by `scripts/signal_report.py 119` from 529 records under store/runs/ whose baseline names one of the sprint's 2 commits. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 119.

## 1. Observed

### scope_confirmation

> Step 1, the chain's sixth and last sprint (docs/design-several-vcpus.md, "The host for several vCPUs, designed
> twice"). One concept: SuspendThread and GetThreadContext on a thread another vCPU is running.

### work_performed

- 2c3dd9505 Sprint 119: the batch run and sweeps on 983af64b; threads and running_thread_context on two vCPUs
- 983af64ba Sprint 119: a thread suspended while another vCPU runs it

### signal_trace

```
class-* 458 records: SESSION_INIT 458, CANDIDATE_GENERATED 458, COMPARISON_COMPLETED 916, CANDIDATE_ACCEPTED 456, SESSION_COMPLETED 458, REDUCTION_COMPLETED 8, DISCREPANCY_REPORTED 8, DISCREPANCY_ROUTED 8; producers failed 0; not finalised 0
family-* 70 records: SESSION_INIT 70, CANDIDATE_GENERATED 70, COMPARISON_COMPLETED 276, CANDIDATE_ACCEPTED 70, SESSION_COMPLETED 70; producers failed 0; not finalised 0
frontier-643a6283510c-1792313027375176000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 107, EXECUTION_COMPLETED 1, TRACE_INGESTED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    0.001  harness -> SESSION_INIT (contract_version=0.9, harness_version=0.0.1, profile=w2m-vm, purpose=application, vocab_version=0.9)
  t=    0.205  destination_runner -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1792313027375176000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afb
  t=   37.076  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792313027
  t=   37.076  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792313027
  t=   37.077  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792313027
  t=   37.079  destination_runner -> EXECUTION_COMPLETED (elapsed_seconds=36.582, execution_id=frontier-643a6283510c-1792313027375176000, exit_code=None, exit_status=killed, guest_line_count=10546, trace_sha256=sha256:e64ac03e2c2bcbb7d
  t=   37.092  ingester -> TRACE_INGESTED (execution_id=frontier-643a6283510c-1792313027375176000, line_count=10546, thread_count=28, trace_sha256=sha256:e64ac03e2c2bcbb7da05446430ab6c589705a367..., vocab_version=0.9)
  t=   37.365  comparator -> COMPARISON_COMPLETED (comparison_id=cmp-frontier-643a6283510c-1792313027375176000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + game-ru
  t=   37.365  closer -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=37.745, outcome=completed)
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

> - On the threads run on two vCPUs (`family_run.py threads --run --vcpus 2`), its record's baseline naming `vcpus` 2:
>   SESSION_INIT (purpose conformance), CANDIDATE_GENERATED, COMPARISON_COMPLETED for each of its two runs (verdict
>   conforms, first_divergence none), CANDIDATE_ACCEPTED, SESSION_COMPLETED (outcome completed). The same for
>   running_thread_context on two vCPUs (one run).
> - On the batch run's record, as on Sprint 118's: SESSION_INIT, EXECUTION_STARTED, MEASUREMENT_RECORDED 107 times,
>   EXECUTION_COMPLETED (exit_status killed), TRACE_INGESTED, COMPARISON_COMPLETED (first_divergence none),
>   SESSION_COMPLETED. Per class run (458) and family run (68), as on Sprint 118's sweep: SESSION_INIT,
>   CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED (456 and 68) or for FCOS and FSIN REDUCTION_COMPLETED,
>   DISCREPANCY_REPORTED and DISCREPANCY_ROUTED, SESSION_COMPLETED.

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 526 times
- `CANDIDATE_GENERATED`: fired 528 times
- `COMPARISON_COMPLETED`: fired 1,193 times
- `DISCREPANCY_REPORTED`: fired 8 times
- `DISCREPANCY_ROUTED`: fired 8 times
- `EXECUTION_COMPLETED`: fired 1 time
- `EXECUTION_STARTED`: fired 1 time
- `MEASUREMENT_RECORDED`: fired 107 times
- `REDUCTION_COMPLETED`: fired 8 times
- `SESSION_COMPLETED`: fired 529 times
- `SESSION_INIT`: fired 529 times
- `TRACE_INGESTED`: fired 1 time

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 119: its Rubber Duck Pass and close.
