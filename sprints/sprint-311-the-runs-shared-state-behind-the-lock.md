# Sprint 311 — The run's shared state behind the lock

```yaml
---
id: 311
status: active
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's card 4c, first part (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`, card 4; split at this
card into 4c1, this, and 4c2, the threads). Sprint 310 put `Guest` behind `GuestLock`, but the state the design's "The
run loop's own state" calls Served stays outside it: `HostCompiler`'s tables, reached at `hvc #1` and `#13`, and the
records `run` writes into `Outcome` (`bridged`, `blocks` and `translations` at `#1`, `created` and `thread_requests`
at `#8`, `handovers` at `#12`). At every exit, too, `vm_compiled` drains the in-VM compiler's log under the word
(Sprint 275) into `blocks`, `translations`, `vm_compile_starts`, `instructions_at` and the host compiler, and
`exit_count` then reads the host compiler's `tails` holding nothing (`run.rs` 864-884); Sprint 309's reading of the
loop listed neither. Two vCPU threads would race on all of it. And `turn` waits on the other processes with the guard
held (Sprint 310's Observation 2), the inversion the guest-memory design's choice 6 forbids.

- **The work:** `GuestLock` holds the run's shared state, the guest's borrow, the host compiler's borrow and those eight
  records, one value behind the one `Mutex`; `serve()` and `get_mut()` give it; `into_inner()` gives it back at the
  run's end, the records moved into `Outcome`. The drain takes the lock inside its word, and the exit's count reads the
  tails inside the same hold. `turn` takes the lock and the guard: it lets the guard go, then the
  word, around the link's wait (`Supervisor::hand_over`, `Child::yield_turn`), and takes the word, then the guard,
  after; the link and the kernel objects out of `Guest` across the wait, as they are today; in debug builds the wait
  asserts the lock free. One vCPU still; every run as held.
- **Not here:** the threads, the stop, the kick, the merged outcome and the stop flag (4c2); the turn's start's reads
  of `served.link` and `served.threads` (4c2's measure, the link being out of `Guest` across another vCPU's wait).

## prerequisites

- 310 (closed)

## context_files

- the design's "The run loop's own state", "How the threads share `Guest`" and card 4; the guest-memory design's
  choice 6; `run.rs` (`run`, `turn`, `hand_over`, `Outcome`); `guest.rs` (`GuestLock`); `turns.rs` (the link's waits)

## signal contract

### Emits

- None; the lock's contents.

### Invariants

- Every run as held; one vCPU.

## artifact contract

### Files modified

- `crates/w2m_vm/src/guest.rs`, `crates/w2m_vm/src/run.rs`; the design's card 4 split

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** every run as held: the family runs, a child process's turns among them, the qpc and sleeper
  runs and the batch run as recorded; `HostCompiler` and the eight records reached only through the lock's guard.
- **Planted faults:** (1) `turn` keeping the guard across the link's wait: a family run with a child process must
  panic on the wait's assertion, naming the lock; (2) the host compiler reached outside the guard, as `run` took it
  before: the build must fail, the borrow being inside the lock.

## done criteria

Every part of the run that a served exit writes is behind the one lock, and no wait holds it.

## self-review checklist

- [x] Scope names one verifiable property: the run's Served state behind the lock, no wait holding it.
- [x] context_files cite the design, the lock order, the run loop and the link.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what its judge reads.
- [x] One concept: the lock's contents and its hold, two files.

## result

