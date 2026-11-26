# SIGNAL_REPORT — Sprint 359 — The runtime C's locks released by a scoped guard

Made by `signal_report 359` from 1 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits or the commit that holds this report added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 359.

## 1. Observed

### scope_confirmation

> The Decision "The runtime C's leaf locks released by a scoped guard" (2026-11-26, reviewer-4's twenty-fifth review,
> Low 1). The runtime C takes its locks in hand-written pairs, and each function's early returns repeat the release:
> - `hold`/`let_go` (heap.c);
> - `hold_slots`/`let_slots_go` (thread_storage.c, 14 releases at 304470288);
> - `hold_handles`/`let_handles_go` and `hold_sync`/`let_sync_go` (sync_objects.c, 49 releases between them);
> - since Sprint 355, `w2m_take_runtime_word`/`w2m_let_runtime_word_go` (runtime.c), which lets the word go only when
>   this call took it.

### work_performed

- 5ff435201 Sprint 359 card: the runtime C's locks released by a scoped guard

### signal_trace

```
w2m-vm-measurements-1795732027000000000: SESSION_INIT 1, MEASUREMENT_RECORDED 107, CAPABILITY_PROBED 3, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=   12.334  run_measurements -> SESSION_INIT (contract_version=0.0.0, harness_version=0.1.0, profile=w2m-vm-measurements, purpose=measurement, vocab_version=0.14)
  t=   12.334  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=qpc-exits-hvc-1, metric=vm_
  t=   12.335  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=qpc-exit-ns-p50-hvc-1, metr
  t=   12.335  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, lock=None, measurement_id=qpc-exit-ns-p99-hvc-1, metr
  t=   12.350  run_measurements -> CAPABILITY_PROBED (capability=feat_lse2, detail=hw.optional.arm.FEAT_LSE2 is 1 (sysctlbyname), ..., execution_id=None, host_id=mac-Mac17,7-macos-25F80, outcome=present)
  t=   12.350  run_measurements -> CAPABILITY_PROBED (capability=feat_bbm_level2, detail=FEAT_BBM level 0: ID_AA64MMFR2_EL1 0x1001001102..., execution_id=None, host_id=mac-Mac17,7-macos-25F80, outcome=absent)
  t=   12.350  run_measurements -> CAPABILITY_PROBED (capability=guest_wfe_exit, detail=the WFE probe's run of 1,000 WFE ended at its h..., execution_id=None, host_id=mac-Mac17,7-macos-25F80, outcome=absent)
  t=   12.350  run_measurements -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=11.909, outcome=completed)
  ... MEASUREMENT_RECORDED 104 more
```

### signal_trace_tags

- `CAPABILITY_PROBED`
- `MEASUREMENT_RECORDED`
- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - None: no emit site changes. The runtime's behavior and its records are unchanged, and Sprint 358's
>   `run_measurements` record, remade on this tree, carries the same metrics. That remade record is the observation's,
>   not a new emit.

## 3. Delta

### emits_check

- fired, not named in Emits: `CAPABILITY_PROBED` (3), `MEASUREMENT_RECORDED` (107), `SESSION_COMPLETED` (1), `SESSION_INIT` (1)
- the card's Emits names no tag, and the sprint's records fired the ones above

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 359: its Rubber Duck Pass and close.
