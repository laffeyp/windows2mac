# SIGNAL_REPORT — Sprint 356 — The sync objects' lock: waits and releases leave the runtime word

Made by `signal_report 356` from 1 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits or the commit that holds this report added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 356.

## 1. Observed

### scope_confirmation

> Card 5 of `docs/design-handles-and-sync-objects-2026-11-26.md`, the last of its five: the sync objects' own lock, so a
> wait or a release holds the runtime word only to park, to ready or to make a host request. The design is the spec
> ("Which lock guards which state", "The order, and the two leaves", "The wait across two locks", "Where each service's
> class lands", "The judges each card needs", "The cards that follow" 361-364). The parts:
> - **The lock.** `sync_lock` in `tools/guest_runtime/sync_objects.c`, in the form of `hold_handles` and
>   `let_handles_go`. It guards the mutable state of semaphores, events and mutexes (item 1's `count` and `max` for those
>   kinds) and items 5 to 8: `waiters[]`, `nwaiters`, `all_waiters[]` and `nall`.
> - **The services.** In `crates/guest_runtime/src/services.rs`, `Lock::SyncObjects` and the list `SYNC_LOCK`: SetEvent,
>   ResetEvent, ReleaseSemaphore, ReleaseMutex, WaitForSingleObject, WaitForSingleObjectEx, WaitForMultipleObjects,
>   WaitForMultipleObjectsEx. The class test follows Sprint 355's.
> - **The word in the C.** These services take the runtime word themselves, with no leaf held, around:
>   - parking (`w2m_block`);
>   - readying (`w2m_ready_thread`, `w2m_signal`);
>   - the host-backed kinds' requests (THREAD, PROCESS, SHARED_EVENT; the design, 294-299).
>   Use `w2m_take_runtime_word` (Sprint 355). Say how the RETRY path (`W2M_SCHEDULE_RETRY`, runtime.h 136-142;
>   `guest.rs` 624-627) behaves when the bridge holds no word, and judge it.
> - **The class first.** Before any service moves, list every reader and writer of the state `sync_lock` guards, and
>   every place a wait or a release reaches state the word keeps (items 10, 11, 13 and 14). Give one line each, with its
>   file:line and the lock it holds today. Each must take the lock its state needs, or the list says why it need not.
>   The list includes at least:
>   - `grant`, `grant_all`, `take`, `take_for`, `has_grant`, `waiter_add`, `waiter_drop`, `waiters_drop`, `all_drop`,
>     `timed_out`, `wait_one`, and WaitForMultipleObjects' any and all paths;
>   - CloseHandle's path through these objects, and the services that stay on the word but read these kinds' state;
>   - the TEB's `wait_deadline` and `wait_key` (items 10 and 11), written by a waiter and read by `w2m_signal` and the
>     switch on another vCPU.
>   Report each further one to the Supervisor at once, with its evidence.
> - **The leaf rule holds.** `sync_lock` and `handles_lock` are never held together. Neither is held across the word,
>   a host request, `w2m_unimplemented` or `w2m_logged`. References are dropped under the table's lock after `sync_lock`
>   is let go (the design, 241-246).
> - **What card 5 owes from earlier cards:**
>   - **Sprint 354, Deferred (a).** Arm the woken mark at a wait's check, under `sync_lock`, so only a thread that has
>     checked and not yet parked can take it. A thread running guest code elsewhere then takes none, and the spare
>     retry ends. Remove the `held_elsewhere` marking only if the arming covers every path in Sprint 354's class list,
>     and say so line by line.
>   - **Sprint 354, Deferred (b).** The C switch's clearing of the mark at load gets a judge of its own.
>   - **Sprint 353's Decision, "How a readying in the VM on one vCPU ends another vCPU's slice".** This card changes the
>     switch path, so measure `two_counters` by the command in the Decision "How the slicing Decision's 7.6% is measured
>     again", before and after, and quote both.
> - **Under one vCPU nothing observable changes.** The batch run, the family runs (with `wait_handoff`), the exit ledger,
>   `wait_closed_handle`, `wait_closed_kinds`, `three_counters` and `handle_values` must pass unchanged.
> - **The judges.**
>   - **The semaphore's units across vCPUs**, the design's guest, built as `tests/data/timed_waits`' guests are: four
>     threads release and wait on one semaphore (maximum 1,000) 100,000 times. The final count equals the releases less
>     the satisfied waits, and every wait returns. Run it 20 times on two vCPUs.
>   - **The lost wakeup**, `tests/data/timed_waits/lost_wakeup`, kept by Sprint 354 for this card: 20 runs on two vCPUs,
>     each inside its limit.
>   - **Each plant must make its judge fail on two vCPUs**, and the result says in how many of 20 runs. A judge that does
>     not catch its plant is strengthened until it does (Sprint 355's lesson), and the result says how.
> - **Not here:** a lock per object (the design's Revisit if); the critical sections and the slim locks.

### work_performed

- 9a0f0e937 Sprint 356 card amendment: the probe mutex_abandoned, for waits on a mutex whose owner thread ended holding it
- 39ec05978 Sprint 356 card: the sync objects' lock, waits and releases off the runtime word; the woken mark armed at a wait's check

### signal_trace

```
w2m-vm-measurements-1795720575000000000: SESSION_INIT 1, MEASUREMENT_RECORDED 56, CAPABILITY_PROBED 3, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    2.006  run_measurements -> SESSION_INIT (contract_version=0.0.0, harness_version=0.1.0, profile=w2m-vm-measurements, purpose=measurement, vocab_version=0.13)
  t=    2.006  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, measurement_id=qpc-exits-hvc-1, metric=vm_exits, phas
  t=    2.006  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, measurement_id=qpc-exit-ns-p50-hvc-1, metric=exit_ns_
  t=    2.006  run_measurements -> MEASUREMENT_RECORDED (application_profile_id=timed-waits-qpc, artifact_state=cold, backend=managed_vm, destination_profile_id=w2m-vm, holder=None, measurement_id=qpc-exit-ns-p99-hvc-1, metric=exit_ns_
  t=    2.008  run_measurements -> CAPABILITY_PROBED (capability=feat_lse2, detail=hw.optional.arm.FEAT_LSE2 is 1 (sysctlbyname), ..., execution_id=None, host_id=mac-Mac17,7-macos-25F80, outcome=present)
  t=    2.008  run_measurements -> CAPABILITY_PROBED (capability=feat_bbm_level2, detail=FEAT_BBM level 0: ID_AA64MMFR2_EL1 0x1001001102..., execution_id=None, host_id=mac-Mac17,7-macos-25F80, outcome=absent)
  t=    2.008  run_measurements -> CAPABILITY_PROBED (capability=guest_wfe_exit, detail=the WFE probe's run of 1,000 WFE ended at its h..., execution_id=None, host_id=mac-Mac17,7-macos-25F80, outcome=absent)
  t=    2.008  run_measurements -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=1.802, outcome=completed)
  ... MEASUREMENT_RECORDED 53 more
```

### signal_trace_tags

- `CAPABILITY_PROBED`
- `MEASUREMENT_RECORDED`
- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - None; which lock the waits and releases take.

## 3. Delta

### emits_check

- fired, not named in Emits: `CAPABILITY_PROBED` (3), `MEASUREMENT_RECORDED` (56), `SESSION_COMPLETED` (1), `SESSION_INIT` (1)
- the card's Emits names no tag, and the sprint's records fired the ones above

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 356: its Rubber Duck Pass and close.
