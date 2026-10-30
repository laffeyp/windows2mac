# SIGNAL_REPORT — Sprint 246 — The compile counts on the record

Made by `signal_report 246` from 3 records under store/runs/ whose baseline names one of the sprint's 4 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 246.

## 1. Observed

### scope_confirmation

> The Decision "Sequence for step 5" (reviewer-3's Medium 1 of its review at 243): plan step 5's gate counts "compiles at
> run time … on the record" and their fall by the share the cache covers, and the Rust batch record counts no compile,
> so the cache would have no figure to fall from. Census row 15d (compile time per block) is carried to step 5 in Deferred.

### work_performed

- 41b83a5de Sprint 246: census row 15d as step 5's baseline; its Deferred line annotated
- ea6ab8619 Sprint 246: the batch run measured three times with its compile counts
- 92f4b4cb8 Sprint 246: the compile counts on the batch record
- 0a546a85e Sprint 246 card: the compile counts on the record

### signal_trace

```
w2m-vm-game-1793369487000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 43, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=  135.606  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=  135.607  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793369623290998000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=  135.607  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933696232909980
  t=  135.607  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933696232909980
  t=  135.607  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933696232909980
  t=  135.609  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=135.287, execution_id=frontier-643a6283510c-1793369623290998000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c442
  t=  135.940  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793369623290998000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=  135.941  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=135.41, outcome=completed)
  ... MEASUREMENT_RECORDED 40 more
w2m-vm-game-1793369623000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 43, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=  113.706  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=  113.706  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793369737805815000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=  113.706  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933697378058150
  t=  113.706  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933697378058150
  t=  113.707  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933697378058150
  t=  113.708  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=113.678, execution_id=frontier-643a6283510c-1793369737805815000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c442
  t=  113.799  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793369737805815000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=  113.799  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=113.765, outcome=completed)
  ... MEASUREMENT_RECORDED 40 more
w2m-vm-game-1793369737000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 43, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=  104.826  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=  104.826  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793369842944203000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=  104.826  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933698429442030
  t=  104.826  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933698429442030
  t=  104.826  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933698429442030
  t=  104.827  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=104.81, execution_id=frontier-643a6283510c-1793369842944203000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=  104.919  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793369842944203000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=  104.919  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=104.898, outcome=completed)
  ... MEASUREMENT_RECORDED 40 more
```

### signal_trace_tags

- `COMPARISON_COMPLETED`
- `EXECUTION_COMPLETED`
- `EXECUTION_STARTED`
- `MEASUREMENT_RECORDED`
- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - Three batch-run records, each with 23 `vm_compile_blocks` measurements, `run_seconds` by host exit kind, and the
>   measurements of Sprint 240, all `cold`, with their conditions.

## 3. Delta

### emits_check

- fired, not named in Emits: `COMPARISON_COMPLETED` (3), `EXECUTION_COMPLETED` (3), `EXECUTION_STARTED` (3), `MEASUREMENT_RECORDED` (129), `SESSION_COMPLETED` (3), `SESSION_INIT` (3)
- the card's Emits names no tag, and the sprint's records fired the ones above

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 246: its Rubber Duck Pass and close.
