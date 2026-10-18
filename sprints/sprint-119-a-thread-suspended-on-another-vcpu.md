# Sprint 119 — a thread suspended while another vCPU runs it

```yaml
---
id: 119
status: done
phase: 5
pass_kind: functional
---
```

## scope

Step 1, the chain's sixth and last sprint (docs/design-several-vcpus.md, "The host for several vCPUs, designed
twice"). One concept: SuspendThread and GetThreadContext on a thread another vCPU is running.

- **SuspendThread stops the thread before it returns.** When another vCPU holds the thread (its mark, Sprint 118), the
  suspending vCPU's host kicks that vCPU (Sprint 117) and waits, giving the runtime lock back, until the mark clears
  (vm_backend.py `_stop_elsewhere`). Windows' SuspendThread may return before the thread stops, and GetThreadContext
  completes the stop; returning after it is one of the outcomes Windows allows.
- **A suspended thread leaves its vCPU through the host.** The kicked vCPU's next run has SLICE_UNTIL 0 for a suspended
  current thread (`_slice_until`), so the slice ends within one TIME_SLICE; threads.c `w2m_switch` declines a suspended
  current thread, so the slice's end reaches the host, whose switch saves the thread at an instruction boundary,
  clears the mark and notifies the waiting suspender.
- **vCPU 0 puts its thread down before it waits.** `_switch` on vCPU 0 with no thread to run while another vCPU runs
  one saves the current thread before it waits (once), so a suspender on another vCPU sees it stopped.
- **A thread's vectors are in its slot when it is put down.** On several vCPUs `_save_thread` sends the vector swap at
  once (`Vcpu.flush_swap`): the swap waited for the vCPU's next run, so another vCPU loading the thread, or a
  GetThreadContext, could read the slot before it was written (a Sprint 118 gap).
- **The run names it.** `VM.remote_suspends` counts the suspends of a thread another vCPU held; `family_run.py
  --vcpus N` prints it beside the instructions by vCPU.

Out of scope: two threads on two vCPUs suspending each other at once (each host waits for the other's thread: a hang,
where Windows returns from both and the program hangs); a machine of several processes with several vCPUs; the parked
vCPU's wake (SGI).

## prerequisites

- 118.

## context_files

- src/w2m/vm_backend.py (`_thread_request` THREAD_SUSPEND and THREAD_CONTEXT, `_switch`, `_save_thread`,
  `_slice_until`, `_wait_for_change`, `Vcpu.kick`, `Vcpu.flush_swap`); tools/guest_runtime/threads.c (`w2m_switch`,
  `k32_SuspendThread`, `k32_GetThreadContext`); probes/threads/threads.c (suspend, suspend.frozen, context);
  probes/running_thread_context/running_thread_context.c; scripts/family_run.py (`run_in_vm`).

## signal contract

### Emits

- On the threads run on two vCPUs (`family_run.py threads --run --vcpus 2`), its record's baseline naming `vcpus` 2:
  SESSION_INIT (purpose conformance), CANDIDATE_GENERATED, COMPARISON_COMPLETED for each of its two runs (verdict
  conforms, first_divergence none), CANDIDATE_ACCEPTED, SESSION_COMPLETED (outcome completed). The same for
  running_thread_context on two vCPUs (one run).
- On the batch run's record, as on Sprint 118's: SESSION_INIT, EXECUTION_STARTED, MEASUREMENT_RECORDED 107 times,
  EXECUTION_COMPLETED (exit_status killed), TRACE_INGESTED, COMPARISON_COMPLETED (first_divergence none),
  SESSION_COMPLETED. Per class run (458) and family run (68), as on Sprint 118's sweep: SESSION_INIT,
  CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED (456 and 68) or for FCOS and FSIN REDUCTION_COMPLETED,
  DISCREPANCY_REPORTED and DISCREPANCY_ROUTED, SESSION_COMPLETED.

### Invariants

- Vocabulary 0.9 unchanged.
- One vCPU (`thread_vcpus` 1) behaves as before: no thread is ever held by another vCPU there, so `_stop_elsewhere`
  returns at once and `_switch` never waits.

## artifact contract

- `_stop_elsewhere`, the decline in threads.c `w2m_switch`, the save before vCPU 0 waits, the flush in
  `_save_thread`, `remote_suspends`; `scripts/gates.sh` exit 0; CI `success`.

## observation contract

