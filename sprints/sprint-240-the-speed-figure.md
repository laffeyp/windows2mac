# Sprint 240 — The speed figure

```yaml
---
id: 240
status: active
phase: 6
pass_kind: functional
---
```

## scope

Plan step 4's "Moves: instructions a second on one vCPU", against the reference run's 2,354,295,320 (amended
2026-10-28), census row 10, and BLACKBOARD Deferred (Sprint 221) item 2 as annotated at 224: the Rust batch run
measured in the configuration the stored record ran, read beside the stored figure.

- **Where it stands, by lookup (2026-10-30):**
  - The Rust batch run already writes `instructions_per_second` (`record.rs` 229-240), by the frozen formula:
    instructions over the VM's seconds and the host's (`run_frontier.py` 164-165 at the tag). The gates' run on
    91177ad59 gave 2,856,098,221.9 over 80,306,813,574 instructions, in `build/batch_run/`, which no commit keeps.
    Census row 10 still reads "not measured".
  - The configuration: the stored run's measurements are all `artifact_state` "warm", which the frozen host writes
    only when its translation cache had hits (`run_frontier.py` 383: `"warm" if vm.tcache_hits else "cold"`). The Rust
    host keeps no translation cache (`compile.rs` 4, 70), yet `record.rs` 325 writes `ArtifactState::Warm` for every
    measurement, and `batch_record.rs` 60-67 holds `artifact_state` to the reference's among "the measurement fields
    both hosts set alike". The label is false.
  - The record holds `run_seconds` "vm" alone, 24.71 s on that run, while the rate divides by the VM's and the host's
    seconds, 28.12 s; the host's 3.41 s are in `Execution::host_seconds` and on no row, so no reader can check the rate.
    The frozen host wrote its host seconds by exit kind and `measurement_problems` (`run_frontier.py` 189-200) failed
    a run whose rate times its seconds was not its instructions; no Rust test checks it.
  - The configuration cannot be matched on Rust, which has no cache; a cache only takes host compile time away, so
    the Rust figure without one is a lower bound on the matched figure. The comparison stands one way: if the Rust
    figure is above the stored one, the matched one is too.
- **The readers of the label, by search (2026-10-30; WORKING_AGREEMENT, a card lists its readers first):**
  - `record.rs` 250 (the batch run's measurements) and `run_measurements.rs` 127, 136, 214, 237 (step 3's), each
    through `measurement_line`.
  - `batch_record.rs` 60-67 and 172-179: `CONVENTIONS` holds `artifact_state` to the reference's.
  - Stored records: the step-3 measurement records under `store/runs/` say "warm"; they stay as written.
- **The work:**
  - `measurement_line` writes `ArtifactState::Cold`, by the frozen host's rule, the Rust host having no cache.
  - The record gains `run_seconds` "host", `Execution::host_seconds`; the Rust host keeps one total, not one per exit
    kind.
  - `batch_record.rs`: `artifact_state` leaves `CONVENTIONS`; the record's is "cold", held as the Rust host's own. The
    rate check carried from `measurement_problems`, over the gates' record and every committed record whose baseline
    names the host `w2m-vm`.
  - The batch run measured three times in a row, nothing else running, each by `harness/w2m-vm-game.toml` into
    `store/runs/`, the load average read before each.
  - Census row 10 from the three records, beside 2,354,295,320 and its configuration.
- **Not here:** the ledgers' join, card 241 (the Decision "Sequence after Sprint 214" grouped it with this card;
  it is its own concept). The plan is the user's and is not edited.

## prerequisites

- 239 (closed)

## context_files

- `crates/w2m_vm/src/record.rs`, `crates/w2m_vm/src/bin/run_measurements.rs`, `crates/w2m_vm/tests/batch_record.rs`
- `scripts/run_frontier.py` 150-200, 380-392 at the tag; `store/runs/frontier-643a6283510c-1792623423346201000`
- `docs/step-4-census-2026-10-28.md` row 10; `docs/rust-port-plan-2026-10-24.md` step 4

## signal contract

### Emits

- Three batch-run records, each with MEASUREMENT_RECORDED `instructions_per_second` (variant "process",
  `artifact_state` "cold") and the run's other measurements.

### Invariants

- The batch run conforms on each record (COMPARISON_COMPLETED "conforms", 168 lines); its counts as
  `batch_record.rs` holds them.

## artifact contract

### Files modified

- `crates/w2m_vm/src/record.rs`, `crates/w2m_vm/tests/batch_record.rs`, `docs/step-4-census-2026-10-28.md` (row 10);
  three records under `store/runs/`

### Command exit codes

- `scripts/gates.sh` returns 0; each record's `check_records` 0 violations

## observation contract

- **Expected values:** each record's `instructions_per_second` against 2,354,295,320; `artifact_state` "cold" on
  every Rust measurement; each rate times its seconds equal to its instructions.
- **Planted faults:**
  1. `measurement_line` writing `Warm` again: `batch_record.rs` names the measurement.
  2. A record's `instructions_per_second` changed by one part in a thousand: the rate check names it.
  3. A record's COMPARISON_COMPLETED verdict changed: `check_records` or the batch record test names it.

## done criteria

Census row 10 holds the Rust host's instructions a second on one vCPU from committed records, labelled by the
configuration it ran.

## self-review checklist

- [x] Scope names one verifiable property: the batch run's rate on the record, truly labelled.
- [x] context_files cite the formula, the label's rule, the reference run and the census row.
- [x] Emits: three records.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the speed figure.

## result
