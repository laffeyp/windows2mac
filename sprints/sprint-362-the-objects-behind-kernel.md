# Sprint 362 — The objects that cross processes behind their own lock, `kernel`

```yaml
---
id: 362
status: active
phase: 6
pass_kind: functional
---
```

## scope

Card 4f1 of `docs/design-a-process-that-starts-another-2026-11-26.md` ("The cards that follow", item 1). The design is
the spec. `kernel.rs`' `Objects` holds what processes share: named events, pipes, and a record per process. Today it
lives in the first process's `Guest` (`served.kernel`), and the turn form's `Supervisor` with it. `turn` moves both out
of the `Guest` and back in around each hand-over (`run.rs` 948-949, 969-972 at ae82b92ba). This card moves them into one
mutex beside `GuestLock`, the host design's Change 4 `kernel` lock: "`kernel`, for processes, cross-process names,
handles and pipes, one request at a time" (`docs/design-rust-host-2026-10-24.md` 289).

- **The lock.** A KERNEL 33 service takes `kernel` under its runtime word and the run's lock, as a leaf held for one
  request. `turn` no longer moves the objects out. A spawn is made under `kernel`, since the children's table is under
  it too.
- **The guard.** `kernel`'s guard asserts that its taker holds a runtime word, or is the server (none exists until card
  4f2), in release builds too, as `GuestLock::serve` asserts the word.
- **The class first.** List every reader and writer of `Objects` and the `Supervisor` today, in `host_request.rs`,
  `run.rs`, `turns.rs`, `kernel.rs` and the exit ledger, one line each with file:line and what it holds. Each takes
  `kernel`, or the list says why it need not. Report each defect found to the Supervisor at once, with its evidence.
- **The lock order**, as the design states it: in a vCPU thread, the runtime word, the run's lock, then `kernel`, a
  leaf. Nothing holding `kernel` waits for a word or the run's lock.
- **One vCPU and every run unchanged.** The turns stay, and `MACHINE_ON_ONE` stays. On two vCPUs the six held runs are
  still refused by name.
- **Not here:** the request server (card 4f2); any run of processes at once (4f3).

## prerequisites

- 361 (closed)

## context_files

- `docs/design-a-process-that-starts-another-2026-11-26.md`: in full.
- `crates/w2m_vm/src/kernel.rs`, `host_request.rs` 130-250 and 500-640, `turns.rs`, and `run.rs` 790-1000, 1500-1540,
  1800-1830 and 1930-1965.
- `crates/w2m_vm/src/guest.rs` (`GuestLock`, `served`) and `crates/w2m_vm/src/exit_ledger.rs` (any reader of the
  objects).
- `crates/w2m_vm/tests/family_runs.rs`, the `twoproc` guests' tests, and `store/family_runs/two-vcpus-refused-2026-11-26.tsv`.
- `grep -n "The form of card 4f on several vCPUs" BLACKBOARD.md`: the Decision.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None: where the shared objects live and which lock covers them; no emit site changes. Every run's records are
  unchanged.

### Invariants

- The batch run conforming, the 273 family runs on one vCPU and two as held (the six still refused by name), the
  `twoproc` guests and the exit ledger, all unchanged.

## artifact contract

### Files modified

- `crates/w2m_vm/src/host_request.rs`, `run.rs` and the field in `guest.rs` (the design's three files for 4f1), with
  `turns.rs` or `kernel.rs` only where the class list requires. One concept: the shared objects behind `kernel`.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The class list, one line for each reader and writer.
  - Every run as held on one vCPU and two.
  - The guard's assertion's text.
- **Planted faults:**
  - (1) A KERNEL 33 service that takes `kernel` without its runtime word. The guard's assertion names it, in a test
    that drives that service.
  - (2) `turn` moving the objects out again beside the lock, as today. A test that holds `kernel` across a hand-over
    fails, or the `twoproc` guest's run differs.

## done criteria

The objects that cross processes live behind one leaf lock, `kernel`, taken under the runtime word and the run's lock.
`turn` no longer moves them, and every run is unchanged on one vCPU and two.

## self-review checklist

- [x] Scope names one verifiable property: the shared objects behind `kernel`, every run unchanged.
- [x] context_files cite the design, the code that moves the objects, the guard's precedent and the judges.
- [x] Emits: none, with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the class list, the runs, the assertion; numbered plants.
- [x] One concept: where the shared objects live.

## result

GATES-LINE
