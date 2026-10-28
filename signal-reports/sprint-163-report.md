# SIGNAL_REPORT — Sprint 163 — An exit's time by kind, and whether a guest WFE exits, on a record

Made by `signal_report 163` from 1 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 163.

## 1. Observed

### scope_confirmation

> Two of step 3's three measurements (`docs/rust-port-plan-2026-10-24.md` step 3, amended 2026-10-27; reviewer-2's High
> 1; Decision 2026-10-27), on a substrate record as MEASUREMENT_RECORDED with vocabulary 0.11's metrics (P24). The third,
> the runtime word's hold and wait times, needs timestamps inside the monitor code's lock and unlock and is Sprint 164.

### work_performed

- bcb9618f3 Sprint 163: an exit's time by kind, and whether a guest WFE exits, on a record
- 5d9983eb8 Sprint 163: its card, before its code (an exit's time and WFE)

### signal_trace

```
w2m-vm-measurements-1793169234000000000: SESSION_INIT 1, MEASUREMENT_RECORDED 17, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    1.213  run_measurements -> SESSION_INIT (contract_version=0.0.0, harness_version=0.1.0, profile=w2m-vm-measurements, purpose=measurement, vocab_version=0.11)
  t=    1.214  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=qpc-exits-hvc-1, metric=vm_exits, phase=discovery, 
  t=    1.214  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=qpc-exit-ns-p50-hvc-1, metric=exit_ns_p50, phase=di
  t=    1.214  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=warm, backend=managed_vm, destination_profile_id=w2m-vm, measurement_id=qpc-exit-ns-p99-hvc-1, metric=exit_ns_p99, phase=di
  t=    1.215  run_measurements -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=0.918, outcome=completed)
  ... MEASUREMENT_RECORDED 14 more
```

### signal_trace_tags

- `MEASUREMENT_RECORDED`
- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - On `store/runs/w2m-vm-measurements-<ns>`: SESSION_INIT (purpose `measurement`), MEASUREMENT_RECORDED (`exit_ns_p50`,
>   `exit_ns_p99` and `vm_exits` per guest and exit kind; `wfe_exits` for `wfe` and `wfi`), SESSION_COMPLETED
>   (completed).

## 3. Delta

### emits_check

- `MEASUREMENT_RECORDED`: fired 17 times
- `SESSION_COMPLETED`: fired 1 time
- `SESSION_INIT`: fired 1 time

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 163: its Rubber Duck Pass and close.
