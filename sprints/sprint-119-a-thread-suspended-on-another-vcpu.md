# Sprint 119 — a thread suspended while another vCPU runs it

```yaml
---
id: 119
status: active
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
