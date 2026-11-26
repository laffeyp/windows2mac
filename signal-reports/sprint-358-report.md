# SIGNAL_REPORT — Sprint 358 — Each leaf lock's takings, hold and wait on the record

Made by `signal_report 358` from 2 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits or the commit that holds this report added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 358.

## 1. Observed

### scope_confirmation

> Vocabulary 0.14 (Sprint 357) gave a record the words for each leaf lock's takings, hold and wait. No code writes them
> yet. Change 1's four leaf locks carry the waits, releases, creates and heap calls that left the runtime word in
> Sprints 343, 345, 355 and 356, and G6 asks for "each lock's hold and wait". The runtime word's own wait figures on two
> vCPUs are a lower bound: one shared wait-start word is overwritten by a second waiter (Sprint 356, 4,349 of 16,384
> waits before its change and 146 after starting past their taking; the Decision "Where the timing area's wait start per
> vCPU goes" puts the fix here). This card writes every leaf lock's figures, and fixes that.

### work_performed

- e865af826 Sprint 358 card: each leaf lock's takings, hold and wait on the record; the runtime word's wait start per vCPU

### signal_trace

```
w2m-vm-measurements-1795724698000000000: SESSION_INIT 1, MEASUREMENT_RECORDED 99, CAPABILITY_PROBED 3, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.871  run_measurements -> SESSION_INIT (contract_version=0.0.0, harness_version=0.1.0, profile=w2m-vm-measurements, purpose=measurement, vocab_version=0.14)
  t=    3.871  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=qpc-exits-hvc-1, metric=vm_
  t=    3.871  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=qpc-exit-ns-p50-hvc-1, metr
  t=    3.871  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=qpc-exit-ns-p99-hvc-1, metr
  t=    3.874  run_measurements -> CAPABILITY_PROBED (capability=feat_lse2, detail=hw.optional.arm.FEAT_LSE2 is 1 (sysctlbyname), ..., execution_id=None, host_id=mac-Mac17,7-macos-25F80, outcome=present)
  t=    3.874  run_measurements -> CAPABILITY_PROBED (capability=feat_bbm_level2, detail=FEAT_BBM level 0: ID_AA64MMFR2_EL1 0x1001001102..., execution_id=None, host_id=mac-Mac17,7-macos-25F80, outcome=absent)
  t=    3.874  run_measurements -> CAPABILITY_PROBED (capability=guest_wfe_exit, detail=the WFE probe's run of 1,000 WFE ended at its h..., execution_id=None, host_id=mac-Mac17,7-macos-25F80, outcome=absent)
  t=    3.874  run_measurements -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=3.647, outcome=completed)
  ... MEASUREMENT_RECORDED 96 more
w2m-vm-measurements-1795725897000000000: SESSION_INIT 1, MEASUREMENT_RECORDED 107, CAPABILITY_PROBED 3, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    4.515  run_measurements -> SESSION_INIT (contract_version=0.0.0, harness_version=0.1.0, profile=w2m-vm-measurements, purpose=measurement, vocab_version=0.14)
  t=    4.515  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=qpc-exits-hvc-1, metric=vm_
  t=    4.515  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=qpc-exit-ns-p50-hvc-1, metr
  t=    4.515  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=qpc-exit-ns-p99-hvc-1, metr
  t=    4.519  run_measurements -> CAPABILITY_PROBED (capability=feat_lse2, detail=hw.optional.arm.FEAT_LSE2 is 1 (sysctlbyname), ..., execution_id=None, host_id=mac-Mac17,7-macos-25F80, outcome=present)
  t=    4.519  run_measurements -> CAPABILITY_PROBED (capability=feat_bbm_level2, detail=FEAT_BBM level 0: ID_AA64MMFR2_EL1 0x1001001102..., execution_id=None, host_id=mac-Mac17,7-macos-25F80, outcome=absent)
  t=    4.519  run_measurements -> CAPABILITY_PROBED (capability=guest_wfe_exit, detail=the WFE probe's run of 1,000 WFE ended at its h..., execution_id=None, host_id=mac-Mac17,7-macos-25F80, outcome=absent)
  t=    4.519  run_measurements -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=4.307, outcome=completed)
  ... MEASUREMENT_RECORDED 104 more
```

### signal_trace_tags

- `CAPABILITY_PROBED`
- `MEASUREMENT_RECORDED`
- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - MEASUREMENT_RECORDED on the committed record, with `vcpus` 2:
>   - for each of `heap`, `slots`, `handles` and `sync_objects` as `lock`: `lock_takings`, `lock_hold_ns_p50`,
>     `lock_hold_ns_p99`, `lock_wait_ns_p50` and `lock_wait_ns_p99`;
>   - `runtime_word_takings`, beside `runtime_word_hold_ns_*` and `runtime_word_wait_ns_*`.

## 3. Delta

### emits_check

- `MEASUREMENT_RECORDED`: fired 206 times
- fired, not named in Emits: `CAPABILITY_PROBED` (6), `SESSION_COMPLETED` (2), `SESSION_INIT` (2)

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 358: its Rubber Duck Pass and close.
