# Sprint 120 — every timed wait ends

```yaml
---
id: 120
status: done
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
  machine's (`_catch_up`, the warp), at its start and on each resume (the start added after the review of 2026-10-18:
  a child's first reading came before its parent's), and a deadline passes on that clock (`_machine_now`).
- **A signal from another process** (`VM.wake_all`, which kernel.py calls) now notifies the waiting vCPUs of the process it wakes.

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
- On the batch run's record, as on Sprint 119's: SESSION_INIT, EXECUTION_STARTED, MEASUREMENT_RECORDED 104 times (Sprint 119's 107 less hvc 12's exits and seconds and the
  "a thread ready" hand-overs, which no longer occur),
  EXECUTION_COMPLETED (exit_status killed), TRACE_INGESTED, COMPARISON_COMPLETED (first_divergence none),
  SESSION_COMPLETED. Per class run (458) and family run (68), as on Sprint 119's sweep: SESSION_INIT,
  CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED (456 and 68) or for FCOS and FSIN REDUCTION_COMPLETED,
  DISCREPANCY_REPORTED and DISCREPANCY_ROUTED, SESSION_COMPLETED. (Amended at the close: the first draft named the
  sweep's tags by reference, and the Emits check counted the last three as unnamed.)

### Invariants

- Vocabulary 0.9 unchanged.
- The batch run conforms: 168 lines, 0 divergences. Its instruction count at the stop is no invariant (the stop is tested
  on wall-clock progress lines; 80.7 to 83.0 billion over Sprints 114-119). Its calls are expected to move: the crash
  handler, a second process, now gets turns and a clock that keeps up (the review of 2026-10-18 isolated the two
  changes that move them).

## artifact contract

- The fixes above; tests in `tests/test_timed_waits.py` with the sleeping probe; `scripts/gates.sh` exit 0; CI
  `success`.

## observation contract

- **Judge:** the guests in tests/data/timed_waits (the card was written after the first measurement, the sleeping probe on
  both clocks, and before the first code change, and committed with the code in 0da7f85e); the sleeping probe ends (its two lines, ExitProcess) on one and two vCPUs on the wall clock; two threads
  suspending each other from two vCPUs both end suspended and the run goes on; the families as Emits says; the batch
  run unchanged; 456 of 458 classes, 68 of 68 families.
- **Plants**, measured before they are trusted: the wall clock's expiry removed (the sleeping probe stops at its
  limit); the waits untimed (the sleeping probe on two vCPUs stops at its limit, measured to reach that wait); the
  mutual suspend's early return removed (the mutual probe stops at its limit).

## done criteria

No thread with a timeout waits past it for want of a wake; no two hosts wait on each other; CI green.

## observed
- **Made:** 0da7f85e (the fixes, the card, the guests and tests); 25e57995 (after the review of 2026-10-18: a child takes
  the machine's clock at its start, `wake_all` notifies, tests for the deadlock stop, the refusal with several
  processes, the child's sleep on the instruction clock and the counter on vCPU 1); 8838314f and ef33fe1d (two race
  plants' tests, the no-wait plant's and the lock plant's, run until their race shows).
- **Before** (each measured on the tree before its fix): a 20 ms Sleep beside a spinner stopped at the 20 s limit on one
  vCPU and on two on the wall clock; a mutual suspend on two vCPUs hung past its 20 s limit (88 s, killed); a child's
  sleep while its parent spun hung on one vCPU on both clocks (31 s and 28 s past a 15 s limit, killed); the trace:
  1,115,464 host requests and no slice's end in 8 s.
- **Judge:** the eight guests' tests pass (23 tests with the two-vCPU file, three runs in a row 23 of 23). On 8838314f:
  the batch run (frontier-643a6283510c-1792347897080595000) conforms, 168 lines, 0 divergences, 81,944,304,896
  instructions, seven exit kinds: hvc 12 (2 in every run of Sprints 110-119) gone and hvc 4 at 10 (8), from the second
  process's slices (a run with the clock changes planted out kept them so; the review isolated the call-stream change to
  the slices and the catch-up together); `run_numbers.py` 104 numbers, 0 that do not add up. 456 of 458 classes (FCOS
  and FSIN as before), 68 of 68 families. On two vCPUs: family-threads-1792348019048745000 (one crossing suspend in
  each run), family-running_thread_context-1792348019878009000 (two), family-wait_handoff-1792348020413763000 conform.
- **Plants**, each measured to fail: expiry on the instruction clock only (the sleep never ends: the run's limit);
  untimed host waits (the limit, once reached: up to 30 runs, a missed run 0.1 s); Sprint 119's waits on a mutual
  suspend (the deadlock stop, 1.1 s); a whole run's budget (the child never runs); no catch-up at a child's start
  ("clock behind"); the context retry reached with SuspendThread not waiting (3 of 6 runs), every run reading its 20
  contexts.
- **Records moved to build/:** 0da7f85e's sweep (superseded by the follow-up), 25e57995's sweep (BLACKBOARD.md was
  uncommitted; the batch run refused), and a batch run with the clock changes planted out.
- **Open, none deferred:** the parked vCPU's wake is Sprint 121 (BLACKBOARD 2026-10-18); the Machine driven from
  somewhere other than vCPU 0's wait is a prerequisite of step 7 running the game (docs/design-several-vcpus.md).
