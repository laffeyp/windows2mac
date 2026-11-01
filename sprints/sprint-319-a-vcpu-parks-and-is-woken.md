# Sprint 319 — A vCPU parks and is woken

```yaml
---
id: 319
status: active
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's card 4c3b1 ("A vCPU with no thread", Sprint 318; BLACKBOARD Decision "Architecture: a vCPU
with no thread"), its park, wait and wake; the deadlock over every vCPU and `slice_until` past the waiting vCPUs are
the card's second part, 4c3b1b, and the run loop's use of them 4c3b2.

- **The work:** `threads::Sched`, a change count under a `Mutex` and a `Condvar`, `notify` and `wait_change(seen,
  timeout)`, one per guest (`Guest.sched`, an `Arc`, outside the lock); `RT_WAITING`, the run table's `waiting` word
  (0x28); `threads::set_status` notifying on a change to READY; `THREAD_WAKE` notifying; `switch` returning
  `Switch::Parked` when no thread this vCPU may take is ready and a thread is held by another vCPU, its own thread put
  down (saved, its `vcpu` word cleared); `threads::take`, the load of a ready thread onto a vCPU with no thread, the
  switch's load half alone. One vCPU never parks: no thread is held elsewhere.
- **Not here:** the run loop's park (4c3b2), the deadlock across vCPUs and `slice_until` (4c3b1b).

## prerequisites

- 318 (closed)

## context_files

- the run-loop design's "A vCPU with no thread"; `threads.rs` (`switch`, `set_status`, `schedule`); `host_request.rs`
  (`THREAD_WAKE`); `tools/guest_runtime/threads.c` 50-53, 124-127; `guest.rs` (`private_chain`, `VcpuPages`)

## signal contract

### Emits

- None; the scheduler's states.

### Invariants

- Every run as held; one vCPU.

## artifact contract

### Files modified

- `crates/w2m_vm/src/` (`threads.rs`, `guest.rs`, `host_request.rs`); a unit test of a park and a wake

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** every run as held; a unit test, two host threads on one guest: vCPU 1 (its private pages, owner
  2) switches with nothing ready it may take while vCPU 0's thread is held by owner 1, is `Parked`, its thread put down
  and `waiting` 1, waits on `Sched`; vCPU 0 readies a blocked thread through `set_status`; vCPU 1 wakes within the
  test's bound, `take`s that thread, its `vcpu` word 2, `waiting` 0.
- **Planted faults:** (1) `set_status` without its notify: vCPU 1 waits its whole bound and the test names the missed
  wake; (2) `take` loading a thread held elsewhere (its `here` dropped): the test names the thread loaded.

## done criteria

A vCPU with nothing it may run parks, its thread put down, and a thread readied anywhere in the host wakes it to take
one.

## self-review checklist

- [x] Scope names one verifiable property: a parked vCPU is woken by a readied thread and takes it.
- [x] context_files cite the design, the scheduler and the C runtime's half.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what its judge reads.
- [x] One concept: one park and its wake.

## result

