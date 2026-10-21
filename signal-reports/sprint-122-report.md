# SIGNAL_REPORT — Sprint 122 — processes beside several vCPUs

Made by `scripts/signal_report.py 122` from 1060 records under store/runs/ whose baseline names one of the sprint's 4 commits. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 122.

## 1. Observed

### scope_confirmation

> Step 7 of the several-vCPU chain needs the Machine driven from somewhere other than vCPU 0's wait (Sprint 120's
> open item, docs/design-several-vcpus.md). Today one Python thread drives a kernel.Machine: it runs each process's
> `VM.steps` in turn, and serves the main process's vCPU 0 inside it. When vCPU 0 has no thread while another vCPU runs
> one, it waits in `_switch` for that vCPU, and while it waits no other process runs, so w2m refuses the wait once a
> second process exists (Unimplemented: "a thread waiting on vCPU 0 for another vCPU while another process could run").
> The game always starts UnityCrashHandler64.exe, so a batch run with its threads on several vCPUs stops there. Measured
> on 1953fbfe: tests/data/timed_waits spinchild reaches the refusal in 11 runs of 20.

### work_performed

- 31efecd7f Sprint 122: the batch run and sweeps on 37784f1a; threads, running_thread_context and wait_handoff on two vCPUs
- 4b36b5adc Sprint 122: the batch run and sweeps on f1838b96; threads, running_thread_context and wait_handoff on two vCPUs
- 37784f1a7 Sprint 122: a host request reads and writes the asking vCPU's own pages
- f1838b964 Sprint 122: processes beside several vCPUs

### signal_trace

```
class-* 916 records: SESSION_INIT 916, CANDIDATE_GENERATED 916, COMPARISON_COMPLETED 1832, CANDIDATE_ACCEPTED 912, SESSION_COMPLETED 916, REDUCTION_COMPLETED 16, DISCREPANCY_REPORTED 16, DISCREPANCY_ROUTED 16; producers failed 0; not finalised 0
family-* 142 records: SESSION_INIT 142, CANDIDATE_GENERATED 142, COMPARISON_COMPLETED 554, CANDIDATE_ACCEPTED 142, SESSION_COMPLETED 142; producers failed 0; not finalised 0
frontier-643a6283510c-1792622186569209000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 104, EXECUTION_COMPLETED 1, TRACE_INGESTED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    0.001  harness -> SESSION_INIT (contract_version=0.9, harness_version=0.0.1, profile=w2m-vm, purpose=application, vocab_version=0.9)
  t=    0.204  destination_runner -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1792622186569209000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afb
  t=   33.987  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792622186
  t=   33.987  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792622186
  t=   33.987  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792622186
  t=   33.990  destination_runner -> EXECUTION_COMPLETED (elapsed_seconds=33.581, execution_id=frontier-643a6283510c-1792622186569209000, exit_code=None, exit_status=killed, guest_line_count=10535, trace_sha256=sha256:1d9c4e4b215d64f00
  t=   34.002  ingester -> TRACE_INGESTED (execution_id=frontier-643a6283510c-1792622186569209000, line_count=10535, thread_count=28, trace_sha256=sha256:1d9c4e4b215d64f00cf9d675a6fffd6f5905d834..., vocab_version=0.9)
  t=   34.329  comparator -> COMPARISON_COMPLETED (comparison_id=cmp-frontier-643a6283510c-1792622186569209000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + game-ru
  t=   34.329  closer -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=34.702, outcome=completed)
  ... MEASUREMENT_RECORDED 101 more
frontier-643a6283510c-1792623423346201000: SESSION_INIT 1, EXECUTION_STARTED 1, MEASUREMENT_RECORDED 104, EXECUTION_COMPLETED 1, TRACE_INGESTED 1, COMPARISON_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    0.001  harness -> SESSION_INIT (contract_version=0.9, harness_version=0.0.1, profile=w2m-vm, purpose=application, vocab_version=0.9)
  t=    0.214  destination_runner -> EXECUTION_STARTED (execution_id=frontier-643a6283510c-1792623423346201000, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afb
  t=   35.587  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792623423
  t=   35.587  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792623423
  t=   35.587  destination_runner -> MEASUREMENT_RECORDED (application_profile_id=hail-to-the-rainbow-643a6283510c, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=frontier-643a6283510c-1792623423
  t=   35.590  destination_runner -> EXECUTION_COMPLETED (elapsed_seconds=34.865, execution_id=frontier-643a6283510c-1792623423346201000, exit_code=None, exit_status=killed, guest_line_count=10535, trace_sha256=sha256:9b1834ee3b964586f
  t=   35.601  ingester -> TRACE_INGESTED (execution_id=frontier-643a6283510c-1792623423346201000, line_count=10535, thread_count=28, trace_sha256=sha256:9b1834ee3b964586ff944c5e4e6117178331c9b9..., vocab_version=0.9)
  t=   35.869  comparator -> COMPARISON_COMPLETED (comparison_id=cmp-frontier-643a6283510c-1792623423346201000, contract_id=windows.game.calls, contract_version=0.1.0, first_divergence=None, reference_ref=game-run-38216081440 + game-ru
  t=   35.869  closer -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=36.24, outcome=completed)
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
> - On the batch run's record, as on Sprint 121's: SESSION_INIT, EXECUTION_STARTED, MEASUREMENT_RECORDED 104 times,
>   EXECUTION_COMPLETED (exit_status killed), TRACE_INGESTED, COMPARISON_COMPLETED (first_divergence none),
>   SESSION_COMPLETED. Per class run (458) and family run (68), as on Sprint 121's sweep: SESSION_INIT,
>   CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED (456 and 68) or for FCOS and FSIN
>   REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED, SESSION_COMPLETED.
> - On the batch run on two vCPUs (scratch, `--scratch`): SESSION_INIT, EXECUTION_STARTED, MEASUREMENT_RECORDED,
>   EXECUTION_COMPLETED, and TRACE_INGESTED and COMPARISON_COMPLETED when the run reaches the log; its verdict reported
>   whatever it is.

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 1,054 times
- `CANDIDATE_GENERATED`: fired 1,058 times
- `COMPARISON_COMPLETED`: fired 2,388 times
- `DISCREPANCY_REPORTED`: fired 16 times
- `DISCREPANCY_ROUTED`: fired 16 times
- `EXECUTION_COMPLETED`: fired 2 times
- `EXECUTION_STARTED`: fired 2 times
- `MEASUREMENT_RECORDED`: fired 208 times
- `REDUCTION_COMPLETED`: fired 16 times
- `SESSION_COMPLETED`: fired 1,060 times
- `SESSION_INIT`: fired 1,060 times
- `TRACE_INGESTED`: fired 2 times

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 122: its Rubber Duck Pass and close.
