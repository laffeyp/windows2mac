# SIGNAL_REPORT — Sprint 117 — the kick

Made by `scripts/signal_report.py 117` from 527 records under store/runs/ whose baseline names one of the sprint's 2 commits. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 117.

## 1. Observed

### scope_confirmation

> Step 1, the chain's fourth sprint (docs/design-several-vcpus.md, "The host for several vCPUs, designed twice": "a
> kick, not a cut"). One concept: the host stops one running vCPU on demand. Today a vCPU leaves `hv_vcpu_run` at an
> exit of its own or when its run's deadline passes (the monitor's watchdog, `hv_vcpus_exit`); a host thread that wants
> another vCPU stopped (a stop request, chain step 6's SuspendThread on a thread another vCPU runs) has no way to ask,
> and the parked Sprint 107 attempt cut every run at 50 ms instead.

### work_performed

- 6f4bcfc3b Sprint 117: the batch run and sweeps on 43d8b323
- 43d8b323e Sprint 117: the kick; a host thread stops one vCPU on demand

### signal_trace

```
class-* 458 records: SESSION_INIT 458, CANDIDATE_GENERATED 458, COMPARISON_COMPLETED 916, CANDIDATE_ACCEPTED 456, SESSION_COMPLETED 458, REDUCTION_COMPLETED 8, DISCREPANCY_REPORTED 8, DISCREPANCY_ROUTED 8; producers failed 0; not finalised 0
family-* 68 records: SESSION_INIT 68, CANDIDATE_GENERATED 68, COMPARISON_COMPLETED 273, CANDIDATE_ACCEPTED 68, SESSION_COMPLETED 68; producers failed 0; not finalised 0
frontier-643a6283510c-1792300672353112000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 107, EXECUTION_COMPLETED 1, TRACE_INGESTED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    0.001  harness -> SESSION_INIT (contract_version=0.9, harness_version=0.0.1, profile=w2m-vm, purpose=application, vocab_version=0.9)
  t=    0.225  destination_runner -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1792300672353112000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afb
  t=   39.307  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792300672
  t=   39.307  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792300672
  t=   39.307  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792300672
  t=   39.311  destination_runner -> EXECUTION_COMPLETED (elapsed_seconds=38.841, execution_id=frontier-643a6283510c-1792300672353112000, exit_code=None, exit_status=killed, guest_line_count=10546, trace_sha256=sha256:f2fe27e756497def2
  t=   39.323  ingester -> TRACE_INGESTED (execution_id=frontier-643a6283510c-1792300672353112000, line_count=10546, thread_count=28, trace_sha256=sha256:f2fe27e756497def23713eee48109f15a98945e5..., vocab_version=0.9)
  t=   39.669  comparator -> COMPARISON_COMPLETED (comparison_id=cmp-frontier-643a6283510c-1792300672353112000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + game-ru
  t=   39.670  closer -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=40.081, outcome=completed)
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

> - On the batch run's record, as on Sprint 116's (frontier-643a6283510c-1792299009808193000): SESSION_INIT
>   (vocab_version 0.9); EXECUTION_STARTED (timeout_seconds 10800, kill_grace_seconds 60); MEASUREMENT_RECORDED 107
>   times; EXECUTION_COMPLETED (exit_status killed); TRACE_INGESTED (thread_count 28); COMPARISON_COMPLETED
>   (first_divergence none); SESSION_COMPLETED (outcome completed).
> - Per class run (458): SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, then CANDIDATE_ACCEPTED (456) or, for
>   FCOS and FSIN, REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED (8 each), SESSION_COMPLETED. Per
>   family run (68): SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED, SESSION_COMPLETED.

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

See the BLACKBOARD tail entry for Sprint 117: its Rubber Duck Pass and close.
