# SIGNAL_REPORT — Sprint 368 — G3's runnable part: the game's batch run on two vCPUs, conforming as on one

Made by `signal_report 368` from 5 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits or the commit that holds this report added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 368.

## 1. Observed

### scope_confirmation

> G3 (`docs/step-6-census-2026-10-30.md` 20): "the batch run on the wall clock on two vCPUs conforming, in several runs".
> The Decision "Step 6's order after G2, weighed again at Sprint 366" puts it here, before C2's build cards, so that the
> game itself judges every change after it; G3 by Sprint 370. Sprint 367 made the batch run run on two vCPUs on the wall
> clock and stop by the reference's rule, in two runs.

### work_performed

- f6d228be8 Sprint 368 card: G3's runnable part, the game's batch run on two vCPUs conforming as on one

### signal_trace

```
w2m-vm-game-1795802342000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 54, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=  233.611  w2m_vm_game -> SESSION_INIT (contract_version=0.14, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.14)
  t=  233.611  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1795802576737828000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=  233.611  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=frontier-643a62
  t=  233.611  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=frontier-643a62
  t=  233.611  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=frontier-643a62
  t=  233.613  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=233.46, execution_id=frontier-643a6283510c-1795802576737828000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=  233.704  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1795802576737828000, contract_id=windows.game.calls, contract_version=0.2.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=  233.704  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=233.548, outcome=completed)
  ... MEASUREMENT_RECORDED 51 more
w2m-vm-game-1795802591000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 54, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=  176.698  w2m_vm_game -> SESSION_INIT (contract_version=0.14, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.14)
  t=  176.698  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1795802768791724000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=  176.698  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=frontier-643a62
  t=  176.698  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=frontier-643a62
  t=  176.698  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=frontier-643a62
  t=  176.700  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=176.655, execution_id=frontier-643a6283510c-1795802768791724000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c442
  t=  176.783  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1795802768791724000, contract_id=windows.game.calls, contract_version=0.2.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=  176.784  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=176.736, outcome=completed)
  ... MEASUREMENT_RECORDED 51 more
w2m-vm-game-1795802779000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 54, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=  176.686  w2m_vm_game -> SESSION_INIT (contract_version=0.14, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.14)
  t=  176.686  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1795802956744456000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=  176.686  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=frontier-643a62
  t=  176.686  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=frontier-643a62
  t=  176.686  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=frontier-643a62
  t=  176.688  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=176.645, execution_id=frontier-643a6283510c-1795802956744456000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c442
  t=  176.770  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1795802956744456000, contract_id=windows.game.calls, contract_version=0.2.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=  176.770  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=176.724, outcome=completed)
  ... MEASUREMENT_RECORDED 51 more
w2m-vm-game-1795802967000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 54, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=  175.226  w2m_vm_game -> SESSION_INIT (contract_version=0.14, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.14)
  t=  175.226  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1795803143174162000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=  175.226  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=frontier-643a62
  t=  175.226  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=frontier-643a62
  t=  175.226  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=frontier-643a62
  t=  175.228  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=175.185, execution_id=frontier-643a6283510c-1795803143174162000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c442
  t=  175.310  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1795803143174162000, contract_id=windows.game.calls, contract_version=0.2.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=  175.310  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=175.264, outcome=completed)
  ... MEASUREMENT_RECORDED 51 more
w2m-vm-game-1795803156000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 54, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=  179.227  w2m_vm_game -> SESSION_INIT (contract_version=0.14, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.14)
  t=  179.227  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1795803335910549000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=  179.227  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=frontier-643a62
  t=  179.227  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=frontier-643a62
  t=  179.227  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=frontier-643a62
  t=  179.229  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=179.185, execution_id=frontier-643a6283510c-1795803335910549000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c442
  t=  179.315  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1795803335910549000, contract_id=windows.game.calls, contract_version=0.2.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=  179.315  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=179.268, outcome=completed)
  ... MEASUREMENT_RECORDED 51 more
```

### signal_trace_tags

- `COMPARISON_COMPLETED`
- `EXECUTION_COMPLETED`
- `EXECUTION_STARTED`
- `MEASUREMENT_RECORDED`
- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - None: the game's batch run on two vCPUs through the existing harness and its Producers; no emit site changes. The
>   one-vCPU run's records are unchanged; a two-vCPU run's record carries `vcpus` 2 (Sprint 367).

## 3. Delta

### emits_check

- fired, not named in Emits: `COMPARISON_COMPLETED` (5), `EXECUTION_COMPLETED` (5), `EXECUTION_STARTED` (5), `MEASUREMENT_RECORDED` (270), `SESSION_COMPLETED` (5), `SESSION_INIT` (5)
- the card's Emits names no tag, and the sprint's records fired the ones above

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 368: its Rubber Duck Pass and close.
