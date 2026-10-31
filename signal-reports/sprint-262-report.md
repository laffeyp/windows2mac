# SIGNAL_REPORT — Sprint 262 — Timed records without the ledger

Made by `signal_report 262` from 6 records under store/runs/ whose baseline names one of the sprint's 5 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 262.

## 1. Observed

### scope_confirmation

> reviewer-3's Medium 1 at 261 (Deferred, Sprint 259, as annotated at 261's close): a timed batch record keeps the exit
> ledger, whose per-exit hashing grows with exits, so a comparison that changes the exit count is confounded; Sprint
> 257's "1.1% slower" for the host's install rests on such records, 1,091 against 150,065 `hvc 1`. This card makes a
> timed record carry neither the whole-block judge nor the ledger, holds that by test, and measures card 4's time again
> on records made alike.

### work_performed

- 5e59fccf5 Sprint 262: two design lines wrapped to the page's width
- 6319fbdfc Sprint 262: card 4's time read without the ledger
- 037cef883 Sprint 262: six ledger-free timed records, three each way
- ff1926fdc Sprint 262: a timed record writes no exit ledger
- 66f1a9b73 Sprint 262 card: timed records without the ledger

### signal_trace

```
w2m-vm-game-1793415256000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   26.483  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   26.483  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793415283317534000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   26.483  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934152833175340
  t=   26.483  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934152833175340
  t=   26.483  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934152833175340
  t=   26.485  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=26.467, execution_id=frontier-643a6283510c-1793415283317534000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   26.775  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793415283317534000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   26.779  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=26.562, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793415283000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   25.788  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   25.788  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793415309744704000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   25.788  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934153097447040
  t=   25.788  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934153097447040
  t=   25.788  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934153097447040
  t=   25.789  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=25.767, execution_id=frontier-643a6283510c-1793415309744704000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   25.879  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793415309744704000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   25.879  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=25.854, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793415309000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   26.038  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   26.038  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793415336109435000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   26.038  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934153361094350
  t=   26.038  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934153361094350
  t=   26.038  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934153361094350
  t=   26.039  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=26.023, execution_id=frontier-643a6283510c-1793415336109435000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   26.129  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793415336109435000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   26.129  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=26.11, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793415336000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   25.776  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   25.776  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793415362210003000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   25.776  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934153622100030
  t=   25.776  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934153622100030
  t=   25.776  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934153622100030
  t=   25.778  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=25.754, execution_id=frontier-643a6283510c-1793415362210003000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   25.870  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793415362210003000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   25.870  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=25.843, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793415362000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   26.361  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   26.361  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793415388896243000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   26.361  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934153888962430
  t=   26.361  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934153888962430
  t=   26.361  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934153888962430
  t=   26.362  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=26.345, execution_id=frontier-643a6283510c-1793415388896243000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   26.452  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793415388896243000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   26.452  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=26.432, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793415389000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   25.852  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   25.853  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793415415076911000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   25.853  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934154150769110
  t=   25.853  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934154150769110
  t=   25.853  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934154150769110
  t=   25.854  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=25.834, execution_id=frontier-643a6283510c-1793415415076911000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   25.943  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793415415076911000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   25.943  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=25.92, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
```

### signal_trace_tags

- `COMPARISON_COMPLETED`
- `EXECUTION_COMPLETED`
- `EXECUTION_STARTED`
- `MEASUREMENT_RECORDED`
- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - Six timed batch-run records, ledger-free: the sequence of Sprint 257's with `exit_ledger` "off" in the baseline.

## 3. Delta

### emits_check

- fired, not named in Emits: `COMPARISON_COMPLETED` (6), `EXECUTION_COMPLETED` (6), `EXECUTION_STARTED` (6), `MEASUREMENT_RECORDED` (306), `SESSION_COMPLETED` (6), `SESSION_INIT` (6)
- the card's Emits names no tag, and the sprint's records fired the ones above

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 262: its Rubber Duck Pass and close.
