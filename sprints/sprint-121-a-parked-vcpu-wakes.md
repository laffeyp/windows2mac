# Sprint 121 — a parked vCPU wakes

```yaml
---
id: 121
status: active
phase: 5
pass_kind: functional
---
```

## scope

Sprint 118's Deferred entry, made this sprint's scope by Sprint 120 under Decision 2026-10-11 user. A vCPU with no
thread waits in the host (`_wait_for_change`: `_park` past the first, `_switch` on vCPU 0), and a thread the guest
runtime readies inside the VM (threads.c `w2m_signal`, `w2m_ready_thread`) does not wake it: the vCPU that readied
the thread ends its own slice (SLICE_UNTIL 0) and switches to it, and the waiting vCPU learns of it only at the next
host serve. Measured before the change (scratch probe `readied`, built with mingw as tests/data/timed_waits are): a
worker waits on an auto-reset event 20 times and does 8,000,000 instructions of work after each, while the main
thread sets the event inside the VM and spins until the round is done. On two vCPUs the worker's rounds ran on vCPU 1
in 3 runs of 13; in the other 10 vCPU 1 ran one round (8,000,032 or 8,000,042 instructions) and vCPU 0 the rest,
switching between the two threads.

**Two designs.**

- **A: the guest asks the host.** The run table's spare word (0x28, `unused_slice_count` since Sprint 107) holds the
  count of vCPUs waiting in the host to take a thread, written by the host under the runtime lock as a vCPU starts and
  ends such a wait. `w2m_signal` and `w2m_ready_thread`, having readied a thread while that count is above 0, make one
  host request (W2M_THREAD_WAKE), whose answer notifies the waiters (`_notify_waiters`); with none waiting they end the
  running thread's slice as before. The host, resuming the readying vCPU, ends its slice only for ready threads beyond
  the waiting vCPUs (`_slice_until`): one decider, holding the state. A run of one vCPU never makes the request.
- **B: an SGI through the GIC.** A vCPU with no thread runs a guest idle loop of WFI, and `w2m_signal` writes
  ICC_SGI1R_EL1 for it. Hypervisor.framework returns from hv_vcpu_run on a guest WFI (QEMU's target/arm/hvf/hvf.c
  handles exit class EC_WFX_TRAP in `hvf_wfi` by halting the vCPU thread in the host until it has work), so the idle
  vCPU still waits in the host and still needs the host told: B is A's wake plus an interrupt vector table, a
  redistributor's setup and an idle loop in the guest.

**Choice: A.** It is the second of the two answers Sprint 118's entry named ("the guest's w2m_signal kicking a parked
vCPU's channel"), with a host request in place of a kick, since the vCPU to wake is not running. It is also the rule
the design of Sprint 120 states: a source of a wake from outside the host's serves notifies the waiters. Revisit if:
a measure shows the request's exits a share of a run's time, or a vCPU must wake from a timer interrupt.

Not in scope: threads readied by an expired deadline inside the VM (`wake_expired`); each waiting vCPU already waits
no later than the earliest deadline (Sprint 120) and readies it itself.

## prerequisites

- 120.

## context_files

- tools/guest_runtime/threads.c (`w2m_signal`, `w2m_ready_thread`, the run table); tools/guest_runtime/runtime.h
  (W2M_THREAD_*); src/w2m/vm_backend.py (`_wait_for_change`, `_thread_request`); src/w2m/scheduler.py (RT_*).

## signal contract

### Emits

- On the threads, running_thread_context and wait_handoff runs on two vCPUs (`family_run.py F --run --vcpus 2`):
  SESSION_INIT (purpose conformance), CANDIDATE_GENERATED, COMPARISON_COMPLETED for each reference run (verdict
  conforms, first_divergence none), CANDIDATE_ACCEPTED, SESSION_COMPLETED (outcome completed), baseline `vcpus` 2.
- On the batch run's record, as on Sprint 120's: SESSION_INIT, EXECUTION_STARTED, MEASUREMENT_RECORDED 104 times,
  EXECUTION_COMPLETED (exit_status killed), TRACE_INGESTED, COMPARISON_COMPLETED (first_divergence none),
  SESSION_COMPLETED. Per class run (458) and family run (68), as on Sprint 120's sweep: SESSION_INIT,
  CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED (456 and 68) or for FCOS and FSIN
  REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED, SESSION_COMPLETED.

### Invariants

- Vocabulary 0.9 unchanged.
- One vCPU makes no wake request: the batch run conforms (168 lines, 0 divergences) with Sprint 120's seven exit
  kinds and its 104 measurements.

## artifact contract

- The wake (threads.c, runtime.h, vm_backend.py, scheduler.py); the `readied` guest in tests/data/timed_waits and its
  tests; `scripts/gates.sh` exit 0; CI `success`.

## observation contract

- **Judge:** the `readied` guest on two vCPUs runs all 20 of the worker's rounds on the waiting vCPU (vCPU 1 at the
  worker's 160,000,241 instructions) with no switch in the VM, 5 runs of 5 in the test; the `twoready` guest (two
  workers readied for one waiting vCPU, each spinning until the other's round is done) ends, 5 runs of 5; the families
  as Emits says, with wait_handoff's instructions by vCPU reported beside Sprint 118's [10,063, 144].
- **Plants**, measured before they are trusted: the wake request removed from threads.c (the guest source, as the
  tests plant): `readied` spins to its limit; the host's old slice rule (any ready thread ends the slice): `readied`
  switches in the VM; one waiting vCPU counted too many: `twoready` spins to its limit.

## done criteria

A thread readied inside the VM while a vCPU waits with no thread runs on that vCPU; one vCPU unchanged; CI green.
