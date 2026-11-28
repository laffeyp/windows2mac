# Sprint 382 — The Rust host's batch stream stored as the reference past the frozen host's cut

```yaml
---
id: 382
status: active
phase: 6
pass_kind: functional
---
```

## scope

Sprint 381's profile re-take moved the game's batch run from row 29, so the frozen Python host's stream
(`store/frontier-calls/frontier-643a6283510c-1792623423346201000.tsv`) holds row for row only to row 28 of 10,505
(`LAST_HELD_ROW`, `crates/w2m_vm/tests/batch_run.rs`), and Windows' judge, G3, alone holds the rest. G3 tolerates
scheduling; the frozen stream held the exact order on the one-vCPU instruction clock, the project's determinism
anchor. The Decision "The runner's strings re-taken, and the frozen host's reference after them" (Sprint 381) fires
its revisit now: the Rust host's own stream becomes the reference past the cut.

- **The stream.** The one-vCPU batch run's call stream (`build/batch_run/first-stop.calls.tsv`, 10,515 rows at
  Sprint 381's close, two runs alike) made at the tree of 758f9f900 and stored under `store/batch_run/` with a header
  naming the commit, the run and the date; the frozen stream kept unchanged as the port's history.
- **The test.** The batch-run test holds the run's stream to the frozen stream to `LAST_HELD_ROW`, and to the stored
  Rust stream row for row in every row, cell for cell; a difference fails naming the row and the cell. The two-vCPU run
  stays with G3 alone (its order is the wall clock's).
- **The rule for later cards.** A card that changes the stream regenerates the stored file from its closing run, and
  its result states the rows and cells that changed and why (measured, as Sprint 379 measured); the file's header
  gains one line per such card, as `store/exit_ledger/differences-2026-10-29.tsv`'s does.
- **Not here:** any guest answer. No API id is added. No Windows run.

## prerequisites

- 381 (closed)

## context_files

- `crates/w2m_vm/tests/batch_run.rs` (`LAST_HELD_ROW`, the stream test, the provenance check `built_from`).
- `store/batch_run/` (its files' headers), `store/exit_ledger/differences-2026-10-29.tsv` (its header's form).
- `scripts/gates.sh` (the batch-run step that writes `build/batch_run/`).
- `grep -n "The runner's strings re-taken, and the frozen host's reference after them\|where the frozen stream's cut falls" BLACKBOARD.md`.
- `WORKING_AGREEMENT.md` lines 137-266 and 301.

## signal contract

### Emits

- None declared: the change is to a test and a stored reference.

### Invariants

- Every run as held; G3 conforms on one and two vCPUs; the frozen stream byte-equal to HEAD's;
  `crates/guest_runtime/src/api_ids.txt` unchanged; no Windows run.

## artifact contract

### Files created

- the stored Rust stream under `store/batch_run/`, its name dated.

### Files modified

- `crates/w2m_vm/tests/batch_run.rs`, with others only where the class requires; file count recorded.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The stored stream's row count and sha256, the commit and run it came from, and that two runs at that tree give it
    byte for byte.
  - The test's held share: rows 0-28 against the frozen stream, all 10,515 against the stored stream.
- **Planted faults:**
  - (1) One cell past row 28 changed in the run's stream (in `build/`, data): the test fails naming the row and cell.
  - (2) One row removed from the stored stream: the test fails naming the first row that differs.

## done criteria

The one-vCPU batch run is held row for row and cell for cell to a stored stream of the Rust host's own run in every
row, and to the frozen Python host's to the cut.

## self-review checklist

- [x] Scope names one verifiable property: the whole one-vCPU stream held row for row again.
- [x] context_files cite the test, the store, the gates step and the Decisions.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the stream's count and digest, the held share; numbered plants.
- [x] One concept.

## result

GATES-LINE
