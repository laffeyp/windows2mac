# SIGNAL_REPORT — Sprint 247 — The in-VM compile time

Made by `signal_report 247` from 3 records under store/runs/ whose baseline names one of the sprint's 14 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 247.

## 1. Observed

### scope_confirmation

> Reviewer-3's Medium 1 of its review at 246 and the Decision "Sequence for step 5" as annotated after it: the revisit
> clause ("the run compiles too few blocks at run time for the cache to move the rate") turns on the share of the run's
> time spent compiling; Sprint 246 measured the host's share, 53.2 to 80.7 ms of about 28 s, and not the in-VM
> compiler's, which lies inside `run_seconds` "vm" (census row 15d, partial).

### work_performed

- 2577a0557 Sprint 247: a temporary name left by a killed process is passed over
- 35687d79b Sprint 247: a value in parentheses is not a plant's number
- ae0662289 Sprint 247: a test holds the compile time's writer
- 41310876e Sprint 247: c2fe0bbc8 named in signal_report's BETWEEN_SPRINTS; three working-agreement rules; kit diary 103-104
- 3400f1e46 Sprint 247: the probe's slot and the frequency reader built on every target
- 0dfb1f4bd Sprint 247: census row 15d from the timed records; census.rs names a withdrawn record
- e78f272b1 Sprint 247: three timed records, the compile time from the monitor code
- 13ab71c9a Sprint 247: the compile probe off by default and its size held
- 1df116e4b Sprint 247: step 5's order ruled before the timed records' share is read; Deferred for step 6's counters
- 4f6a736c2 Sprint 247: the compile time timed in the monitor code, the C runtime unchanged
- 387bafff3 Sprint 247: census row 15d met; its Deferred line resolved
- ef904c7e5 Sprint 247: the batch run measured three times with its in-VM compile time
- c000817d3 Sprint 247: the in-VM compile time on the record
- a4e1d903d Sprint 247 card: the in-VM compile time

### signal_trace

```
w2m-vm-game-1793381341000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 44, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=  103.496  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=  103.496  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793381445644820000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=  103.496  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933814456448200
  t=  103.496  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933814456448200
  t=  103.496  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933814456448200
  t=  103.497  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=103.251, execution_id=frontier-643a6283510c-1793381445644820000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c442
  t=  103.846  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793381445644820000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=  103.846  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=103.348, outcome=completed)
  ... MEASUREMENT_RECORDED 41 more
w2m-vm-game-1793381446000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 44, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=  107.374  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=  107.374  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793381553886949000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=  107.374  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933815538869490
  t=  107.374  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933815538869490
  t=  107.374  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933815538869490
  t=  107.375  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=107.357, execution_id=frontier-643a6283510c-1793381553886949000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c442
  t=  107.470  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793381553886949000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=  107.470  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=107.448, outcome=completed)
  ... MEASUREMENT_RECORDED 41 more
w2m-vm-game-1793381554000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 44, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=  100.739  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=  100.739  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793381654980842000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=  100.739  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933816549808420
  t=  100.739  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933816549808420
  t=  100.739  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933816549808420
  t=  100.741  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=100.722, execution_id=frontier-643a6283510c-1793381654980842000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c442
  t=  100.836  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793381654980842000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=  100.836  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=100.814, outcome=completed)
  ... MEASUREMENT_RECORDED 41 more
```

### signal_trace_tags

- `COMPARISON_COMPLETED`
- `EXECUTION_COMPLETED`
- `EXECUTION_STARTED`
- `MEASUREMENT_RECORDED`
- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - Three batch-run records, each with `compile_critical_path_ms` "vm" and Sprint 246's measurements.

## 3. Delta

### emits_check

- fired, not named in Emits: `COMPARISON_COMPLETED` (3), `EXECUTION_COMPLETED` (3), `EXECUTION_STARTED` (3), `MEASUREMENT_RECORDED` (132), `SESSION_COMPLETED` (3), `SESSION_INIT` (3)
- the card's Emits names no tag, and the sprint's records fired the ones above

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 247: its Rubber Duck Pass and close.
