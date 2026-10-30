# SIGNAL_REPORT — Sprint 240 — The speed figure

Made by `signal_report 240` from 3 records under store/runs/ whose baseline names one of the sprint's 4 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 240.

## 1. Observed

### scope_confirmation

> Plan step 4's "Moves: instructions a second on one vCPU", against the reference run's 2,354,295,320 (amended
> 2026-10-28), census row 10, and BLACKBOARD Deferred (Sprint 221) item 2 as annotated at 224: the Rust batch run
> measured in the configuration the stored record ran, read beside the stored figure.

### work_performed

- 13a9b584a Sprint 240: the batch run measured three times, each with its conditions
- 15d04a031 Sprint 240: the machine's state on the batch run's record
- 506c03702 Sprint 240: the batch run's measurements labelled cold, its host seconds on the record
- 15c14c9ef Sprint 240 card: the speed figure

### signal_trace

```
w2m-vm-game-1793364171000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 14, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   94.450  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   94.450  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793364266602587000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   94.450  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933642666025870
  t=   94.450  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933642666025870
  t=   94.450  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933642666025870
  t=   94.450  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=94.312, execution_id=frontier-643a6283510c-1793364266602587000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   94.717  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793364266602587000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   94.717  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=94.398, outcome=completed)
  ... MEASUREMENT_RECORDED 11 more
w2m-vm-game-1793364267000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 14, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   95.260  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   95.260  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793364362452411000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   95.260  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933643624524110
  t=   95.260  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933643624524110
  t=   95.260  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933643624524110
  t=   95.260  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=95.245, execution_id=frontier-643a6283510c-1793364362452411000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   95.351  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793364362452411000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   95.351  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=95.332, outcome=completed)
  ... MEASUREMENT_RECORDED 11 more
w2m-vm-game-1793364362000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 14, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   95.421  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   95.421  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793364458198738000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   95.421  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933644581987380
  t=   95.421  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933644581987380
  t=   95.421  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933644581987380
  t=   95.421  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=95.405, execution_id=frontier-643a6283510c-1793364458198738000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   95.513  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793364458198738000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   95.513  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=95.493, outcome=completed)
  ... MEASUREMENT_RECORDED 11 more
```

### signal_trace_tags

- `COMPARISON_COMPLETED`
- `EXECUTION_COMPLETED`
- `EXECUTION_STARTED`
- `MEASUREMENT_RECORDED`
- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - Three batch-run records, each with MEASUREMENT_RECORDED `instructions_per_second` (variant "process",
>   `artifact_state` "cold") and the run's other measurements.

## 3. Delta

### emits_check

- `MEASUREMENT_RECORDED`: fired 42 times
- fired, not named in Emits: `COMPARISON_COMPLETED` (3), `EXECUTION_COMPLETED` (3), `EXECUTION_STARTED` (3), `SESSION_COMPLETED` (3), `SESSION_INIT` (3)

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 240: its Rubber Duck Pass and close.
