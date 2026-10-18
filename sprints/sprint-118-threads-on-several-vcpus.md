# Sprint 118 — guest threads on several vCPUs

```yaml
---
id: 118
status: active
phase: 5
pass_kind: functional
---
```

## scope

Step 1, the chain's fifth sprint (docs/design-several-vcpus.md, "The host for several vCPUs, designed twice"). One
concept: one process's guest threads run on several vCPUs at once, each vCPU served by a host thread of its own.

- **A thread runs on one vCPU at a time (the RUNNING mark).** A run entry's free word (offset 0x138, between `x` and
  the monitor words; scheduler.py `E_VCPU`, runtime.h `struct w2m_run_entry.vcpu`) holds the index + 1 of the vCPU
  that has the thread loaded, 0 for none. The vCPU that loads a thread sets it (threads.c `w2m_switch`, vm_backend.py
  `_load_thread`) and the one that saves it clears it; `w2m_switch`, `slice_until`, `_switch` and `_slice_until` skip a
  thread another vCPU holds. A running thread's status stays READY, as now.
- **Counts per vCPU.** vCPUs 1.. start with count, PAUSE count and slice counts 0 (their pages are copies of vCPU 0's);
  `instructions_executed`, `pauses_executed` and `thread_census` sum over the vCPUs; the `Vcpu` objects outlive the
  run (their channels do not).
- **The host threads.** `VM.thread_vcpus` (default 1). With n > 1, `steps` has the monitor make at least n vCPUs, the
  runtime lock `shared`, and starts a host thread for each of vCPUs 1 .. n-1 (`_serve_vcpu`): it waits for a ready
  thread no vCPU holds, loads it, runs its channel and serves each exit with `serve_exit` under the lock; the lock
  also covers each vCPU's words written before a run (SLICE_UNTIL reads the thread table). A vCPU with no thread to
  run parks: it gives the lock back and waits on the VM's condition, which every serve notifies while a vCPU is
  parked. A stop on any vCPU (a thread's end that ends the process, an exception) stops them all: the host threads
  see it at their next exit or when kicked (Sprint 117), and `steps` joins them before it returns.
- **No false deadlock.** On any vCPU, `_switch` with no thread to switch to while another vCPU holds one parks
  instead of answering IDLE or raising the deadlock.
- **The record names it.** `scripts/family_run.py --vcpus N`: the VM's `thread_vcpus`; with N > 1 the run's
  baseline gains `vcpus` and `vm_inputs` (the discrepancy's state hash) `vcpus`, and the family index is left as the
  one-vCPU sweep wrote it.

Out of scope, by design: a machine of several processes with several vCPUs (`kernel.Machine` runs other VMs on its one
thread while this VM's vCPUs run); a thread suspended or read while another vCPU runs it (chain step 6, judged by the
threads family, whose probe suspends a spinning thread).

## prerequisites

- 117.

## context_files

- tools/guest_runtime/threads.c (`w2m_switch`, `slice_until`, the run entry); src/w2m/scheduler.py (`GuestThread`,
  `E_*`); src/w2m/vm_backend.py (`steps`, `serve_exit`, `_switch`, `_schedule`, `_load_thread`, `_save_thread`,
  `_start_vcpus`, `RuntimeLock`, `Vcpu`); src/w2m/kernel.py (`Machine.run`, `_next`); scripts/family_run.py
  (`run_in_vm`, `vm_inputs`, the baseline, `main`); probes/wait_handoff/wait_handoff.c.

## signal contract

### Emits

- On the wait_handoff run on two vCPUs (`family_run.py wait_handoff --run --vcpus 2`), its record's baseline naming
  `vcpus` 2: SESSION_INIT (purpose conformance), CANDIDATE_GENERATED, COMPARISON_COMPLETED for each of its runs
  (verdict conforms, first_divergence none), CANDIDATE_ACCEPTED, SESSION_COMPLETED (outcome completed).
- On the batch run's record, as on Sprint 117's: SESSION_INIT, EXECUTION_STARTED, MEASUREMENT_RECORDED 107 times,
  EXECUTION_COMPLETED (exit_status killed), TRACE_INGESTED, COMPARISON_COMPLETED (first_divergence none),
  SESSION_COMPLETED. Per class run (458) and family run (68), as on Sprint 117's sweep: SESSION_INIT,
  CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED (456 and 68) or for FCOS and FSIN REDUCTION_COMPLETED,
  DISCREPANCY_REPORTED and DISCREPANCY_ROUTED, SESSION_COMPLETED.

### Invariants

- Vocabulary 0.9 unchanged (`vcpus` is a baseline key, not a payload field).
- One vCPU (`thread_vcpus` 1) behaves as before.

## artifact contract

- The mark in threads.c, scheduler.py and vm_backend.py; `thread_vcpus`, `_serve_vcpu`, the park; `--vcpus` in
  family_run.py; `scripts/gates.sh` exit 0; CI `success`.

## observation contract

- **Judge:** `family_run.py wait_handoff --run --vcpus 2` conforms on both reference runs, its record's baseline
  `vcpus` 2, and the VM's per-vCPU counts show vCPU 1 ran guest instructions (printed by the run); the same family on
  one vCPU conforms as before. The batch run conforms (168 lines, 0 divergences, the same eight exit kinds); 456 of 458
  classes, 68 of 68 families; the litmus, runtime-lock and kick tests.
- **Plant:** the mark ignored by both switches (a thread another vCPU holds may be chosen): wait_handoff on two vCPUs
  does not conform, or stops; measured over repeated runs before it is trusted.

## done criteria

One process's threads run on two vCPUs at once with the same rows as on one; one vCPU unchanged; CI green.
