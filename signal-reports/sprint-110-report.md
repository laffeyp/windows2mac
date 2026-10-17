# SIGNAL_REPORT — Sprint 110 — the frontier's execution placed in time on its record

Made by `scripts/signal_report.py 110` from 1 records under store/runs/ whose baseline names one of the sprint's 2 commits. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 110.

## 1. Observed

### scope_confirmation

> H2 of the SDD hardening order, its second half (docs/plan.md; Decision 2026-10-17; Deferred 2026-10-17, Sprint 109).
> One concept: the frontier's record says when its execution ran, how long, and within what bound.

### work_performed

- 2be498ebd Sprint 110 closed: the batch run's record on ff2fe2b4; the EnTSO plant given ten runs; BLACKBOARD; Sprint 099 archived
- ff2fe2b47 Sprint 110: the frontier's execution placed in time on its record

### signal_trace

```
frontier-643a6283510c-1792269281973223000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 107, EXECUTION_COMPLETED 1, TRACE_INGESTED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    0.001  harness -> SESSION_INIT (contract_version=0.9, harness_version=0.0.1, profile=w2m-vm, purpose=application, vocab_version=0.9)
  t=    0.207  destination_runner -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1792269281973223000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afb
  t=   33.962  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792269281
  t=   33.962  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792269281
  t=   33.962  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792269281
  t=   33.965  destination_runner -> EXECUTION_COMPLETED (elapsed_seconds=33.546, execution_id=frontier-643a6283510c-1792269281973223000, exit_code=None, exit_status=killed, guest_line_count=10546, trace_sha256=sha256:6ebc57de648980d64
  t=   33.978  ingester -> TRACE_INGESTED (execution_id=frontier-643a6283510c-1792269281973223000, line_count=10546, thread_count=28, trace_sha256=sha256:6ebc57de648980d6484db4f6b5965ceb5d042efb..., vocab_version=0.9)
  t=   34.264  comparator -> COMPARISON_COMPLETED (comparison_id=cmp-frontier-643a6283510c-1792269281973223000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + game-ru
  t=   34.265  closer -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=34.645, outcome=completed)
  ... MEASUREMENT_RECORDED 104 more
```

### signal_trace_tags

- `COMPARISON_COMPLETED`
- `EXECUTION_COMPLETED`
- `EXECUTION_STARTED`
- `MEASUREMENT_RECORDED`
- `SESSION_COMPLETED`
- `SESSION_INIT`
- `TRACE_INGESTED`

## 2. Expected

> - On the batch run's record: EXECUTION_STARTED with `timeout_seconds` 10800 and `kill_grace_seconds` 60, its `t`
>   within 5 s of the harness's SESSION_INIT; EXECUTION_COMPLETED with `elapsed_seconds` the run's wall time, its `t`
>   that many seconds after EXECUTION_STARTED's, within 2 s; the MEASUREMENT_RECORDED of Sprint 108 unchanged in kind.

## 3. Delta

### emits_check

- `EXECUTION_COMPLETED`: fired 1 time
- `EXECUTION_STARTED`: fired 1 time
- `MEASUREMENT_RECORDED`: fired 107 times
- `SESSION_INIT`: fired 1 time
- fired, not named in Emits: `COMPARISON_COMPLETED` (1), `SESSION_COMPLETED` (1), `TRACE_INGESTED` (1)

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 110: its Rubber Duck Pass and close.
