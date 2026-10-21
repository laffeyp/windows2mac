# Sprint 122 — processes beside several vCPUs

```yaml
---
id: 122
status: in progress
phase: 5
pass_kind: functional
---
```

## scope

Step 7 of the several-vCPU chain needs the Machine driven from somewhere other than vCPU 0's wait (Sprint 120's
open item, docs/design-several-vcpus.md). Today one Python thread drives a kernel.Machine: it runs each process's
`VM.steps` in turn, and serves the main process's vCPU 0 inside it. When vCPU 0 has no thread while another vCPU runs
one, it waits in `_switch` for that vCPU, and while it waits no other process runs, so w2m refuses the wait once a
second process exists (Unimplemented: "a thread waiting on vCPU 0 for another vCPU while another process could run").
The game always starts UnityCrashHandler64.exe, so a batch run with its threads on several vCPUs stops there. Measured
on 1953fbfe: tests/data/timed_waits spinchild reaches the refusal in 11 runs of 20.

Three faults stand behind the refusal, each a consequence of one driver thread no longer being the only thread that
touches the Machine:

1. **The wait.** vCPU 0 waiting in `_switch` holds the driver thread.
2. **The shared objects.** `Machine.request` (events, pipes, processes) runs on whichever host thread serves the
   request; a second vCPU's host thread and the driver can run it at once, and nothing locks the Machine's tables.
3. **A wake across processes.** `Machine.wake` sets every blocked thread of every process ready, writing their run
   table entries (and RT_BLOCKED's count) from the requesting process's thread, while the woken process's vCPUs run
   its threads. A thread that read a shared event as unset (EVENT_STATE) and has not yet blocked (threads.c
   `w2m_block`, both inside the runtime's C under the runtime lock) misses a wake made in between and waits forever.

**Two designs.**

- **A: vCPU 0 hands the Machine on.** With its threads on several vCPUs, a process's vCPU 0 never waits in `_switch`
  inside a Machine: with no thread to take it puts its thread down, counts itself in RT_WAITING (it takes a thread
  when it comes back, Sprint 121) and answers IDLE, so `steps` yields "idle" and the Machine runs another process. A
  process is ready for the Machine only when a thread no other vCPU holds is ready. With no process ready, the Machine
  waits on its own condition, notified by every serve of a host thread, until a change, the earliest deadline or its
  stop check; it stops the run as a deadlock after a whole check with no change, no deadline and no host thread
  running a vCPU. A host thread that makes its process ready while the driver runs another process kicks that
  process's vCPU 0 (hv_vcpus_exit), whose next slice end reaches the host and yields. The Machine's tables are
  under one lock. A wake into a process another thread may be running is left as a pending wake, which that process
  applies under its runtime lock at its next exit from outside the runtime's C; until then its slices end at the
  host. A machine whose processes each run on one vCPU keeps today's path exactly: one driver, direct wakes.
- **B: a driver thread per process.** Each process's `VM.steps` runs on a thread of its own; the Machine is a store of
  shared objects under a lock, and nothing is handed on. B needs A's pending wake and lock all the same, and it
  removes the round robin that the instruction clock's runs depend on (one vCPU: the batch run, every family), so it
  would be a second driver beside the first.

**Choice: A.** One driver for every run; the change is confined to runs whose threads are on several vCPUs, which
already refuse the instruction clock. Revisit if: a measure shows the driver thread's hand-overs a share of a run's
time, or a process must run on several vCPUs as a child.

Not in scope: a child process with its threads on several vCPUs (the spawner gives each child one); the speed of a
run on several vCPUs beside a second process (RT_IN_VM, the in-VM switch, is one word for all of a process's vCPUs,
so another process's being due sends every vCPU's switch to the host); the batch run's conformance on several vCPUs,
which this sprint measures and reports (wall clock, `run_frontier.py --vcpus 2 --clock-rate 0`).

## prerequisites

- 121.

## context_files

- src/w2m/kernel.py (Machine.run, `_next`, `request`, `wake`); src/w2m/vm_backend.py (`_switch`, `_resume_idle`,
  `ready`, `wake_all`, `_notify_waiters`, `steps`, `_serve_vcpu`, `_slice_until`, `_before_run`, Channel.kick);
  tests/test_timed_waits.py (spinchild).

## signal contract

### Emits

- On the threads, running_thread_context and wait_handoff runs on two vCPUs (`family_run.py F --run --vcpus 2`):
  SESSION_INIT (purpose conformance), CANDIDATE_GENERATED, COMPARISON_COMPLETED for each reference run (verdict
  conforms, first_divergence none), CANDIDATE_ACCEPTED, SESSION_COMPLETED (outcome completed), baseline `vcpus` 2.
- On the batch run's record, as on Sprint 121's: SESSION_INIT, EXECUTION_STARTED, MEASUREMENT_RECORDED 104 times,
  EXECUTION_COMPLETED (exit_status killed), TRACE_INGESTED, COMPARISON_COMPLETED (first_divergence none),
  SESSION_COMPLETED. Per class run (458) and family run (68), as on Sprint 121's sweep: SESSION_INIT,
  CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED (456 and 68) or for FCOS and FSIN
  REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED, SESSION_COMPLETED.
- On the batch run on two vCPUs (scratch, `--scratch`): SESSION_INIT, EXECUTION_STARTED, MEASUREMENT_RECORDED,
  EXECUTION_COMPLETED, and TRACE_INGESTED and COMPARISON_COMPLETED when the run reaches the log; its verdict reported
  whatever it is.

### Invariants

- Vocabulary 0.9 unchanged.
- One vCPU per process keeps today's driver and direct wakes: the batch run conforms (168 lines, 0 divergences) with
  Sprint 121's seven exit kinds and its 104 measurements.

## artifact contract

- The hand-on, the Machine's lock and wait, the pending wake, the kick (kernel.py, vm_backend.py); the guests handshake
  and busychild in tests/data/timed_waits and their tests; spinchild's test judging an end in place of the refusal;
  `run_frontier.py --vcpus`; `scripts/gates.sh` exit 0; CI `success`.

## observation contract

- **Judge:** on two vCPUs, spinchild, handshake and busychild each end with "done", 10 runs of 10 before the commit
  and 5 of 5 in the tests; on one vCPU each ends as before; the families as Emits says.
- **Plants**, measured before they are trusted: vCPU 0 waits in `_switch` as before, the refusal removed: handshake
  spins to its limit (the child that sets the event never runs); no kick: busychild spins to its limit (the child
  keeps the driver); the wake written at once into a process another thread runs: reported as measured, with the
  guest and run count that tried to reach it.

## done criteria

A run with its threads on several vCPUs runs other processes while vCPU 0 has no thread; no refusal on that path;
one vCPU unchanged; CI green.
