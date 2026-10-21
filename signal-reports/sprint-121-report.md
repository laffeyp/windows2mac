# SIGNAL_REPORT — Sprint 121 — a parked vCPU wakes

Made by `scripts/signal_report.py 121` from 530 records under store/runs/ whose baseline names one of the sprint's 3 commits. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 121.

## 1. Observed

### scope_confirmation

> Sprint 118's Deferred entry, made this sprint's scope by Sprint 120 under Decision 2026-10-11 user. A vCPU with no
> thread waits in the host (`_wait_for_change`: `_park` past the first, `_switch` on vCPU 0), and a thread the guest
> runtime readies inside the VM (threads.c `w2m_signal`, `w2m_ready_thread`) does not wake it: the vCPU that readied
> the thread ends its own slice (SLICE_UNTIL 0) and switches to it, and the waiting vCPU learns of it only at the next
> host serve. Measured before the change (scratch probe `readied`, built with mingw as tests/data/timed_waits are): a
> worker waits on an auto-reset event 20 times and does 8,000,000 instructions of work after each, while the main
> thread sets the event inside the VM and spins until the round is done. On two vCPUs the worker's rounds ran on vCPU 1
> in 3 runs of 13; in the other 10 vCPU 1 ran one round (8,000,032 or 8,000,042 instructions) and vCPU 0 the rest,
> switching between the two threads.

### work_performed

- 4a0f341b2 Sprint 121: the several-process refusal's test runs up to 30 times
- 204c3b42b Sprint 121: the batch run and sweeps on 1953fbfe; threads, running_thread_context and wait_handoff on two vCPUs
- 1953fbfe0 Sprint 121: a parked vCPU wakes

### signal_trace

```
class-* 458 records: SESSION_INIT 458, CANDIDATE_GENERATED 458, COMPARISON_COMPLETED 916, CANDIDATE_ACCEPTED 456, SESSION_COMPLETED 458, REDUCTION_COMPLETED 8, DISCREPANCY_REPORTED 8, DISCREPANCY_ROUTED 8; producers failed 0; not finalised 0
family-* 71 records: SESSION_INIT 71, CANDIDATE_GENERATED 71, COMPARISON_COMPLETED 277, CANDIDATE_ACCEPTED 71, SESSION_COMPLETED 71; producers failed 0; not finalised 0
frontier-643a6283510c-1792611641237977000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 104, EXECUTION_COMPLETED 1, TRACE_INGESTED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    0.001  harness -> SESSION_INIT (contract_version=0.9, harness_version=0.0.1, profile=w2m-vm, purpose=application, vocab_version=0.9)
  t=    0.201  destination_runner -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1792611641237977000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afb
  t=   36.518  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792611641
  t=   36.518  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792611641
  t=   36.518  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792611641
  t=   36.521  destination_runner -> EXECUTION_COMPLETED (elapsed_seconds=36.203, execution_id=frontier-643a6283510c-1792611641237977000, exit_code=None, exit_status=killed, guest_line_count=10535, trace_sha256=sha256:11da7b9e57ef7bcb6
  t=   36.534  ingester -> TRACE_INGESTED (execution_id=frontier-643a6283510c-1792611641237977000, line_count=10535, thread_count=28, trace_sha256=sha256:11da7b9e57ef7bcb67fd685cd97c13b7b2e8a36b..., vocab_version=0.9)
  t=   36.861  comparator -> COMPARISON_COMPLETED (comparison_id=cmp-frontier-643a6283510c-1792611641237977000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + game-ru
  t=   36.861  closer -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=37.229, outcome=completed)
  ... MEASUREMENT_RECORDED 101 more
```

### signal_trace_tags

- `CANDIDATE_ACCEPTED`
- `CANDIDATE_GENERATED`
- `COMPARISON_COMPLETED`
- `DISCREPANCY_REPORTED`
- `DISCREPANCY_ROUTED`
- `EXECUTION_COMPLETED`
- `EXECUTION_STARTED`
- `MEASUREMENT_RECORDED`
- `REDUCTION_COMPLETED`
- `SESSION_COMPLETED`
- `SESSION_INIT`
- `TRACE_INGESTED`

## 2. Expected

> - On the threads, running_thread_context and wait_handoff runs on two vCPUs (`family_run.py F --run --vcpus 2`):
>   SESSION_INIT (purpose conformance), CANDIDATE_GENERATED, COMPARISON_COMPLETED for each reference run (verdict
>   conforms, first_divergence none), CANDIDATE_ACCEPTED, SESSION_COMPLETED (outcome completed), baseline `vcpus` 2.
> - On the batch run's record, as on Sprint 120's: SESSION_INIT, EXECUTION_STARTED, MEASUREMENT_RECORDED 104 times,
>   EXECUTION_COMPLETED (exit_status killed), TRACE_INGESTED, COMPARISON_COMPLETED (first_divergence none),
>   SESSION_COMPLETED. Per class run (458) and family run (68), as on Sprint 120's sweep: SESSION_INIT,
>   CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED (456 and 68) or for FCOS and FSIN
>   REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED, SESSION_COMPLETED.

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 527 times
- `CANDIDATE_GENERATED`: fired 529 times
- `COMPARISON_COMPLETED`: fired 1,194 times
- `DISCREPANCY_REPORTED`: fired 8 times
- `DISCREPANCY_ROUTED`: fired 8 times
- `EXECUTION_COMPLETED`: fired 1 time
- `EXECUTION_STARTED`: fired 1 time
- `MEASUREMENT_RECORDED`: fired 104 times
- `REDUCTION_COMPLETED`: fired 8 times
- `SESSION_COMPLETED`: fired 530 times
- `SESSION_INIT`: fired 530 times
- `TRACE_INGESTED`: fired 1 time

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 121: its Rubber Duck Pass and close.
