# Sprint 320 — How a parked vCPU's wait ends

```yaml
---
id: 320
status: done
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
  card 4c3b2's scope in the design, with this card's commit); `threads::park_deadline(guest)`, the earliest deadline of
  a blocked, unsuspended thread as an `Instant` on the host's clock (the guest's counter in 100 ns units, `counter_now`),
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

- `crates/w2m_vm/src/threads.rs`; unit tests of the wait's three ends; the run-loop design's card 4c3b2 (the stop
  notifies `Sched`)

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

- **The wait** (`threads.rs`): `Sched`'s counts, the changes, the vCPUs parked and a declared deadlock, under one
  mutex; `Woken` (566), `Changed`, `Deadline` or `Deadlock`; `Sched::park(seen, deadline, vcpus)` (591), its deadline
  a fixed `Instant`, its quiet second started when every vCPU is parked with no deadline, a wait with neither taking no
  timeout of its own; `park_deadline` (637), the earliest deadline of a blocked, unsuspended thread on the host's clock
  from the guest's 100 ns counter. Sprint 319's `wait_change` removed, its test moved to `park`: one way to wait.
- **Found by the test, a deadlock only one vCPU saw:** the first form ended the park of the vCPU whose second ran out
  and told no other; the first of two to park, alone then and waiting with no timeout, waited on, and the test run hung
  past its 600 s bound (stopped by `TaskStop`, no process left). The deadlock is now set in the counts and broadcast
  (615), and every parked vCPU ends on it, the whole run's deadlock as the Python host's rule has it.
- **Not built here, moved:** a vCPU woken by its deadline making expired waits ready before it takes is the run loop's
  park sequence, card 4c3b2's; `wake_expired` exists and is called there.
- **The unit tests:** `a_parked_wait_ends_at_its_deadline_at_a_change_or_in_a_deadlock_of_every_vcpu` (1531): a park
  50 ms ahead ends `Deadline` within its bound; one of two parked with no deadline waits past 1.3 s and ends `Changed`
  at a notify; two of two end `Deadlock` after one second; `the_parks_deadline_is_the_earliest_blocked_threads`
  (1579): of 50 ms and 10 s ahead, the deadline within 60 ms. With Sprint 319's, ten `threads` tests pass in 2.38 s.
- **Plant 1,** restored by `cp` and checked by `cmp`: every parked count taken as the whole run (`parked >= 1`); the
  test failed, "one of two vCPUs parked with no deadline ended its wait", left Deadlock, right Changed.
- **Plant 2,** restored likewise: the latest deadline (`max`); the test failed, "the park's deadline 9.999999092s
  ahead, not the earliest's 50 ms".
- **A second work commit, from an audit after the first:** the first commit's rule judged "every vCPU parked and no
  deadline" by the vCPU's own deadline, read at its park. `park_deadline` reads every blocked thread, so a vCPU parking
  after another sees its deadline; but a vCPU parked with no deadline that `Condvar` wakes with no change, which it
  allows, re-judged by its own stale none and could declare a deadlock while another parked vCPU's deadline stood. The
  counts now hold `timed`, the parked vCPUs with a deadline, and the quiet second runs only when it is 0. The first
  commit's gates were stopped before their end, no row written. The test gains the case, its spurious wakeup made by a
  bare `notify_all`; its first form, without that wakeup, passed with the old rule planted, a judge that did not reach
  the line, found by running the plant before the commit.
- **Plant 3,** restored by `cp` and checked by `cmp`: the old rule put back (`deadline.is_none()` for `timed == 0`); the
  test failed, "a deadlock declared while another parked vCPU's deadline stood", left (Deadlock, Deadlock), right
  (Changed, Deadline).
- **The gates:** `scripts/gates.sh` exit 0 in 744 s, kept in `store/gates/passes.tsv` as sprint 320 at 86dcbf605 with root=7775b1174 and "rust=crates python=0 samples=0", 191 tests, 658 ns an exit at load 2.87, base=dbe5e7da4, Sprint 319's close, the run on 52387e735 stopped by the audit; multi-vCPU: not run (Python gates not run)

