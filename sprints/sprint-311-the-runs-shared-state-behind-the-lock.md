# Sprint 311 — The run's shared state behind the lock

```yaml
---
id: 311
status: done
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

- **The lock and what it holds** (`run.rs` 605-681): `Records`, the eight records; `Shared`, the guest's borrow, the
  host compiler's borrow and the records; `GuestLock` moved from `guest.rs` to `run.rs`, beside the state it guards,
  a `Mutex<Shared>` and the mapping's view, `view()` returning a view as long-lived as the guest's borrow, so a view
  taken before a guard stays usable after it;
  `into_inner()` hands the records to `Outcome` at the run's end (1342). The compile-time `Sync` assertion holds with
  `HostCompiler` inside.
- **The run loop:** whether the host compiles read once before the lock (822); the drain and the exit's count take the
  word (918), then the lock, the drain writing `records` and the host, the count reading the tails under the same
  hold, the word released after the guard, an `exit_count` error raised after the release; `hvc #13`, `#8`, `#12`,
  `#4` and `#1` as Sprint 310 made them, their records written through the guard; the hand-overs recorded under the
  lock before it is let go; `Stop::Repeated`'s check of `bridged` moved inside `hvc #1`'s hold; `hvc #9`'s write of
  `served.unimplemented` under the word and the lock (1259), a served write the card's list had missed.
- **`turn`** (689-729): takes the lock and the guard and hands a guard back; the link and the kernel objects out of
  `Guest`, the guard let go, then the word, the wait, the word taken, then the guard; `hand_over` folded into it.
- **The drain:** `vm_compiled` no longer takes the word itself; `vm_compiled_waiting` and its test (Sprint 276, the
  drain's wait for another vCPU's hold) removed, the wait being the host's take's, which `monitor_code`'s test of
  Sprints 275-276 holds, and the drain's hold now `serve()`'s assertion.
- **Found and fixed, a class:** the gates run `w2m-vm` built `--release`, whose profile (`Cargo.toml` 16-20) has no
  `debug-assertions`, so every `debug_assert!` was compiled out of the family runs, the two-process guest and the batch
  run; plant 1, written as a debug assertion, passed the family test (which reads stored stops, 0.11 s). The class in
  `w2m_vm` and `hypervisor`, six: `serve`'s word check, `turn`'s lock check and the scheduler's word check (Sprint 294,
  `threads.rs` 707) now `assert!`, run in release (two relaxed loads a served exit, one `try_lock` a turn); the
  stop-companion table check (`run_outputs.rs` 18) and the two destroy codes in `hypervisor`'s `Drop`s stay debug.
- **Expected values:** `twoproc.exe` on the release build with the instruction clock, `ExitProcess { code: 0 }`
  printing "child ended\n", as `store/batch_run/first-stop-2026-10-28.tsv` 54 holds; every other run by the gates.
- **The unit test** moved to `run.rs` (1474), its lock built with no host.
- **Plant 1,** restored by `cp` and checked by `cmp`: `turn`'s `drop(held)` struck; `twoproc.exe` on the release
  build exited 101, "panicked at crates/w2m_vm/src/run.rs:705:5: a turn's wait with the run's lock held", no stop
  written. (Before the class's fix, the same plant passed the family test, a debug assertion in a release run.)
- **Plant 2,** restored likewise: the host compiler read inside the loop; the build failed, E0502, "cannot borrow
  `host` as immutable because it is also borrowed as mutable".
- **Changed from the card:** plant 1's judge is the two-process guest, not a family run, the family test reading stored
  stops; a third file, `threads.rs`, for the scheduler's assertion.
- **The gates:** `scripts/gates.sh` exit 0 in 747 s, kept in `store/gates/passes.tsv` as sprint 311 at 6ed36976a with root=2adcd9e42 and "rust=crates python=0 samples=0", 184 tests, 651 ns an exit at load 4.08 (Sprint 310: 661 ns at load 2.98, the checks run in release with no time a single run shows), base=56ec1421d, Sprint 310's close; multi-vCPU: not run (Python gates not run)
