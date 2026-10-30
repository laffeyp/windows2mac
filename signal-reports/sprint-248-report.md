# SIGNAL_REPORT — Sprint 248 — The compile burst

Made by `signal_report 248` from 3 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 248.

## 1. Observed

### scope_confirmation

> The Decision "Sequence for step 5" as annotated at Sprint 247's close: before the translation cache, the largest
> compile time in a short window, on timed cold records run alone, with the total re-measured on them (reviewer-3's
> Direction and Low 2 at 247's close). Sprint 247 put the total on the record, 1,626 to 1,800 ms of 274,834 compiles,
> measured beside the session's own builds; a total says nothing of how the compiles bunch.

### work_performed

- c62d9ac5e Sprint 248: three timed records run alone, the compile burst on each
- 653c1f857 Sprint 248: the longest compile and the most in a 60 Hz frame, timed by the probe
- fb40eda76 Sprint 248 card: the compile burst

### signal_trace

```
w2m-vm-game-1793386986000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 46, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   93.680  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   93.680  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793387079978655000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   93.680  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933870799786550
  t=   93.680  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933870799786550
  t=   93.680  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933870799786550
  t=   93.681  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=93.664, execution_id=frontier-643a6283510c-1793387079978655000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   93.770  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793387079978655000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   93.770  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=93.75, outcome=completed)
  ... MEASUREMENT_RECORDED 43 more
w2m-vm-game-1793387080000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 46, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   93.691  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   93.691  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793387173986045000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   93.691  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933871739860450
  t=   93.691  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933871739860450
  t=   93.692  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933871739860450
  t=   93.693  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=93.675, execution_id=frontier-643a6283510c-1793387173986045000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   93.782  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793387173986045000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   93.782  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=93.761, outcome=completed)
  ... MEASUREMENT_RECORDED 43 more
w2m-vm-game-1793387174000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 46, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   94.974  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   94.974  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793387269270898000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   94.974  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933872692708980
  t=   94.975  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933872692708980
  t=   94.975  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933872692708980
  t=   94.976  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=94.955, execution_id=frontier-643a6283510c-1793387269270898000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   95.065  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793387269270898000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   95.065  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=95.041, outcome=completed)
  ... MEASUREMENT_RECORDED 43 more
```

### signal_trace_tags

- `COMPARISON_COMPLETED`
- `EXECUTION_COMPLETED`
- `EXECUTION_STARTED`
- `MEASUREMENT_RECORDED`
- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - Three batch-run records, each with `compile_critical_path_ms` "vm", "vm longest compile" and "vm most in a 60 Hz
>   frame", and Sprint 246's measurements.

## 3. Delta

### emits_check

- fired, not named in Emits: `COMPARISON_COMPLETED` (3), `EXECUTION_COMPLETED` (3), `EXECUTION_STARTED` (3), `MEASUREMENT_RECORDED` (138), `SESSION_COMPLETED` (3), `SESSION_INIT` (3)
- the card's Emits names no tag, and the sprint's records fired the ones above

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 248: its Rubber Duck Pass and close.
