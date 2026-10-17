# SIGNAL_REPORT — Sprint 114 — one exit handler

Made by `scripts/signal_report.py 114` from 527 records under store/runs/ whose baseline names one of the sprint's 2 commits. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 114.

## 1. Observed

### scope_confirmation

> Step 1, the chain's first sprint (docs/design-several-vcpus.md, "The host for several vCPUs, designed twice"). One
> concept: the body of `VM.steps`'s loop, which serves one exit, becomes `VM.serve_exit(run, exit, cpsr) -> Resume |
> Stop | Slice | Switched`; `steps` keeps the run's setup, the pre-run words, the monitor call and the yields to a
> kernel.Machine (`Slice`: another VM's turn; `Switched` with IDLE: no thread of this VM can run). `RunState` holds what
> the loop's exits read and change (the hooks, max_steps, forced_stop, last_kind). No behaviour changes: the body moves
> as written, each `continue` a `Resume`, each `break` a `Stop`.

### work_performed

- f3ecb7c16 Sprint 114: the batch run and sweeps on 123de94c
- 123de94ca Sprint 114: one exit handler; steps' loop body becomes VM.serve_exit

### signal_trace

```
class-* 458 records: SESSION_INIT 458, CANDIDATE_GENERATED 458, COMPARISON_COMPLETED 916, CANDIDATE_ACCEPTED 456, SESSION_COMPLETED 458, REDUCTION_COMPLETED 8, DISCREPANCY_REPORTED 8, DISCREPANCY_ROUTED 8; producers failed 0; not finalised 0
family-* 68 records: SESSION_INIT 68, CANDIDATE_GENERATED 68, COMPARISON_COMPLETED 273, CANDIDATE_ACCEPTED 68, SESSION_COMPLETED 68; producers failed 0; not finalised 0
frontier-643a6283510c-1792273674526334000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 107, EXECUTION_COMPLETED 1, TRACE_INGESTED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    0.001  harness -> SESSION_INIT (contract_version=0.9, harness_version=0.0.1, profile=w2m-vm, purpose=application, vocab_version=0.9)
  t=    0.218  destination_runner -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1792273674526334000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afb
  t=   40.212  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792273674
  t=   40.212  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792273674
  t=   40.212  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792273674
  t=   40.216  destination_runner -> EXECUTION_COMPLETED (elapsed_seconds=39.701, execution_id=frontier-643a6283510c-1792273674526334000, exit_code=None, exit_status=killed, guest_line_count=10546, trace_sha256=sha256:9c8a1c6cb5686b2e1
  t=   40.229  ingester -> TRACE_INGESTED (execution_id=frontier-643a6283510c-1792273674526334000, line_count=10546, thread_count=28, trace_sha256=sha256:9c8a1c6cb5686b2e1d7cd1dc05ffa34280752a47..., vocab_version=0.9)
  t=   40.573  comparator -> COMPARISON_COMPLETED (comparison_id=cmp-frontier-643a6283510c-1792273674526334000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + game-ru
  t=   40.574  closer -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=41.015, outcome=completed)
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

> - On the batch run's record, as on Sprint 110's (frontier-643a6283510c-1792269281973223000): SESSION_INIT,
>   EXECUTION_STARTED, MEASUREMENT_RECORDED (the compile counts and P20's numbers, 107 on that run), EXECUTION_COMPLETED,
>   TRACE_INGESTED, COMPARISON_COMPLETED (0 divergences), SESSION_COMPLETED. Per class and family: their item runs'
>   tags as on Sprint 112's sweep.

## 3. Delta

### emits_check

- `COMPARISON_COMPLETED`: fired 1,190 times
- `EXECUTION_COMPLETED`: fired 1 time
- `EXECUTION_STARTED`: fired 1 time
- `MEASUREMENT_RECORDED`: fired 107 times
- `SESSION_COMPLETED`: fired 527 times
- `SESSION_INIT`: fired 527 times
- `TRACE_INGESTED`: fired 1 time
- fired, not named in Emits: `CANDIDATE_ACCEPTED` (524), `CANDIDATE_GENERATED` (526), `DISCREPANCY_REPORTED` (8), `DISCREPANCY_ROUTED` (8), `REDUCTION_COMPLETED` (8)

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 114: its Rubber Duck Pass and close.
