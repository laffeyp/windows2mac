# SIGNAL_REPORT — Sprint 164 — The runtime word's hold and wait times, on a record

Made by `signal_report 164` from 1 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 164.

## 1. Observed

### scope_confirmation

> The third of step 3's measurements (`docs/rust-port-plan-2026-10-24.md` step 3, amended 2026-10-27; reviewer-2's
> High 1): how long the runtime word (`RUN_TABLE + RUNTIME_LOCK`, `monitor_code.rs` 27; `vm_backend.py` 518) is held per
> taking, and how long a taker waits for it, as `runtime_word_hold_ns_p50/p99` and `runtime_word_wait_ns_p50/p99`
> (vocabulary 0.11, P24). The host design budgets 200 ns a hold and revisits at a p99 wait over 10 µs
> (`docs/design-rust-host-2026-10-24.md` 131-136; Decision 2026-10-24).

### work_performed

- bf02b3784 Sprint 164: the runtime word's hold and wait times, by site, on a record
- 0902ec126 Sprint 164: its card, before its code (the runtime word's hold times)

### signal_trace

```
w2m-vm-measurements-1793171817000000000: SESSION_INIT 1, MEASUREMENT_RECORDED 41, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    1.841  run_measurements -> SESSION_INIT (contract_version=0.0.0, harness_version=0.1.0, profile=w2m-vm-measurements, purpose=measurement, vocab_version=0.11)
  t=    1.841  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=qpc-exits-hvc-1, metric=vm_exits, phase=discovery, 
  t=    1.841  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=qpc-exit-ns-p50-hvc-1, metric=exit_ns_p50, phase=di
  t=    1.841  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=qpc-exit-ns-p99-hvc-1, metric=exit_ns_p99, phase=di
  t=    1.843  run_measurements -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=1.609, outcome=completed)
  ... MEASUREMENT_RECORDED 38 more
```

### signal_trace_tags

- `MEASUREMENT_RECORDED`
- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - On `store/runs/w2m-vm-measurements-<ns>`: SESSION_INIT (purpose `measurement`), MEASUREMENT_RECORDED (Sprint 163's
>   metrics, and `runtime_word_hold_ns_p50`, `runtime_word_hold_ns_p99`, `runtime_word_wait_ns_p50`,
>   `runtime_word_wait_ns_p99` per guest), SESSION_COMPLETED (completed).

## 3. Delta

### emits_check

- `MEASUREMENT_RECORDED`: fired 41 times
- `SESSION_COMPLETED`: fired 1 time
- `SESSION_INIT`: fired 1 time

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 164: its Rubber Duck Pass and close.
