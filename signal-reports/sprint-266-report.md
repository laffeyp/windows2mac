# SIGNAL_REPORT — Sprint 266 — The compile burst counted on the guest's own clock

Made by `signal_report 266` from 4 records under store/runs/ whose baseline names one of the sprint's 6 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 266.

## 1. Observed

### scope_confirmation

> Card 5's gate (`docs/design-translation-cache-2026-10-30.md` 170-173) reads the most compiling in a 60 Hz frame's span,
> in `CNTVCT_EL0` ticks (`monitor_code.rs` 313-316), and the virtual counter runs while the host holds the vCPU off-core,
> so the figure moves with the machine's load (reviewer-3's Medium at 264: 14.79 to 14.85 ms at load 3.3 to 4.2, 11.86 ms
> at load near 2, the same uncached guest). This card gives the gate a figure the guest's instructions fix: the most
> compiles inside any window of W instructions retired, installs counted apart, written in the design before any record
> is read.

### work_performed

- 977dca5b2 Sprint 266: four records of the bursts in a frame of instructions, and card 5's gate read on them
- 8c5795ca8 Sprint 266: the record check takes the bursts in a frame of instructions; the record script refuses a directory that exists
- 74b9dc5bb Sprint 266: the record script passes --instructions-at-compile and names it in the baseline
- 22757427f Sprint 266: the instructions retired at each in-VM compile and install, and the most in a frame of them
- e67bf4ee4 Sprint 266: card 5's frame gate counted in instructions retired
- 3f5098166 Sprint 266 card: the compile burst counted on the guest's own clock

### signal_trace

```
w2m-vm-game-1793423671000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   98.508  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   98.508  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793423770968857000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   98.508  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934237709688570
  t=   98.508  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934237709688570
  t=   98.508  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934237709688570
  t=   98.510  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=98.323, execution_id=frontier-643a6283510c-1793423770968857000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   98.608  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793423770968857000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   98.608  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=98.415, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793423788000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   96.051  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   96.051  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793423884589154000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   96.051  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934238845891540
  t=   96.051  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934238845891540
  t=   96.051  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934238845891540
  t=   96.053  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=96.037, execution_id=frontier-643a6283510c-1793423884589154000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   96.143  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793423884589154000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   96.143  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=96.124, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793423896000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   99.052  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   99.053  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793423995448693000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   99.053  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934239954486930
  t=   99.053  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934239954486930
  t=   99.053  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934239954486930
  t=   99.054  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=99.037, execution_id=frontier-643a6283510c-1793423995448693000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   99.144  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793423995448693000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   99.144  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=99.123, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793424005000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   99.719  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   99.719  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793424105438074000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   99.719  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934241054380740
  t=   99.719  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934241054380740
  t=   99.720  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934241054380740
  t=   99.721  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=99.704, execution_id=frontier-643a6283510c-1793424105438074000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   99.810  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793424105438074000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   99.810  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=99.79, outcome=completed)
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

> - Two `vm_compile_blocks` variants on the record.

## 3. Delta

### emits_check

- fired, not named in Emits: `COMPARISON_COMPLETED` (4), `EXECUTION_COMPLETED` (4), `EXECUTION_STARTED` (4), `MEASUREMENT_RECORDED` (204), `SESSION_COMPLETED` (4), `SESSION_INIT` (4)
- the card's Emits names no tag, and the sprint's records fired the ones above

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 266: its Rubber Duck Pass and close.
