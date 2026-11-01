# Sprint 324 — The run loop takes the lock as a thread would

```yaml
---
id: 324
status: active
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's card 4c3b2 runs a host thread per vCPU over one `&GuestLock`. Its run loop (`run.rs` `run`)
reaches `Guest` five times through `GuestLock::get_mut` (`run.rs` 844, 961, 1143, 1262, 1370), which needs the lock
borrowed `&mut`, which two threads cannot hold; the borrow at 844 serves the rest of the body too: the exit ledger
(897), `time_runtime_word` (906), the block judge (924), the model step (`model_step(guest, …)`, 1175) and the slice
end's test of `served.link` (1194). The design decided how a thread reaches `Guest` outside a service ("How the
threads share `Guest`", Sprint 309): each vCPU thread keeps a copy of the Load facts it reads, and the turn's start
reads `served.link` and `served.threads` either under the word and the lock or from memory the services write, card 4c
to measure the first against the second. No card has built it; card 4c3b2 as the design states it would build it with
the threads. This card builds it on one vCPU first, so 4c3b2 changes how many threads run the loop and not how the
loop reaches `Guest`.

- **The work:** the loop body reaches `Guest` only through `GuestLock::serve`, under the word, and through a copy of
  the Load facts made before the loop (`code.dispatch_va`, `code.miss_va`, `ttbr`, `entry_va`, `time_runtime_word`);
  the turn's start takes the word and the lock for its reads of `served.link` and `served.threads` (the design's
  first form), and its time is measured against the run as held; the slice end's test of `served.link` read under the
  word and `serve`, not copied: a request makes the supervisor's link mid-run (`host_request.rs` 527); the model step under
  the word and `serve`, as the other services; the five `get_mut` sites gone from the loop, the diagnostics' reads
  (`exit_ledger`, `block_judge`), refused on several vCPUs (Sprint 308), under the same `serve`.
  `get_mut` stays for the run's set-up and its end.
- **Not here:** a second thread; the second form (the services writing the slice's end into the vCPU's monitor data),
  which the measurement decides whether to build.

## prerequisites

- 323 (closed)

## context_files

- `docs/design-run-loop-on-several-vcpus-2026-10-31.md` 136-152 ("How the threads share `Guest`") and 206-215 (the
  cards); `crates/w2m_vm/src/run.rs` `run` 760-1408 and `GuestLock`; `crates/w2m_vm/src/threads.rs` `slice_until`

## signal contract

### Emits

- None; how the run loop reaches its state.

### Invariants

- Every run as held: `qpc`, `sleeper`, the model-steps run and the batch run end as before.

## artifact contract

### Files modified

- `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/tests/` (the source test); with the card, the design's card list (4c3b2
  split, as at Sprints 311, 312 and 316); with the work, the design's "How the threads share `Guest`" (the form the
  measure decided)

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** a source test, `the_run_loop_takes_the_lock_only_to_serve`, reading `run`'s loop between its
  `loop {` and the loop's end and failing on any `get_mut(`; `qpc` and `sleeper` ending as held; the turn's start
  timed per turn on `sleeper` and the batch run, its median and 99th percentile against the median exit service
  time, kept in the result; the rule the measure decides by, set before it is taken: the turn's start's median above
  the median exit service time opens the second form's card before 4c3b2, else the first form stands and the design
  says so.
- **Planted faults:** (1) one `get_mut` put back at the turn's start: the source test fails naming its line; (2)
  `serve` called at the turn's start without the word taken: `serve`'s assertion, "a served exit's lock taken without
  the runtime word", stops the run.

## done criteria

The run loop on one vCPU touches `Guest` only under the word and the lock, as a second vCPU's thread would.

## self-review checklist

- [x] Scope names one verifiable property: no whole-lock borrow inside the run loop.
- [x] context_files cite the design's decision, the loop and the lock.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: a source test, the runs as held, a measurement, and two plants, each changing what its
  judge reads.
- [x] One concept: the loop's reach to `Guest`.

## result


- **The loop** (`crates/w2m_vm/src/run.rs`): `vcpu_loop(lock: &GuestLock<'_>, vcpu, pages, max_steps, facts,
  out)`, the run loop moved out of `run`, which keeps `get_mut` for the set-up and the end and calls it with `&lock`
  (reviewer-3's Low 1 at 324, the review in BLACKBOARD Surfaced). The five whole-lock borrows are gone: the turn's
  start takes the word and `serve` for the slice's end and whether another process is due, timed into the new
  `Outcome.turn_start_ns`; after each exit one section under the word and `serve` holds the ledger, the exit counted
  for the word's timing, the block judge, the drain, the count and the ledger's row, a stop inside it leaving before
  the word is let go; the model step under the word and `serve`; the slice end's test of `served.link` under its own
  take, the link never removed once made; `dispatch_va`, `miss_va`, `time_runtime_word` and whether the host compiles
  copied before the loop as `LoadFacts`. The word is in the monitor region (`RUN_TABLE`, below `MONITOR_SIZE`), so the
  take changes nothing the ledger hashes.
- **The judge changed with the review:** the card named a scan of the loop's text for `get_mut(`,
  `the_run_loop_takes_the_lock_only_to_serve`, and its plant 1 that scan's failure. With the loop holding the lock by
  `&`, the compiler refuses `get_mut` in it and in anything it hands the lock to, so plant 1's judge is the compiler, and
  `tests/run_loop_lock.rs` holds what the compiler does not, the signature: `the_run_loop_holds_the_lock_by_ref`
  (`vcpu_loop` takes `lock: &GuestLock<'_>`, `run` calls it with `&lock` and keeps no loop of its own) and
  `a_mut_lock_or_a_loop_in_run_is_named`. No `compile_fail` doc test: `vcpu_loop` is private, out of a doc test's
  reach, and one on `GuestLock` would hold only that `&T` cannot call a `&mut self` method.
- **Files, beyond the card's list:** `crates/w2m_vm/src/bin/w2m-vm.rs` (one stderr line a run, the measure),
  `crates/w2m_vm/src/threads.rs` (a comment naming 4c3b2b), `crates/w2m_vm/tests/all.rs` (the module).
- **Every run as held:** `cargo nextest run -E 'rdeps(w2m_vm)'`, 194 tests passed in 381 s, `qpc`, `sleeper` and the
  model-steps run among them; `cargo clippy` with the gates' flags clean after three `turn(&lock, …)` became
  `turn(lock, …)`.
- **The measure** (`w2m-vm`'s stderr, a release build, load 2.68 at the start): `sleeper`, "the turn's start: median
  125 ns, 99th percentile 916 ns, 23 turns; an exit's service: median 1709 ns"; `qpc`, "median 84 ns, 99th percentile
  1375 ns, 8 turns; an exit's service: median 2625 ns".
- **Plant 1,** `run.rs` copied aside and restored by `cp`, checked by `cmp`: `let _planted = &lock.get_mut().guest;`
  at the turn's start; `cargo check` failed, "error[E0596]: cannot borrow `*lock` as mutable, as it is behind a `&`
  reference".
- **The batch run** (`w2m-vm --game` with the harness's arguments but `--exit-ledger` and `--judge-blocks`, whose
  hashing and judging sit inside each exit's service; load 2.50 at the start): stopped where held, "ReferenceEnd: the
  log holds the reference's 168 lines and the main thread its calls", in 26.4 s; "the turn's start: median 375 ns,
  99th percentile 1458 ns, 54388 turns; an exit's service: median 1709 ns".
- **The rule decided:** the turn's start's median below the median exit service on all three runs (375 against 1709
  on the batch run), so the first form stands, the second is not built, and the design's "How the threads share
  `Guest`" says so.
- **Plant 2,** restored likewise: the turn's start's `take_word` made `let took = false;`; `qpc` exited 101, "thread
  'main' panicked at crates/w2m_vm/src/run.rs:686:9: a served exit's lock taken without the runtime word".
- **The gates:** GATES-LINE
