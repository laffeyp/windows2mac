# SIGNAL_REPORT — Sprint 115 — the Vcpu object, passed

Made by `scripts/signal_report.py 115` from 527 records under store/runs/ whose baseline names one of the sprint's 2 commits. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 115.

## 1. Observed

### scope_confirmation

> Step 1, the chain's second sprint (docs/design-several-vcpus.md, "The host for several vCPUs, designed twice"). One
> concept: what belongs to one vCPU is a `Vcpu` object, and every handler that reads it takes it as an argument. Today
> `VM.serving`, `VM._md`, `VM._st`, `VM._vswap` and `VM._channel` hold the served vCPU's index, the file offsets of its
> monitor data page and state block, its pending vector swap and its channel; `VM.serve(i)` would switch them and has no
> caller, and `VM.vcpu_channels` holds vCPUs 1.. apart from vCPU 0. 92 sites in 38 methods read them, and 11 more
> methods reach them through a call (the call graph over `VM`'s methods: 49 in all).

### work_performed

- ffd6bbff5 Sprint 115: the batch run and sweeps on 076b3225
- 076b32258 Sprint 115: the Vcpu object, passed; no VM attribute names a served vCPU

### signal_trace

```
class-* 458 records: SESSION_INIT 458, CANDIDATE_GENERATED 458, COMPARISON_COMPLETED 916, CANDIDATE_ACCEPTED 456, SESSION_COMPLETED 458, REDUCTION_COMPLETED 8, DISCREPANCY_REPORTED 8, DISCREPANCY_ROUTED 8; producers failed 0; not finalised 0
family-* 68 records: SESSION_INIT 68, CANDIDATE_GENERATED 68, COMPARISON_COMPLETED 273, CANDIDATE_ACCEPTED 68, SESSION_COMPLETED 68; producers failed 0; not finalised 0
frontier-643a6283510c-1792295964556678000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 107, EXECUTION_COMPLETED 1, TRACE_INGESTED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    0.001  harness -> SESSION_INIT (contract_version=0.9, harness_version=0.0.1, profile=w2m-vm, purpose=application, vocab_version=0.9)
  t=    0.201  destination_runner -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1792295964556678000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afb
  t=   35.147  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792295964
  t=   35.147  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792295964
  t=   35.147  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792295964
  t=   35.150  destination_runner -> EXECUTION_COMPLETED (elapsed_seconds=34.706, execution_id=frontier-643a6283510c-1792295964556678000, exit_code=None, exit_status=killed, guest_line_count=10546, trace_sha256=sha256:eb4296f54246d814d
  t=   35.162  ingester -> TRACE_INGESTED (execution_id=frontier-643a6283510c-1792295964556678000, line_count=10546, thread_count=28, trace_sha256=sha256:eb4296f54246d814dd7c1577f4aaac6f3ba91fbf..., vocab_version=0.9)
  t=   35.487  comparator -> COMPARISON_COMPLETED (comparison_id=cmp-frontier-643a6283510c-1792295964556678000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + game-ru
  t=   35.487  closer -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=35.867, outcome=completed)
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

> - On the batch run's record, as on Sprint 114's (frontier-643a6283510c-1792273674526334000): SESSION_INIT
>   (vocab_version 0.9, purpose application); EXECUTION_STARTED (timeout_seconds 10800, kill_grace_seconds 60);
>   MEASUREMENT_RECORDED 107 times (23 `vm_compile_blocks`, 84 of P20's metrics); EXECUTION_COMPLETED (exit_status
>   killed, exit_code absent); TRACE_INGESTED (thread_count 28); COMPARISON_COMPLETED (first_divergence none);
>   SESSION_COMPLETED (outcome completed, discrepancy_count 0).
> - Per class run (458): SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED (2 each), then CANDIDATE_ACCEPTED
>   (456) or, for FCOS and FSIN, REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED (8 each),
>   SESSION_COMPLETED. Per family run (68): SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED,
>   SESSION_COMPLETED.

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 524 times
- `CANDIDATE_GENERATED`: fired 526 times
- `COMPARISON_COMPLETED`: fired 1,190 times
- `DISCREPANCY_REPORTED`: fired 8 times
- `DISCREPANCY_ROUTED`: fired 8 times
- `EXECUTION_COMPLETED`: fired 1 time
- `EXECUTION_STARTED`: fired 1 time
- `MEASUREMENT_RECORDED`: fired 107 times
- `REDUCTION_COMPLETED`: fired 8 times
- `SESSION_COMPLETED`: fired 527 times
- `SESSION_INIT`: fired 527 times
- `TRACE_INGESTED`: fired 1 time

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 115: its Rubber Duck Pass and close.
