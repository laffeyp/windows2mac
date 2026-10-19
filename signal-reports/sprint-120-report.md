# SIGNAL_REPORT — Sprint 120 — every timed wait ends

Made by `scripts/signal_report.py 120` from 530 records under store/runs/ whose baseline names one of the sprint's 6 commits. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 120.

## 1. Observed

### scope_confirmation

> A defect class, not a step: a thread blocked with a timeout whose deadline passes and that nothing makes ready, and a
> host thread that waits with nothing able to end its wait. Sprint 119 recorded one member ("vCPU 0 keeps no deadline
> while it waits") as deferred; it is a hang, and the class has the members below, all fixed here (the last three found
> while fixing the first ones).

### work_performed

- 41e338e29 Sprint 120: the context retry's reach test runs up to 40 times
- ea52c2f99 Sprint 120: the batch run and sweeps on 8838314f; threads, running_thread_context and wait_handoff on two vCPUs
- ef33fe1d3 Sprint 120: the lock plant's test runs until the race shows
- 8838314f1 Sprint 120: the no-wait plant's test runs until a crossing run diverges
- 25e57995e Sprint 120: a child starts on the machine's clock; the review's untested paths
- 0da7f85e9 Sprint 120: every timed wait ends

### signal_trace

```
class-* 458 records: SESSION_INIT 458, CANDIDATE_GENERATED 458, COMPARISON_COMPLETED 916, CANDIDATE_ACCEPTED 456, SESSION_COMPLETED 458, REDUCTION_COMPLETED 8, DISCREPANCY_REPORTED 8, DISCREPANCY_ROUTED 8; producers failed 0; not finalised 0
family-* 71 records: SESSION_INIT 71, CANDIDATE_GENERATED 71, COMPARISON_COMPLETED 277, CANDIDATE_ACCEPTED 71, SESSION_COMPLETED 71; producers failed 0; not finalised 0
frontier-643a6283510c-1792347897080595000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 104, EXECUTION_COMPLETED 1, TRACE_INGESTED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    0.001  harness -> SESSION_INIT (contract_version=0.9, harness_version=0.0.1, profile=w2m-vm, purpose=application, vocab_version=0.9)
  t=    0.200  destination_runner -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1792347897080595000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afb
  t=   36.827  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792347897
  t=   36.827  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792347897
  t=   36.827  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792347897
  t=   36.830  destination_runner -> EXECUTION_COMPLETED (elapsed_seconds=36.206, execution_id=frontier-643a6283510c-1792347897080595000, exit_code=None, exit_status=killed, guest_line_count=10535, trace_sha256=sha256:d1715445a33682c22
  t=   36.843  ingester -> TRACE_INGESTED (execution_id=frontier-643a6283510c-1792347897080595000, line_count=10535, thread_count=28, trace_sha256=sha256:d1715445a33682c22fece75d512ff4c363aab1d0..., vocab_version=0.9)
  t=   37.116  comparator -> COMPARISON_COMPLETED (comparison_id=cmp-frontier-643a6283510c-1792347897080595000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + game-ru
  t=   37.116  closer -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=37.49, outcome=completed)
  ... MEASUREMENT_RECORDED 101 more
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

> - On the threads, running_thread_context and wait_handoff runs on two vCPUs (`family_run.py F --run --vcpus 2`), as on
>   Sprint 119's: SESSION_INIT (purpose conformance), CANDIDATE_GENERATED, COMPARISON_COMPLETED for each reference run
>   (verdict conforms, first_divergence none), CANDIDATE_ACCEPTED, SESSION_COMPLETED (outcome completed), baseline
>   `vcpus` 2.
> - On the batch run's record, as on Sprint 119's: SESSION_INIT, EXECUTION_STARTED, MEASUREMENT_RECORDED 104 times (Sprint 119's 107 less hvc 12's exits and seconds and the
>   "a thread ready" hand-overs, which no longer occur),
>   EXECUTION_COMPLETED (exit_status killed), TRACE_INGESTED, COMPARISON_COMPLETED (first_divergence none),
>   SESSION_COMPLETED. Per class run (458) and family run (68), as on Sprint 119's sweep: SESSION_INIT,
>   CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED (456 and 68) or for FCOS and FSIN REDUCTION_COMPLETED,
>   DISCREPANCY_REPORTED and DISCREPANCY_ROUTED, SESSION_COMPLETED. (Amended at the close: the first draft named the
>   sweep's tags by reference, and the Emits check counted the last three as unnamed.)

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 527 times
- `CANDIDATE_GENERATED`: fired 529 times
- `COMPARISON_COMPLETED`: fired 1,194 times
- `DISCREPANCY_REPORTED`: fired 8 times
- `DISCREPANCY_ROUTED`: fired 8 times
- `EXECUTION_COMPLETED`: fired 1 time
- `EXECUTION_STARTED`: fired 1 time
- `MEASUREMENT_RECORDED`: fired 104 times
- `REDUCTION_COMPLETED`: fired 8 times
- `SESSION_COMPLETED`: fired 530 times
- `SESSION_INIT`: fired 530 times
- `TRACE_INGESTED`: fired 1 time

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 120: its Rubber Duck Pass and close.
