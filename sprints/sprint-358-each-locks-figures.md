# Sprint 358 — Each leaf lock's takings, hold and wait on the record

```yaml
---
id: 358
status: active
phase: 6
pass_kind: functional
---
```

## scope

Vocabulary 0.14 (Sprint 357) gave a record the words for each leaf lock's takings, hold and wait. No code writes them
yet. Change 1's four leaf locks carry the waits, releases, creates and heap calls that left the runtime word in
Sprints 343, 345, 355 and 356, and G6 asks for "each lock's hold and wait". The runtime word's own wait figures on two
vCPUs are a lower bound: one shared wait-start word is overwritten by a second waiter (Sprint 356, 4,349 of 16,384
waits before its change and 146 after starting past their taking; the Decision "Where the timing area's wait start per
vCPU goes" puts the fix here). This card writes every leaf lock's figures, and fixes that.

- **The leaf locks' timing.** The four takes and releases are `hold`/`let_go` (heap), `hold_slots`/`let_slots_go`,
  `hold_handles`/`let_handles_go` and `hold_sync`/`let_sync_go`.
  - When timing is on, each taking records its wait start, taken and released times by `CNTVCT_EL0`, as `process.c`
    25-27 reads it. The record goes to a per-lock area the host reads at the stop.
  - Each lock counts its takings by an atomic add, even past its area's end.
  - **No shared word in flight.** A taker keeps its own wait start in a local, not in a shared header, and writes its
    whole entry while it holds the lock. That is the shared-header fault this card fixes in the runtime word's area.
  - When timing is off, the lock's built code is unchanged. Show it by disassembly of one take, or hold the cost to a
    stated bound by measurement.
- **Switching timing on.** The way `--runtime-word` turns on the runtime word's timing (`Options::time_runtime_word`,
  `crates/w2m_vm/src/bin/w2m-vm.rs` 368-415) also turns on the leaf locks', through a word the C reads. The batch run,
  the family runs and every test that does not ask for timing must be unchanged.
- **The runtime word's wait start per vCPU.** The timing area (`crates/w2m_vm/src/monitor_code.rs` 227-241) keeps the
  wait start per vCPU, or in the taker's own register, so no other vCPU writes it. Two-vCPU waits then run backward 0
  times in 16,384, and `run_measurements` refuses any that do.
- **The record.** `run_measurements` writes on two vCPUs:
  - for each leaf lock: MEASUREMENT_RECORDED `lock_takings`, `lock_hold_ns_p50`, `lock_hold_ns_p99`,
    `lock_wait_ns_p50` and `lock_wait_ns_p99`, each with its `lock`;
  - `runtime_word_takings` beside the runtime word's figures, now trusted.

  Its rows are the guests that drive each lock: `semaphore_units` (sync objects), `handle_values` (handles), and the
  heap's and slots' two-vCPU guests of Sprints 343 and 345. The record passes `check_records`, E16 included, and is
  committed under `store/runs/`.
- **The judges.**
  - **Takings are counted, not guessed.** For each guest, a lower bound on each lock's takings, from the guest's own
    counts of the calls it made (`handle_values` claims 104,064 values, each create a taking), held by the record's
    `lock_takings`.
  - **Waits are real.** On a two-vCPU guest whose threads contend for one lock, its p99 wait is above 0. On one vCPU
    every lock's waits are 0 (no other taker), and the record shows it.
  - **The runtime word's waits:** 0 of 16,384 run backward on `semaphore_units` on two vCPUs.
- **Not here:** the scoped guard (the next card); any lock's change of class.

## prerequisites

- 357 (closed)

## context_files

- `signals/0.14.json`: MEASUREMENT_RECORDED, the six new metrics, `lock`, E16.
- `tools/guest_runtime/heap.c`, `thread_storage.c` 20-90, `sync_objects.c` (each lock's take and release),
  `process.c` 1-60 (the counter), `runtime.h`.
- `crates/w2m_vm/src/monitor_code.rs` 200-320 (the runtime word's timing area), `crates/w2m_vm/src/bin/w2m-vm.rs`
  360-430 (`--runtime-word`), and `crates/w2m_vm/src/bin/run_measurements.rs` in full, with `harness/w2m-vm-measurements.toml`.
- `crates/w2m_vm/tests/run_loop.rs`: the tests of `semaphore_units`, `handle_values`, the heap's and the slots'
  two-vCPU guests.
- `crates/guest_runtime/tests/python_image.rs` (`DEPARTURES`).
- `grep -n "Where the timing area's wait start per vCPU goes\|A card that moves a service class between locks"
  BLACKBOARD.md`: the two Decisions.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands", and 300, "Signals that bind".

## signal contract

### Emits

- MEASUREMENT_RECORDED on the committed record, with `vcpus` 2:
  - for each of `heap`, `slots`, `handles` and `sync_objects` as `lock`: `lock_takings`, `lock_hold_ns_p50`,
    `lock_hold_ns_p99`, `lock_wait_ns_p50` and `lock_wait_ns_p99`;
  - `runtime_word_takings`, beside `runtime_word_hold_ns_*` and `runtime_word_wait_ns_*`.

### Invariants

- With timing off, the batch run, the family runs on one vCPU and two, the exit ledger and every run test pass
  unchanged.
- `check_records` over every record from 2026-10-17 prints no new violation.

## artifact contract

### Files modified

- The four locks' take and release in `heap.c`, `thread_storage.c` and `sync_objects.c`, with their timing areas; the
  timing area in `monitor_code.rs`; `w2m-vm.rs`' switch; `run_measurements.rs` and its baseline; the new record under
  `store/runs/`; `DEPARTURES`; and the tests. One concept: the leaf locks' timing, which these files share.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The record's figures for each lock on two vCPUs, quoted.
  - Each guest's lower bound on takings against `lock_takings`.
  - The contended p99 wait above 0.
  - One vCPU's waits at 0.
  - The runtime word's backward waits, 0 of 16,384.
  - With timing off, one take's built code, or the cost bound.
- **Planted faults:**
  - (1) One lock's takings counter not added to. Its `lock_takings` falls below the guest's bound, and the judge fails.
  - (2) A taker's wait start taken from a shared word again. The waits run backward on two vCPUs, and
    `run_measurements` refuses the run.
  - (3) The runtime word's per-vCPU wait start put back to the shared header. Backward waits above 0, refused.
  - (4) A lock's entry written after its release. A hold reads 0 or negative, refused.
  - (5) Timing left on when not asked for. A test with timing off fails, or the cost bound is broken.

## done criteria

A two-vCPU record carries every leaf lock's takings, hold and wait. Its takings meet each guest's own counts, its
contended waits are above 0, and the runtime word's waits are trusted. With timing off, nothing changes.

## self-review checklist

- [x] Scope names one verifiable property: each leaf lock's figures on the record, the runtime word's waits trusted.
- [x] context_files cite the vocabulary, the locks, the timing area, the switch, `run_measurements` and the guests.
- [x] Emits: MEASUREMENT_RECORDED with the five `lock_*` metrics for each lock, and `runtime_word_takings`.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the figures, the bounds, the waits; numbered plants.
- [x] One concept: the leaf locks' timing on the record.

## result

GATES-LINE
