# Sprint 310 — The guest behind one lock

```yaml
---
id: 310
status: active
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's card 4b as Sprint 309 decided it (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`, "How
the threads share `Guest`"; BLACKBOARD Decision "Architecture: how the vCPU threads share `Guest`"): the whole `Guest`
behind one `Mutex`, taken once per served exit inside the runtime word, its acquire asserting the word in debug builds
(card 2's guard and plant). On one vCPU the run loop owns the lock, so outside a served exit it reaches `Guest` by
`Mutex::get_mut`, no lock taken; card 4c turns those reaches into each thread's copies and view.

- **The work:** `guest::GuestLock`, a `Mutex<Guest>`: `serve(&self) -> MutexGuard<Guest>`, a debug assertion first that
  this vCPU holds the runtime word (`monitor_code::runtime_word_held`, `vcpu_owner`, through the mapping read without
  the lock), a poisoned lock taken as its panicking holder left it, named; `get_mut`; `into_inner`. `run` takes the
  guest into the lock at its start and gives it back at its end, its callers unchanged; the served exits, `hvc #1`,
  `#4`, `#12`, `#13` after `take_word`, and `hvc #8`'s `serve` under the guest's own hold, reach `Guest` through
  `serve()`; every other reach through `get_mut`. Deferred (Sprint 298)'s clause read at the start.
- **Not here:** a second thread (4c), the turn's start's two reads (4c's measure).

## prerequisites

- 309 (closed)

## context_files

- the design's "How the threads share `Guest`" and card 4; `run.rs` `run` (the four `take_word` sites, `serve`'s call);
  `guest.rs`; `monitor_code.rs` (`runtime_word_held`, `vcpu_owner`)

## signal contract

### Emits

- None; a lock and its guard.

### Invariants

- Every run as held; one vCPU.

## artifact contract

### Files modified

- `crates/w2m_vm/src/guest.rs` (`GuestLock`), `crates/w2m_vm/src/run.rs`; a unit test of `serve`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** every run as held, `serve()` taken at each served exit with the word held; the unit test
  refusing `serve()` with the word free.
- **Planted faults:** (1) `hvc #12`'s service reaching `Guest` through `serve()` before its `take_word`: a run-loop test
  must panic on the assertion, naming the word; (2) the assertion removed: the unit test that takes `serve()` with the
  word free must fail for want of the panic.

## done criteria

`Guest` is behind one lock, taken at each served exit with the word asserted held; the run loop reaches it otherwise
without locking, ready for 4c.

## self-review checklist

- [x] Scope names one verifiable property: a served exit's lock taken under the word.
- [x] context_files cite the design, the Decision, the run loop and the word.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what its judge reads.
- [x] One concept: one lock and its guard, two files.

## result

- **The lock** (`guest.rs` 636-678): `GuestLock<'a>`, a `Mutex<&'a mut Guest>` and a view of the mapping, made once
  from `GuestMemory::as_ptr` at creation; `view()`, the mapping without the lock; `serve()`, which asserts in debug
  builds `runtime_word_held(view, vcpu_owner(view))` before it locks and takes a poisoned lock as its holder left it;
  `get_mut()`. **Changed from the scope:** the lock holds the run's borrow, not the guest, and has no `into_inner`:
  `Guest` has no `Default`, so `run` could not move it out of its `&mut Guest` with its callers unchanged; dropping the
  lock (run.rs 1253) gives the borrow back. 4c's threads, inside `std::thread::scope` so none outlives the borrow,
  share the lock by `&`.
- **The lock made `Sync`, a second work commit:** the first commit (afc8727bc) kept the mapping as `(*mut u8, usize)`,
  which made `GuestLock` neither `Sync` nor shareable by threads, against the done criteria's "ready for 4c"; a probe
  crate outside the tree showed `Guest` `Send`, `Mutex<&mut Guest>` `Sync` and `GuestLock` not, for `*mut u8`. The lock
  now keeps a `ConcurrentGuestMemory<'a>` (`Sync` and `Copy`, `concurrent_memory.rs` 40-50), made once in `new`, so
  `view()` has no `unsafe`; a compile-time assertion (`the_lock_is_shared_by_threads`, 642-645) holds it `Sync`. The
  gates of afc8727bc were stopped before their end, no row written.
- **The run loop** (`run.rs`): the lock made before the loop (773), `get_mut` at its top (775); `take_word` takes the
  lock's view (627), so the word is taken before the lock; `serve()` at `hvc #13` (932, a temporary dropped before the
  word's release), `#8` (950, under the guest's own hold, the loop's `mem` shadowed by the lock's view), `#12` (1033),
  `#4` (1124) and `#1` (1211); at `#12`, `#4` and `#1` the guard dropped before the word is released (1044, 1158,
  1244: the lock let go first, the word outermost) and `get_mut` taken again for the reads after. `threads.rs`' test
  module and its `guest_with_a_thread` made `pub(crate)` for the unit test, a third file.
- **Found for 4c:** `turn` (run.rs 639-648) releases the word for `hand_over`'s wait on the other processes and takes it
  again after, called at `#12` and `#4` with the guard held. With two vCPUs that is a lock-order inversion: a second
  vCPU takes the word in the gap and waits on the lock, while the first holds the lock and waits on the word. On one
  vCPU no second taker exists. 4c drops the guard across the wait, as choice 6 requires (reviewer-3 at 310).
- **The unit test** `guest::tests::a_served_exit_takes_the_lock_only_under_the_word`: the word free, `serve()` panics
  "a served exit's lock taken without the runtime word" and leaves the lock free; the host holding it, the guard gives
  the guest, its second thread present.
- **Plant 1,** restored by `cp` and checked by `cmp`: `hvc #12`'s `serve()` moved before its `take_word`;
  `run_loop::sleeper_creates_and_ends_its_thread_as_the_python_host` failed, panicked at `guest.rs:657:9`, "a served
  exit's lock taken without the runtime word" (the assertion, 667 now).
- **Plant 2,** restored likewise: the assertion removed; the unit test failed, "a served exit with the word free: ()".
- **Plant 3,** restored likewise: a `PhantomData<*mut u8>` field added to `GuestLock`; the build failed, E0277 at the
  assertion (644), "`*mut u8` cannot be shared between threads safely".
- **Deferred (Sprint 298),** read at the start: no change to `struct w2m_state` or a monitor-data slot, G4 not met; the
  three cards stay deferred.
- **The gates:** GATES-LINE
