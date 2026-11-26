# SIGNAL_REPORT — Sprint 342 — Seven calls take no runtime word

Made by `signal_report 342` from 1 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits or the commit that holds this report added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 342.

## 1. Observed

### scope_confirmation

> This card picks up Change 1's None class (`docs/design-rust-host-2026-10-24.md` 95-140), after Sprint 339's proof and
> Sprint 341's ring. The Decision "Sprint 339's None class stopped at its proof, and the order after it" (2026-11-25)
> names this card. Sprint 339 proved seven calls touch only the calling thread's own state, the shared call ring apart:

### work_performed

- 99b63cb80 Sprint 342 card: seven calls take no runtime word; Sprint 341's late Decision and Deferred

### signal_trace

```
w2m-vm-measurements-1795665127000000000: SESSION_INIT 1, MEASUREMENT_RECORDED 44, CAPABILITY_PROBED 3, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    1.984  run_measurements -> SESSION_INIT (contract_version=0.0.0, harness_version=0.1.0, profile=w2m-vm-measurements, purpose=measurement, vocab_version=0.13)
  t=    1.984  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, measurement_id=qpc-exits-hvc-1, metric=vm_exits, phas
  t=    1.984  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, measurement_id=qpc-exit-ns-p50-hvc-1, metric=exit_ns_
  t=    1.984  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, measurement_id=qpc-exit-ns-p99-hvc-1, metric=exit_ns_
  t=    1.986  run_measurements -> CAPABILITY_PROBED (capability=feat_lse2, detail=hw.optional.arm.FEAT_LSE2 is 1 (sysctlbyname), ..., execution_id=None, host_id=mac-Mac17,7-macos-25F80, outcome=present)
  t=    1.986  run_measurements -> CAPABILITY_PROBED (capability=feat_bbm_level2, detail=FEAT_BBM level 0: ID_AA64MMFR2_EL1 0x1001001102..., execution_id=None, host_id=mac-Mac17,7-macos-25F80, outcome=absent)
  t=    1.986  run_measurements -> CAPABILITY_PROBED (capability=guest_wfe_exit, detail=the WFE probe's run of 1,000 WFE ended at its h..., execution_id=None, host_id=mac-Mac17,7-macos-25F80, outcome=absent)
  t=    1.986  run_measurements -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=1.668, outcome=completed)
  ... MEASUREMENT_RECORDED 41 more
```

### signal_trace_tags

- `CAPABILITY_PROBED`
- `MEASUREMENT_RECORDED`
- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - The new measurements record: MEASUREMENT_RECORDED `runtime_word_hold_ns_p50` and `runtime_word_hold_ns_p99` for
>   `qpc`, their `variant` naming the takings, and the rest of Sprint 337's record in its form.

## 3. Delta

### emits_check

- `MEASUREMENT_RECORDED`: fired 44 times
- fired, not named in Emits: `CAPABILITY_PROBED` (3), `SESSION_COMPLETED` (1), `SESSION_INIT` (1)

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 342: its Rubber Duck Pass and close.