- **Judge:** `family_run.py threads --run --vcpus 2` conforms on both reference runs over repeated runs (measured
  before the commit), with suspends of a thread another vCPU ran counted (printed by the run), and its record's
  baseline `vcpus` 2; `family_run.py running_thread_context --run --vcpus 2` conforms the same way. Before the
  change, threads on two vCPUs lost every row from suspend.frozen on in one run of two (measured, record
  family-threads-1792311298733780000). The batch run conforms (168 lines, 0 divergences, the same eight exit kinds);
  456 of 458 classes, 68 of 68 families; the VM tests.
- **Plants**, each measured over repeated runs before it is trusted: SuspendThread not waiting for the other vCPU
  (`_stop_elsewhere` returning at once): threads on two vCPUs does not conform or stops; the flush in `_save_thread`
  removed: running_thread_context on two vCPUs does not conform (its XMM rows) or the plant is recorded as not caught.

## done criteria

A thread another vCPU runs is suspended and read with the rows it has on one vCPU; one vCPU unchanged; CI green.

## observed
- **Made** (983af64b): `_stop_elsewhere` (kick, then wait with the lock given back until the mark clears); the
  decline in threads.c `w2m_switch`; `_slice_until` 0 for a suspended current thread; vCPU 0's save before it waits,
  carried through the Machine's idle turn (`saved`); `_vectors_to_slot` on a save with the lock shared;
  `remote_suspends`, printed by `family_run.py --vcpus`; tests in `tests/test_threads_on_vcpus.py`.
- **Before:** threads on two vCPUs lost every row from suspend.frozen on in 1 run of 2, vCPU 0 at 215 billion
  instructions (record family-threads-1792311298733780000, from the uncommitted tree, moved to
  build/sprint-119-uncommitted-records/ with six others).
- **Found on the way, three defects, all fixed here:**
  - *A save with no notify.* The first run hung with both host threads waiting on the VM's condition: vCPU 0 put its
    thread down and went to wait without telling the suspender, which waited for that mark. vCPU 0 now notifies.
  - *A put-down thread run from stale registers.* In 4 of 8 runs (the in-VM switch off; 2 of 3 with it on)
    events.created_suspended listed the worker's and spinner's events again: main's `seen` went back to its value
    before events.worker_started. A trace of every save and load showed vCPU 0 put main down, vCPU 1 ran main on,
    and vCPU 0's `_switch`, back from the Machine's idle turn, resumed main from the exit's registers, unmarked, so
    vCPU 1 loaded it as well. `saved` now rides through the idle turn, and the picks by deadline (the timed lists,
    `warp_to_deadline`, `_resume_idle`) skip a thread another vCPU holds. 0 of 10 runs wrong since.
  - *A slot read before its save.* Without the flush, 13 of 18 context reads in six running_thread_context runs read
    a slot a vCPU still owed its save; with it, 0 of 18.
- **Judge:** threads on two vCPUs conformed 20 of 20 runs, the suspend crossing vCPUs in 12; running_thread_context
  conformed 12 of 12, crossing in 9 (measured before the commit). On 983af64b: family-threads-1792313149099557000
  conforms on both reference runs, CANDIDATE_ACCEPTED, baseline `vcpus` 2, one crossing suspend in each;
  family-running_thread_context-1792313150102405000 conforms, CANDIDATE_ACCEPTED, two crossing suspends.
- **Plants:** SuspendThread not waiting: 6 of 10 runs reached the other vCPU, and all 6 stopped at the family's 60 s
  limit with 12 of 32 rows (vCPU 0, waiting, keeps no deadline, and the spinner never stops); the 4 that did not
  reach it conformed. The flush removed: the rows conformed 10 of 10 (the spinner's vectors match any save made after
  it planted them), so the test judges it by its witness, the pending-slot reads.
- **Unchanged on one vCPU:** the batch run (frontier-643a6283510c-1792313027375176000) conforms: 168 lines, 0
  divergences, 83,034,845,069 instructions; `check_records.py` 0 violations; `run_numbers.py` 107 numbers, 0 that do
  not add up; the same eight exit kinds. 456 of 458 classes (FCOS and FSIN as before), 68 of 68 families. Gates before
  the commit: 2,511 passed, 3 failed: the threads family's plant `suspend_answers_new_count` named the changed lines
  (fixed before the commit), and the two committed indexes the sweep then rewrote.
- **Open:** vCPU 0, waiting for another vCPU, keeps no deadline, and a vCPU whose run began with SLICE_UNTIL ~0 does
  not learn of a deadline made elsewhere (the plant showed it; the real path does not reach it here). Two threads
  suspending each other at once from two vCPUs would leave both hosts waiting.
