# SIGNAL_REPORT — Sprint 337 — A two-vCPU run on a substrate record

Made by `signal_report 337` from 1 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits or the commit that holds this report added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 337.

## 1. Observed

### scope_confirmation

> Plan step 6's evidence is meant to be "on a record" (`docs/rust-port-plan-2026-10-24.md` 58; reviewer-4's first review
> at 331, High 1). Today the two-vCPU runs are judged only by what tests read from `w2m-vm`'s stderr and stop files. One
> example is `a_locked_model_step_loses_no_add_of_another_vcpus_aligned_lock` in `crates/w2m_vm/tests/run_loop.rs`
> (1201-1223), which reads "w2m-vm: 100000 model steps of a locked form". No substrate record carries a run's vCPU count,
> its locked steps, its turn's start, or the capabilities `vcpus_allowed` (`run.rs` 605-633) checks. Vocabulary 0.13
> (Sprint 336) gave those their words. This card puts them on a record, as the Decision "Step 6 restored to the user's fix
> (b)" orders: "`w2m-vm --vcpus N` on a substrate record, CAPABILITY_PROBED emitted where `w2m-vm` makes its refusal
> checks".

### work_performed

- b8e6bbc70 Sprint 337 card: a two-vCPU run on a substrate record

### signal_trace

```
w2m-vm-measurements-1795656377000000000: SESSION_INIT 1, MEASUREMENT_RECORDED 44, CAPABILITY_PROBED 3, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    1.297  run_measurements -> SESSION_INIT (contract_version=0.0.0, harness_version=0.1.0, profile=w2m-vm-measurements, purpose=measurement, vocab_version=0.13)
  t=    1.297  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, measurement_id=qpc-exits-hvc-1, metric=vm_exits, phas
  t=    1.297  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, measurement_id=qpc-exit-ns-p50-hvc-1, metric=exit_ns_
  t=    1.297  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, measurement_id=qpc-exit-ns-p99-hvc-1, metric=exit_ns_
  t=    1.299  run_measurements -> CAPABILITY_PROBED (capability=feat_lse2, detail=hw.optional.arm.FEAT_LSE2 is 1 (sysctlbyname), ..., execution_id=None, host_id=mac-Mac17,7-macos-25F80, outcome=present)
  t=    1.299  run_measurements -> CAPABILITY_PROBED (capability=feat_bbm_level2, detail=FEAT_BBM level 0: ID_AA64MMFR2_EL1 0x1001001102..., execution_id=None, host_id=mac-Mac17,7-macos-25F80, outcome=absent)
  t=    1.299  run_measurements -> CAPABILITY_PROBED (capability=guest_wfe_exit, detail=the WFE probe's run of 1,000 WFE ended at its h..., execution_id=None, host_id=mac-Mac17,7-macos-25F80, outcome=absent)
  t=    1.299  run_measurements -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=1.108, outcome=completed)
  ... MEASUREMENT_RECORDED 41 more
```

### signal_trace_tags

- `CAPABILITY_PROBED`
- `MEASUREMENT_RECORDED`
- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - `run_measurements` on its record:
>   - SESSION_INIT;
>   - MEASUREMENT_RECORDED with `metric` `locked_model_steps`, `turn_start_ns_p50` and `turn_start_ns_p99`, each with
>     `vcpus` 2;
>   - CAPABILITY_PROBED with `capability` `feat_lse2`, `feat_bbm_level2` and `guest_wfe_exit`, each with its
>     `outcome` and `detail`;
>   - and, as before, its one-vCPU measurements and SESSION_COMPLETED.

## 3. Delta

### emits_check

- `CAPABILITY_PROBED`: fired 3 times
- `MEASUREMENT_RECORDED`: fired 44 times
- `SESSION_COMPLETED`: fired 1 time
- `SESSION_INIT`: fired 1 time

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 337: its Rubber Duck Pass and close.
