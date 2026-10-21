# Sprint 121 — a parked vCPU wakes

```yaml
---
id: 121
status: done
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

## observed
- **Made:** 1953fbfe (the wake, the card, the guests readied and twoready, tests/test_parked_vcpu_wake.py); 204c3b42
  (the records); 4a0f341b (the several-process refusal's test, below).
- **Before** (on 2f16919b): the readied guest on two vCPUs ran the worker's 20 rounds on the waiting vCPU in 3 runs of
  13; in the other 10 vCPU 1 ran one round (8,000,032 or 8,000,042 instructions) and vCPU 0 switched between the two
  threads for the rest.
- **A first draft, replaced before the commit:** the guest claimed one waiting vCPU per readied thread and ended the
  slice itself. Removing the claim changed nothing in 10 runs: the host's resume after the wake request rewrites
  SLICE_UNTIL by its own rule (`_slice_until`, any ready thread ends the slice), so vCPU 0 raced the woken vCPU for
  the readied thread. The rule moved to the host, which now counts the waiting vCPUs; the claim went.
- **Judge:** readied, 10 runs of 10 before the commit: vCPU 1 at 160,000,241 instructions (the worker exactly), 0
  switches in the VM, 19 or 20 wake requests; twoready 10 of 10 ended, 38 requests; one vCPU 0 requests. The five
  tests pass. On 1953fbfe: the batch run (frontier-643a6283510c-1792611641237977000) conforms, 168 lines, 0
  divergences, 80,463,443,700 instructions, Sprint 120's seven exit kinds (hvc 8 at 39,198 against 42,315, the stop
  moving with wall-clock progress), `run_numbers.py` 104 numbers, 0 that do not add up; 456 of 458 classes (FCOS and
  FSIN as before), 68 of 68 families. On two vCPUs: family-threads-1792611776442439000 (instructions by vCPU [39,469,
  848,396] and [40,322, 868,669]), family-running_thread_context-1792611777227867000 ([6,365,141, 200,000]) and
  family-wait_handoff-1792611777811559000 ([9,978, 229], against Sprint 118's [10,063, 144]) conform. wait_handoff's
  waiters (probes/wait_handoff/wait_handoff.c) each run a few instructions between their waits, about 10,000 in the
  whole probe, so its second vCPU has little to run whichever vCPU wakes.
- **Plants**, each measured to fail: no wake request: readied spins to its 10 s limit (5 of 5 and 3 of 3; vCPU 0 at 37
  to 39 billion instructions); the old slice rule (`> 0`): readied switches in the VM 193 to 218 times and splits the
  worker (3 of 3); a waiting vCPU counted too many (`> waiting + 1`): twoready spins to its limit (3 of 3, before any
  wake request). A doubled count does not hang twoready (5 of 5): a vCPU that loads one worker while the other is
  ready gets a slice's end of its own.
- **A test that passed by luck:** the close's first gate run failed one test, and its name was lost (the command kept
  pytest's last line). Five runs of the five files that race vCPUs found it: the refusal with several processes,
  whose test ran spinchild at most 5 times until vCPU 0 reached the refused wait; on 1953fbfe it reaches that wait in
  11 runs of 20 (the other 9 end with "done"). 4a0f341b runs it up to 30 times; 10 test runs of 10 passed. Gate runs now
  keep pytest's whole output in build/.
- **Open, none deferred:** none from this sprint.
