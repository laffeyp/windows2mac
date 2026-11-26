# Sprint 359 — The runtime C's locks released by a scoped guard

```yaml
---
id: 359
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "The runtime C's leaf locks released by a scoped guard" (2026-11-26, reviewer-4's twenty-fifth review,
Low 1). The runtime C takes its locks in hand-written pairs, and each function's early returns repeat the release:
- `hold`/`let_go` (heap.c);
- `hold_slots`/`let_slots_go` (thread_storage.c, 14 releases at 304470288);
- `hold_handles`/`let_handles_go` and `hold_sync`/`let_sync_go` (sync_objects.c, 49 releases between them);
- since Sprint 355, `w2m_take_runtime_word`/`w2m_let_runtime_word_go` (runtime.c), which lets the word go only when
  this call took it.

Correctness rests on every future edit remembering the release. Sprint 338 removed that pattern from the host with
`RuntimeWordGuard` and its source test (`crates/w2m_vm/src/run.rs` 2288-2330). This card gives the C the same form.

- **The guard.** Clang builds the runtime (`crates/guest_runtime/src/lib.rs` 152), and its `cleanup` attribute calls a
  function when a variable leaves scope. Look its behavior up in Clang's attribute reference, and quote it.
  - Write a scoped form per lock, for instance `HELD(handles)`: it declares a guard whose cleanup lets the lock go,
    and every take in these five files goes through it.
  - The runtime word's guard lets it go only when its take took it (`w2m_take_runtime_word`'s answer).
  - Under `W2M_TIME_LOCKS` (Sprint 358) the guard's release is the timed one, so the timing's entries are unchanged.
- **The one exception stays explicit.** `w2m_unimplemented` never returns, and cleanup runs at scope exit, not before
  a call. So a function that calls it, or calls `w2m_logged` while it would otherwise hold a lock, still releases first
  by hand, as today. Do it through a named form that ends the guard early (for instance `LET_GO_EARLY(handles)`), not a
  bare release, so the source test can tell the two apart. The macro's comment says why.
- **Unchanged behavior.**
  - The batch run, the family runs on one vCPU and two, the exit ledger, every run test, and the two-vCPU judges of
    Sprints 343-356 pass unchanged: `semaphore_units`, `handle_values`, `lost_wakeup`, `mutex_abandoned`,
    `heap_blocks`, `thread_slots_race`.
  - The record of Sprint 358 can be remade with the same figures, within a run's spread.
- **The judges.**
  - **A source test**, in the form of `the_word_is_released_only_by_its_guards_drop`: in the five files, no release
    outside a guard's cleanup or a named early release. Each early release sits on a path that then calls
    `w2m_unimplemented` or `w2m_logged`, or its line is named with its reason.
  - **The built image**, by disassembly of at least three functions with early returns (for instance
    `w2m_object_close`, TlsFree and `wait_one`): a release store (`stlr wzr`) on each return path while the lock is
    held, as today.
- **Not here:** any change of lock class or of what a lock guards.

## prerequisites

- 358 (closed)

## context_files

- `tools/guest_runtime/heap.c`, `thread_storage.c`, `sync_objects.c` and `runtime.c`: in full. Also `runtime.h`
  (`w2m_lock_take` and `w2m_lock_let_go` under `W2M_TIME_LOCKS`).
- `crates/w2m_vm/src/run.rs` 790-830 (`RuntimeWordGuard`) and 2280-2335 (its source test).
- `crates/guest_runtime/src/lib.rs` 140-170 (the runtime's build), and `crates/guest_runtime/tests/python_image.rs`
  (`DEPARTURES`, the frozen key).
- `grep -n "The runtime C's leaf locks released by a scoped guard" BLACKBOARD.md`: the Decision.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None: no emit site changes. The runtime's behavior and its records are unchanged, and Sprint 358's
  `run_measurements` record, remade on this tree, carries the same metrics. That remade record is the observation's,
  not a new emit.

### Invariants

- Every existing run, record and judge passes unchanged on one vCPU and two.

## artifact contract

### Files modified

- `tools/guest_runtime/heap.c`, `thread_storage.c`, `sync_objects.c`, `runtime.c`, `runtime.h`, the source test, and
  `DEPARTURES`. One concept: the locks' scoped form.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The count of hand releases before (by `grep`) and after: after, only the named early releases, each listed with
    its reason.
  - The disassembly of three functions' return paths.
  - Clang's `cleanup` quoted from its reference.
  - The six two-vCPU judges' runs.
- **Planted faults:**
  - (1) A bare release written beside a guard in one function. The source test names its line.
  - (2) A guard removed from one take in `sync_objects.c`, its releases removed with it. The source test fails, a
    take with no guard; and on two vCPUs `semaphore_units` or `handle_values` fails, the lock left held.
  - (3) An early release removed before a `w2m_unimplemented` call. The source test fails on the path that calls it.
  - (4) The runtime word's guard releasing when its take did not take it. A run test fails: a traced bridge's word
    let go under it, or the record's takings disagree.

## done criteria

Every take of the runtime C's five locks goes through a scoped guard, every release outside one is a named early
release before a call that must not run under the lock, and the built image releases on every return path as before,
with every existing judge unchanged.

## self-review checklist

- [x] Scope names one verifiable property: no hand-paired release left outside a guard or a named early release.
- [x] context_files cite the five files, the host's guard and its test, the build and the Decision.
- [x] Emits: none, with its reason; no emit site changes.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the counts, the disassembly, the reference, the judges; numbered plants.
- [x] One concept: the locks' scoped form.

## result

GATES-LINE
