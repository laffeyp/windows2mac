# Sprint 320 — How a parked vCPU's wait ends

```yaml
---
id: 320
status: active
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's "A vCPU with no thread" (Sprint 318), its timed wait and its deadlock: Sprint 319's `Sched`
waits on a change with a timeout the caller gives, and nothing yet says what that timeout is or when a run of parked
vCPUs is deadlocked. The Python host's Sprint 120: every host wait has the earliest deadline of a blocked thread as its
timeout and makes expired waits ready when it wakes; a run is deadlocked only when every vCPU waits a whole second with
no deadline and no notify.

- **The work:** `Sched` counting the vCPUs parked against the vCPUs the run has; `Sched::park(seen, deadline, vcpus)`
  waiting until a change, the deadline (an `Instant`, fixed, so a spurious wakeup does not re-arm it), or, with every
  vCPU parked and no deadline, one second with no change, and saying which (`Woken::Changed`, `Woken::Deadline`,
  `Woken::Deadlock`); a wait with neither has no timeout of its own, the run's stop notifying `Sched` (written into
  card 4c3b2's scope in the design, with this card's commit); `threads::park_deadline(guest)`, the earliest deadline of a
  blocked, unsuspended thread as an `Instant` on the host's clock (the guest's counter in 100 ns units, `counter_now`),
  none without one; a vCPU woken by its deadline making expired waits ready (`wake_expired`) before it takes. One vCPU
  never parks.
- **Not here:** `slice_until` past the waiting vCPUs (card 321); the run loop's park (4c3b2).

## prerequisites

- 319 (closed)

## context_files

- the run-loop design's "A vCPU with no thread" and card 4c3b2; `docs/design-several-vcpus-2026-10-17.md` 176-192
  (Sprint 120); `threads.rs` (`Sched`, `counter_now`, `wake_expired`, `timed_waiter`)

## signal contract

### Emits

- None; the scheduler's waits.

### Invariants

- Every run as held; one vCPU.

## artifact contract

### Files modified

- `crates/w2m_vm/src/threads.rs`; unit tests of the wait's three ends; the run-loop design's card 4c3b2 (the stop notifies `Sched`)

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** every run as held; unit tests: a park with a deadline 50 ms ahead ends `Deadline` within its
  bound; two vCPUs both parked with no deadline end `Deadlock` after one second; one of two parked with no deadline
  waits on and ends `Changed` at a notify; `park_deadline` reading a blocked thread's deadline back.
- **Planted faults:** (1) the deadlock rule counting parked vCPUs but not the run's: one of two parked ends `Deadlock`,
  and its test names it; (2) `park_deadline` taking the latest deadline: the deadline test waits past its bound and
  names it.

## done criteria

A parked vCPU's wait ends by a change, by the earliest deadline, or by a deadlock only every vCPU's wait can make.

## self-review checklist

- [x] Scope names one verifiable property: the three ends of a parked vCPU's wait.
- [x] context_files cite the design, the prior form and the scheduler.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what its judge reads.
- [x] One concept: one wait and its ends.

## result

