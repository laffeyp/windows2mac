# Sprint 095 — a release readies the waiter it can satisfy, not every waiter

```yaml
---
id: 095
status: active
phase: 5
pass_kind: functional
---
```

## scope

Roadmap step 2, first link (`docs/plan.md`: "the scheduler, waits and NtDelayExecution inside the VM, with the wake
path waking one waiter"). Today a release readies every thread blocked on its key, and all but the one the release can
satisfy run their retry only to block again. This sprint counts those re-blocks on the batch run, then changes the
wake sites the count names so a release readies only the waiters it satisfies. The frontier's stop is checked each
second rather than on the 30-second progress tick, so the time to the stop measures the run.

**Known first (from the code and Microsoft's pages, before any run).**

- Waits, sleeps and switches already run in the VM: `w2m_switch` (threads.c, Sprint 069), the deadline warp (Sprint
  070). Sprint 094's run: 997,529 switches in the VM, 2 hand-overs in the host. Host exits are file reads (op 9,
  42,948 of 50,372), not waits. Sleep is kernel32's face of NtDelayExecution (`slept`, threads.c).
- `w2m_signal(key)` (threads.c) readies every blocked thread whose wait key is `key` or 0. The callers:
  - `SetEvent`, `ReleaseSemaphore`, `ReleaseMutex` (sync_objects.c). These already hand a unit to the first waiter
    by `grant()` (Sprint 066's handoff fix), then ready every waiter on the object.
  - `LeaveCriticalSection` (critical_section.c): every waiter, and one acquires.
  - `ReleaseSRWLock*` and `WakeConditionVariable` / `WakeAll` (slim_locks.c). A wake bumps the variable's
    generation, so both wake every waiter.
- Windows (Microsoft Learn, read 2026-10-16): SetEvent, "the state of an auto-reset event object remains signaled
  until a single waiting thread is released" (page updated 2024-02-22); ReleaseSemaphore, "Each time a waiting thread
  is released because of the semaphore's signaled state, the count of the semaphore is decreased by one" (2024-02-22);
  WakeConditionVariable, "Wake a single thread waiting on the specified condition variable" (2025-08-27);
  SleepConditionVariableSRW, "Condition variables are subject to spurious wakeups (those not associated with an
  explicit wake) and stolen wakeups" (2025-10-31). Waking more is allowed for a condition variable; for the
  handed-over objects the extra wakes find nothing and block again.

## the measure

- Two counters in the monitor's data, read into the end report: `readied` (threads a signal made ready, at 0xB0) and
  `rewaits` (a retried wait or sleep that blocks again before its deadline, at 0xB8), and per call site the rewaits'
  keys' kinds, so the fix takes only the sites the count names.
- The batch run to its stop, the stop checked each second, two runs before the fix and two after, the load average
  at each start: seconds to the stop, instructions, switches, readied, rewaits.

## prerequisites

- 094 (the starting measure).

## context_files

- tools/guest_runtime/threads.c, sync_objects.c, critical_section.c, slim_locks.c, runtime.h; src/w2m/vm_backend.py
  (the run loop, the end report); scripts/run_frontier.py; sprints/sprint-069, -070.

## signal contract

### Emits

- None new. The frontier's records as before (SESSION_INIT, EXECUTION_STARTED, its stop, TRACE_INGESTED,
  COMPARISON_COMPLETED, SESSION_COMPLETED); the families' item runs (CANDIDATE_GENERATED, COMPARISON_COMPLETED,
  CANDIDATE_ACCEPTED).

### Invariants

- Vocabulary unchanged. A release still hands each unit to the waiter Windows would release (Sprint 066's rows).

## artifact contract

- The counters and the one-second stop; the wake sites the measure names; `scripts/gates.sh` exit 0; CI `success` on
  the closing commit. Files: threads.c, sync_objects.c (and critical_section.c or slim_locks.c if the count names
  them), vm_backend.py, run_frontier.py.

## observation contract

- **Judges:** the thread families (threads, wait_handoff, sync_objects, critical_sections, slim_locks,
  thread_handles, running_thread_context) accept on the new code; every family re-run where its candidate changed.
  The batch run conforms: 168 of 168 lines, 0 value divergences.
- **Plant:** a release that readies no waiter (the granted thread left blocked). Reach: wait_handoff's
  `event_auto.multiple_waiter` row and the threads rows whose worker waits on a released object; the family run must
  fail it (a lost wakeup stops the run as a deadlock or a timeout).
- **Bound:** rewaits on the batch run fall to at most 1% of their count before the fix at the sites changed.

## done criteria

Rewaits measured and cut at the sites the count names, the families and the batch run conforming, the seconds to the
stop reported before and after, CI green.

## observed

- **Census** (f0d68723; threads.c `w2m_wake_census`): before the fix, two runs to the stop readied 186,272 and 185,357
  threads by a signal, and 11,086 and 10,941 of them blocked again unsatisfied: 10,891 to 11,036 in `wait_one` (the
  handed-over objects), 26 in EnterCriticalSection, 23 to 24 in Sleep. Critical sections, SRW locks and condition
  variables were left as they were: the count named `wait_one` alone.
- **The stop each second** gives the run's own time: 39.2 and 38.1 s to the stop (81.4 and 81.3 billion instructions),
  where Sprint 094's runs read 61.4 s off the 30-second tick.
- **The fix** (734fce86, 6c9ce277): `grant()` readies the thread it hands the object to (`w2m_ready_thread`); a release
  readies no other waiter on the object; a manual-reset event still releases all. Two runs after: 58 re-blocks each,
  1 in `wait_one` (the bound: at most 1% of 10,891); switches 744,435 and 748,484 (from 761,001 to 763,511);
  instructions to the stop 80.8 and 81.2 billion; both conform (168 of 168 lines, 0 divergences).
- **A fixture found a defect** (KIT_DIARY #54). Two plants on `w2m_signal`'s key test came back blind once a release
  readied its waiter by grant. The case they stood for, a wait for all of several objects, had no row. The laptop's
  new row `wait_all.semaphore_and_event` (run lap-20261017T040446837Z-dc268176, two executions equal): the waiter takes
  both objects when the event is set, and the setter's zero-timeout waits find them taken (258). w2m let the setter
  take the event (0). A blocked wait-all now registers its objects, and each release takes them for the first
  wait-alls they complete and readies those threads. wait_handoff 0.3.0 (7 rows) accepts; its six plants fail as
  named, among them `granted_waiter_not_readied`, `wait_all_not_granted` and `wait_all_not_readied`. The plants on
  `w2m_signal`'s key-0 path were withdrawn: no row reaches it through a handed-over object, a blind spot (it still
  serves manual-reset events and other processes).
- **Families:** all 68 accept on 6c9ce277 (`runtime.h` changed, so every candidate).
- **Timing pending:** the after-runs ran at load averages 10.2 and 14.9 (twelve processes of another project at 100%
  of a core each), 74.8 and 110.0 s; not comparable. Repeated with Sprint 096's on a quiet Mac.

