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

