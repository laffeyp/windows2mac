# Sprint 329 — A stop found at the word's take

```yaml
---
id: 329
status: active
phase: 6
pass_kind: functional
---
```

## scope

Deferred (Sprint 327) and reviewer-3's Low 2 at 327. At a run's stop, `monitor_code::release_ended_vcpus_word` lets go
an ended vCPU's guest's hold of the runtime word, so a vCPU waiting on the word is not left waiting for ever. The
waiter's service then reads what the word guards (the code cursor, the reconstruction and dispatch tables, the run
table), which the ended vCPU's guest may have left half-written when `hv_vcpus_exit` stopped it, before any turn's
start finds the run's stop flag. Every service takes the word through `take_word` (`run.rs` 685; ten calls in
`vcpu_loop` and `take_a_thread`) or, after each exit, `host_take_runtime_word` directly (`run.rs` 1287).

- **The work:** `take_word` takes the run's stop (`RunStop`) and returns `Option<bool>`: `None` when, after the take,
  the run has stopped, the word given back if the host took it; each caller returns before its service reads,
  `vcpu_loop` with `Ok(None)`, the vCPU ending recording nothing; the after-exit take goes through `take_word`. The
  type makes each caller say what a stop does.
- **Not here:** `turn`'s take (`run.rs` 814), the Machine's wait: a process with a machine link runs on one vCPU
  (`MACHINE_ON_ONE`, Sprint 325), so no other vCPU there leaves state half-written; card 4f, which gives the Machine
  its own thread, takes it.

## prerequisites

- 328 (closed)

## context_files

- BLACKBOARD Deferred (Sprint 327), the word released at a stop; reviewer-3's Low 2 at 327; `crates/w2m_vm/src/run.rs`
  `take_word`, `RunStop`, `vcpu_loop`, `take_a_thread`, `run_on_vcpu`; `crates/w2m_vm/src/monitor_code.rs`
  `host_take_runtime_word`, `release_ended_vcpus_word`

## signal contract

### Emits

- None; where a stopped run ends a service.

### Invariants

- Every run as held, on one vCPU and two.

## artifact contract

### Files modified

- `crates/w2m_vm/src/run.rs` (its unit tests among it)

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** a unit test of `take_word`: with the run's stop not set and the word free, `Some(true)` and the
  word the host's; with the stop set and the word free, `None` and the word free after; with the stop set and the
  word held by this vCPU's guest, `None` and the word still its guest's. `qpc`, `sleeper` and `twoready` on two vCPUs
  and `statsize` through `w2m-vm` as held; every run on one vCPU as held.
- **Planted faults:** (1) the stop's check taken out of `take_word`: the unit test's stopped case reads `Some(true)`;
  (2) the word not given back when a stopped take finds the host took it: the unit test reads the word still the
  host's. Each judge's reach read before this card's commit (WORKING_AGREEMENT, Sprint 284): the unit test calls
  `take_word` itself, with a `RunStop` it sets, on a guest's memory, and reads the word after.

## done criteria

No service reads the word's state once its run has stopped: `take_word` finds the stop before the service runs.

## self-review checklist

- [x] Scope names one verifiable property: a stopped run's take ends the service.
- [x] context_files cite the Deferred entry, the review and the takes.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each judge's reach read.
- [x] One concept: the stop at the word's take.

## result

- **The take** (`crates/w2m_vm/src/run.rs`): `take_word(mem, monitor, outside, stopping: &RunStop) -> Option<bool>`,
  the old take renamed `take_word_unchecked` and called first, then `stopping.stopped()` read: None when the run has
  stopped, the word given back when the host took it, a hold of this vCPU's guest left as it was.
- **The takes it serves:** the card counted ten `take_word` calls in `vcpu_loop` and `take_a_thread`; there were nine
  there, eight in `vcpu_loop` and one in `take_a_thread`, and a tenth in `turn`. The nine and the after-exit take,
  which called `host_take_runtime_word` directly, now go through `take_word`, ten in all, each returning on None
  before its service reads: `vcpu_loop` with `Ok(None)`, `take_a_thread` with `Taken::Ended`. `turn`'s, the Machine's
  wait on one vCPU, calls `take_word_unchecked`. The after-exit take now counts holds found outside the word as the
  others do, which on one vCPU stays 0. `host_take_runtime_word` and `take_word_unchecked` are named in `run.rs` only
  by `take_word_unchecked` itself, `turn`, and the lock's and this card's unit tests (a search; no type holds it).
- **The test** `run::tests::a_stopped_runs_take_of_the_word_ends_its_service`: a running run's take `Some(true)`, the
  word the host's; a stopped run's take of a free word None, the word 0 after; a stopped run's take of a word this
  vCPU's guest holds None, the hold kept; passing, with the lock's test beside it. It first failed to compile, `RUN_TABLE`
  out of the test module's scope, and names it by its path.
- **Plant 1,** `run.rs` copied aside and restored by `cp`, checked by `cmp`: `if false && stopping.stopped()`; the test
  failed, "assertion `left == right` failed: a stopped run's take", left `Some(true)`.
- **Plant 2,** restored likewise: the give-back made `let _ = mem;`; the test failed, "assertion `left == right` failed:
  the word given back", left 4294967295, `HOST_OWNER`.
- **Beyond the card, from reviewer-3's Medium 1 at 329** (the review in Surfaced): `run_on_vcpu` let go its guest's
  hold of the word only on its `Ok` path, after `vcpu_loop(...)?`; a thread that ended by an error or a panic kept
  it, and a peer in `host_take_runtime_word`'s spin, which reads no flag and returns to no check, waited for ever.
  `HaltAtEnd` carries the guest memory and the vCPU's owner, and its `Drop` halts the run, then lets go the guest's
  word (`release_ended_vcpus_word`), on every path out of the thread; the flag stored first, so a take that then gets
  the word finds the stop. The `Ok` path's own release is gone. The same concern as the card's, the run's stop and
  the word, so in this sprint.
- **One owner function:** `owner_of(k)`, `k + 1`, written into `LOCK_OWNER` at the set-up and held by `HaltAtEnd`, so
  the two cannot drift; the two-vCPU test's read of vCPU 1's `LOCK_OWNER` as 2 holds both.
- **The test** `run::tests::a_thread_that_ends_by_a_panic_lets_go_its_guests_word`: vCPU 1's guest, owner 2, holding
  the word, a `HaltAtEnd` dropped through a panic; the word 0 and the run halted after; passing. A `Drop` runs alike on
  an error's return, so the panic holds both paths; the review's run-level form, a thread made to fail while another
  waits in a take, is not built, the window not reachable on demand.
- **Plant 3,** restored likewise: the `Drop`'s release made `let _ = (self.mem, self.own);`; the test failed,
  "assertion `left == right` failed: the guest's word let go".
- **Every run as held:** `cargo nextest run -E 'rdeps(w2m_vm)'`, 202 tests passed in 433 s, the two-vCPU runs among
  them; `cargo clippy` with the gates' flags clean.
- **The gates:** GATES-LINE

