# SIGNAL_REPORT — Sprint 257 — The host installs cached blocks

Made by `signal_report 257` from 9 records under store/runs/ whose baseline names one of the sprint's 7 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 257.

## 1. Observed

### scope_confirmation

> Card 4 of `docs/design-translation-cache-2026-10-30.md` as revised (G, H): the game run with `--cache` sends each
> miss in UnityPlayer.dll's range to the host before the in-VM compiler, and the host installs the block from the cache
> file, relocated into the code area with a host compile's bookkeeping, or compiles it as today when the cache does not
> hold it. The design's card gates as amended (reviewer-3's Medium 1 at 256) are in this card's commit.

### work_performed

- 2cada5aa4 Sprint 257: six timed records alike, judge off, three without and three with the cache
- a202b049a Sprint 257: the block judge's host time counted apart; timed records unjudged
- 2cb9047f3 Sprint 257: three timed batch-run records with the cache
- 1b3ebdb48 Sprint 257: a record with installs holds no compile in the cached image
- b1135620d Sprint 257: card 4's write count read over every page of the image
- 67c09917b Sprint 257: the host installs cached blocks
- a65af557f Sprint 257 card: the host installs cached blocks

### signal_trace

```
w2m-vm-game-1793401027000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=  278.777  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=  278.777  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793401307157015000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=  278.777  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934013071570150
  t=  278.777  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934013071570150
  t=  278.777  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934013071570150
  t=  278.779  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=278.761, execution_id=frontier-643a6283510c-1793401307157015000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c442
  t=  279.096  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793401307157015000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=  279.096  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=278.855, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793401307000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=  264.573  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=  264.573  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793401572460026000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=  264.573  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934015724600260
  t=  264.573  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934015724600260
  t=  264.573  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934015724600260
  t=  264.575  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=264.558, execution_id=frontier-643a6283510c-1793401572460026000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c442
  t=  264.664  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793401572460026000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=  264.664  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=264.644, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793401572000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=  279.488  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=  279.488  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793401852261858000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=  279.488  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934018522618580
  t=  279.488  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934018522618580
  t=  279.488  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934018522618580
  t=  279.489  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=279.454, execution_id=frontier-643a6283510c-1793401852261858000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c442
  t=  279.579  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793401852261858000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=  279.579  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=279.54, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793403313000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   99.144  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   99.144  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793403413242572000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   99.144  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934034132425720
  t=   99.144  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934034132425720
  t=   99.144  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934034132425720
  t=   99.146  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=99.128, execution_id=frontier-643a6283510c-1793403413242572000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   99.442  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793403413242572000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   99.443  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=99.217, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793403413000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=  278.684  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=  278.684  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793403692764192000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=  278.684  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934036927641920
  t=  278.684  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934036927641920
  t=  278.684  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934036927641920
  t=  278.686  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=278.668, execution_id=frontier-643a6283510c-1793403692764192000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c442
  t=  278.775  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793403692764192000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=  278.775  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=278.754, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793403692000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   99.017  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   99.017  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793403792094637000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   99.017  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934037920946370
  t=   99.017  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934037920946370
  t=   99.017  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934037920946370
  t=   99.019  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=98.998, execution_id=frontier-643a6283510c-1793403792094637000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   99.107  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793403792094637000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   99.108  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=99.084, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793403792000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=  278.323  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=  278.323  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793404070733215000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=  278.323  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934040707332150
  t=  278.323  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934040707332150
  t=  278.323  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934040707332150
  t=  278.324  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=278.286, execution_id=frontier-643a6283510c-1793404070733215000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c442
  t=  278.413  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793404070733215000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=  278.413  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=278.372, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793404070000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   98.542  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=   98.542  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793404169588837000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=   98.542  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934041695888370
  t=   98.542  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934041695888370
  t=   98.542  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934041695888370
  t=   98.544  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=98.519, execution_id=frontier-643a6283510c-1793404169588837000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c4429
  t=   98.633  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793404169588837000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=   98.633  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=98.605, outcome=completed)
  ... MEASUREMENT_RECORDED 48 more
w2m-vm-game-1793404169000000000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 51, EXECUTION_COMPLETED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=  279.385  w2m_vm_game -> SESSION_INIT (contract_version=0.12, harness_version=0.1.0, profile=w2m-vm, purpose=application, vocab_version=0.12)
  t=  279.386  w2m_vm_game -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1793404449286386000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=  279.386  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934044492863860
  t=  279.386  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934044492863860
  t=  279.386  w2m_vm_game -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-17934044492863860
  t=  279.387  w2m_vm_game -> EXECUTION_COMPLETED (elapsed_seconds=279.365, execution_id=frontier-643a6283510c-1793404449286386000, exit_code=None, exit_status=killed, guest_line_count=0, guest_trace=False, trace_sha256=sha256:e3b0c442
  t=  279.477  call_compare -> COMPARISON_COMPLETED (comparison_id=cmp-set-frontier-643a6283510c-1793404449286386000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + g
  t=  279.477  call_compare -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=279.452, outcome=completed)
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

> - Three timed batch-run records with the cache: Sprint 248's measurements, the compiles by image, and the installs.

## 3. Delta

### emits_check

- fired, not named in Emits: `COMPARISON_COMPLETED` (9), `EXECUTION_COMPLETED` (9), `EXECUTION_STARTED` (9), `MEASUREMENT_RECORDED` (459), `SESSION_COMPLETED` (9), `SESSION_INIT` (9)
- the card's Emits names no tag, and the sprint's records fired the ones above

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 257: its Rubber Duck Pass and close.
