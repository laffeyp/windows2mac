# SIGNAL_REPORT — Sprint 249 — Compiles by image

Made by `signal_report 249` from 3 records under store/runs/ whose baseline names one of the sprint's 4 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 249.

## 1. Observed

### scope_confirmation

> Plan step 5's gate: "the batch run conforms with the cache, and compiles at run time, counted on the record, fall by
> the share the cache covers" (`docs/rust-port-plan-2026-10-24.md` 158-160), the cache UnityPlayer.dll's first. The
> record counts the in-VM compiles by cause (274,834 "compiled", Sprint 246) and not by the image they come from, so the
> share UnityPlayer.dll's code holds, the share a cache of it covers, is on no record. A gate's quantity is on the record
> before the change that moves it (KIT_DIARY #100).

### work_performed

- 4536bbea9 Sprint 249: overlapping images refused by the count by image
- 9e3b5b13a Sprint 249: three cold records run alone, the compiles by image on each
- e6b0bd61c Sprint 249: the in-VM compiles counted by the image that holds each block
- 6617e3ace Sprint 249 card: compiles by image

### signal_trace

```
w2m-vm-game-1793388575000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 48, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   95.135  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   95.135  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793388671122054000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   95.135  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933886711220540
  t=   95.135  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933886711220540
  t=   95.135  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933886711220540
  t=   95.137  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=94.922, execution_id=frontier-643a6283510c-1793388671122054000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   95.470  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793388671122054000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   95.474  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=95.024, outcome=completed)
  ... MEASUREMENT_RECORDED 45 more
w2m-vm-game-1793388671000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 48, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   96.293  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   96.293  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793388767975633000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   96.293  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933887679756330
  t=   96.293  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933887679756330
  t=   96.293  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933887679756330
  t=   96.295  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=96.278, execution_id=frontier-643a6283510c-1793388767975633000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   96.384  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793388767975633000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   96.384  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=96.364, outcome=completed)
  ... MEASUREMENT_RECORDED 45 more
w2m-vm-game-1793388768000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 48, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   96.426  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   96.426  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793388864711294000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   96.426  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933888647112940
  t=   96.426  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933888647112940
  t=   96.426  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17933888647112940
  t=   96.428  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=96.411, execution_id=frontier-643a6283510c-1793388864711294000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   96.520  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793388864711294000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   96.520  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=96.501, outcome=completed)
  ... MEASUREMENT_RECORDED 45 more
```

### signal_trace_tags

- `COMPARISON_COMPLETED`
- `EXECUTION_COMPLETED`
- `EXECUTION_STARTED`
- `MEASUREMENT_RECORDED`
- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - Three batch-run records, each with Sprint 246's measurements and `vm_compile_blocks` by image.

## 3. Delta

### emits_check

- fired, not named in Emits: `COMPARISON_COMPLETED` (3), `EXECUTION_COMPLETED` (3), `EXECUTION_STARTED` (3), `MEASUREMENT_RECORDED` (144), `SESSION_COMPLETED` (3), `SESSION_INIT` (3)
- the card's Emits names no tag, and the sprint's records fired the ones above

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 249: its Rubber Duck Pass and close.
