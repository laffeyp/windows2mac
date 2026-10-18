# Sprint 120 — every timed wait ends

```yaml
---
id: 120
status: active
phase: 5
pass_kind: functional
---
```

## scope

A defect class, not a step: a thread blocked with a timeout whose deadline passes and that nothing makes ready, and a
host thread that waits with nothing able to end its wait. Sprint 119 recorded one member ("vCPU 0 keeps no deadline
while it waits") as deferred; it is a hang, and the class has the members below, all fixed here (the last three found
while fixing the first ones).

- **The wall clock wakes no timed-out wait while another thread is ready, on one vCPU too.** A probe that sleeps 20 ms
  while a second thread spins (scratch, built with mingw) stops at its 20 s limit on one vCPU and on two on the wall
  clock, and ends at once on the instruction clock (measured). Only the instruction clock made expired waits ready
  (threads.c `wake_expired`, vm_backend.py `_wake_expired`); on the wall clock a timed thread ran only when no thread
  was ready. Both now read the clock of the run's mode: the guest's counter in the VM, and on the host `self.clock()`,
  which is the guest's counter (time.monotonic is mach_absolute_time on this Python, `time.get_clock_info`; the
  guest's QueryPerformanceCounter read 21462022552309 between host reads 21462021399109 and 21462022984827, measured).
- **A host thread that waits keeps the deadlines.** vCPU 0 waiting while another vCPU runs a thread (`_switch`), a
  vCPU past the first waiting for a thread (`_park`), and a SuspendThread waiting for another vCPU
  (`_stop_elsewhere`) wait at most until the earliest deadline of a blocked thread, then make the expired ones ready.
- **Two threads suspending each other from two vCPUs.** Each host waited for the other's thread: a hang. A suspend
  changes what a waiter waits on, so it notifies; a SuspendThread waiting for another vCPU whose own thread is
  suspended meanwhile returns (Windows' SuspendThread may return before the thread stops), and threads.c puts the
  caller down at once (SCHEDULE_YIELD), so it runs no instruction after its suspension.
- **GetThreadContext of a suspended thread another vCPU still holds** read its stale saved state. It now waits as
  SuspendThread does; if its own thread is suspended meanwhile, the call is retried (RETRY: the thread is put down at
  the call, and calls it again when resumed), so no waiter holds a suspended thread. GetThreadContext of a running,
  unsuspended thread on another vCPU is not in the rows: Unimplemented.
- **The instruction clock and the host's clock on several vCPUs** are one vCPU's count or the host's step clock, not
  one clock for the process: a run that asks for either with `thread_vcpus` above 1 stops with Unimplemented.
- **A run whose every host thread waits hung past its limit.** The VM's limit bounds one run of the VM, so a wait in
  the host was never cut short (a planted mutual suspend ran 88 s against a 20 s limit, measured). When every vCPU
  waits a whole second with no deadline pending and no notify, nothing can wake a thread: the run stops as one vCPU's
  does, "every thread waits with no timeout and nothing can wake one (a deadlock)".
- **vCPU 0, waiting for another vCPU, drives no other process** (it drives the kernel.Machine). With threads on several
  vCPUs in a machine of several processes, that wait stops with Unimplemented instead of starving the others.
- **A child process never ran while its parent of one thread spun.** The parent kept its whole run's budget (2^40)
  after CreateProcess, so it never reached a slice's end, where the Machine gives another process a turn: in 8 s,
  1,115,464 host requests and no slice's end (traced). A second process now starts time slices, as a second thread
  does; and a process whose blocked thread's deadline has passed counts as ready to the Machine (`ready`).
- **On the instruction clock a process's clock counts only its own instructions,** so a child's sleep stood still
  while its parent ran. A process that runs again after another's turn has its clock moved to the latest of the
  machine's (`_catch_up`, the warp), and a deadline passes on that clock (`_machine_now`).

## prerequisites

- 119.

## context_files

- src/w2m/vm_backend.py (`_wake_expired`, `_wait_for_change`, `_switch`, `_park`, `_stop_elsewhere`,
  `_thread_request`); tools/guest_runtime/threads.c (`wake_expired`, `k32_SuspendThread`, `k32_GetThreadContext`).

## signal contract

### Emits

- On the threads, running_thread_context and wait_handoff runs on two vCPUs (`family_run.py F --run --vcpus 2`), as on
  Sprint 119's: SESSION_INIT (purpose conformance), CANDIDATE_GENERATED, COMPARISON_COMPLETED for each reference run
  (verdict conforms, first_divergence none), CANDIDATE_ACCEPTED, SESSION_COMPLETED (outcome completed), baseline
  `vcpus` 2.
- On the batch run's record, as on Sprint 119's: SESSION_INIT, EXECUTION_STARTED, MEASUREMENT_RECORDED 107 times,
  EXECUTION_COMPLETED (exit_status killed), TRACE_INGESTED, COMPARISON_COMPLETED (first_divergence none),
  SESSION_COMPLETED. Per class run (458) and family run (68), as on Sprint 119's sweep.

### Invariants

- Vocabulary 0.9 unchanged.
- The instruction clock's runs unchanged: the batch run's 168 lines and instruction count.

## artifact contract

- The fixes above; tests in `tests/test_timed_waits.py` with the sleeping probe; `scripts/gates.sh` exit 0; CI
  `success`.

## observation contract

- **Judge:** the sleeping probe ends (its two lines, ExitProcess) on one and two vCPUs on the wall clock; two threads
  suspending each other from two vCPUs both end suspended and the run goes on; the families as Emits says; the batch
  run unchanged; 456 of 458 classes, 68 of 68 families.
- **Plants**, measured before they are trusted: the wall clock's expiry removed (the sleeping probe stops at its
  limit); the waits untimed (the sleeping probe on two vCPUs stops at its limit, measured to reach that wait); the
  mutual suspend's early return removed (the mutual probe stops at its limit).

## done criteria

No thread with a timeout waits past it for want of a wake; no two hosts wait on each other; CI green.
