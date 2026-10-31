# SIGNAL_REPORT — Sprint 263 — The installer inside the VM

Made by `signal_report 263` from 6 records under store/runs/ whose baseline names one of the sprint's 6 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 263.

## 1. Observed

### scope_confirmation

> Card 5 of `docs/design-translation-cache-2026-10-30.md`, its installer (choices III to VII; the file in the guest is
> Sprint 261's): with a cache, a miss in UnityPlayer.dll's range is installed inside the VM from the mapped file, with
> no exit, the host learning of it from the compile log. The reviews at 260 and 261 are disposed (Sprint 261's close).

### work_performed

- 26a531615 Sprint 263: six timed records, three with the installer inside the VM
- c28554525 Sprint 263: a test holds the installer after the cached blocks' exits
- da9348db9 Sprint 263: choice I's figure at version 2
- 72fc65bdd Sprint 263: the installer leaves the call trace's first block to the host
- 229734920 Sprint 263: the installer inside the VM
- bd2ec8098 Sprint 263 card: the installer inside the VM

### signal_trace

```
w2m-vm-game-1793419013000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   26.336  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   26.336  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793419040071591000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   26.336  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934190400715910
  t=   26.336  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934190400715910
  t=   26.336  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934190400715910
  t=   26.337  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=26.321, execution_id=frontier-643a6283510c-1793419040071591000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   26.584  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793419040071591000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   26.584  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=26.41, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793419040000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   25.618  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   25.618  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793419066291387000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   25.618  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934190662913870
  t=   25.618  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934190662913870
  t=   25.618  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934190662913870
  t=   25.620  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=25.603, execution_id=frontier-643a6283510c-1793419066291387000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   25.709  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793419066291387000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   25.709  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=25.689, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793419066000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   26.128  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   26.128  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793419092715737000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   26.129  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934190927157370
  t=   26.129  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934190927157370
  t=   26.129  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934190927157370
  t=   26.130  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=26.108, execution_id=frontier-643a6283510c-1793419092715737000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   26.220  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793419092715737000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   26.220  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=26.195, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793419092000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   25.740  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   25.740  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793419118746266000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   25.741  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934191187462660
  t=   25.741  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934191187462660
  t=   25.741  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934191187462660
  t=   25.742  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=25.72, execution_id=frontier-643a6283510c-1793419118746266000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c44298
  t=   25.830  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793419118746266000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   25.830  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=25.805, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793419118000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   26.118  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   26.118  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793419145166499000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   26.118  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934191451664990
  t=   26.118  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934191451664990
  t=   26.118  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934191451664990
  t=   26.120  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=26.1, execution_id=frontier-643a6283510c-1793419145166499000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c44298f
  t=   26.211  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793419145166499000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   26.211  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=26.188, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793419145000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   25.534  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   25.534  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793419170994661000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   25.534  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934191709946610
  t=   25.534  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934191709946610
  t=   25.534  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934191709946610
  t=   25.535  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=25.515, execution_id=frontier-643a6283510c-1793419170994661000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   25.624  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793419170994661000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   25.624  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=25.601, outcome=completed)
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

> - Six timed batch-run records, three with the cache: the sequence of Sprint 262's, the installs now inside the VM.

## 3. Delta

### emits_check

- fired, not named in Emits: `COMPARISON_COMPLETED` (6), `EXECUTION_COMPLETED` (6), `EXECUTION_STARTED` (6), `MEASUREMENT_RECORDED` (306), `SESSION_COMPLETED` (6), `SESSION_INIT` (6)
- the card's Emits names no tag, and the sprint's records fired the ones above

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 263: its Rubber Duck Pass and close.
