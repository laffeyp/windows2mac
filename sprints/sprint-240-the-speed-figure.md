# Sprint 240 — The speed figure

```yaml
---
id: 240
status: done
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
  - The machine's state on the record (reviewer-3's Medium 1 of its review at 240, after this card was committed: the
    reference's baseline holds `load_at_start`, load 2.58 with five other w2m VMs, none busy, and three
    `load_average` measurements; the Rust record held none; the Drift watchlist's "Absolute timings drift between
    runs" asks for the QoS class and the cores beside a cross-run comparison). `scripts/batch-run-record.sh` writes
    `load_at_start` (the frozen `host_load`'s three parts, as one string: a run file's baseline holds text), `qos`
    (`w2m-vm --qos-class`) and `cores` (no process on Apple silicon can pin one: `THREAD_AFFINITY_POLICY` returns
    `KERN_NOT_SUPPORTED`) into a transient run file's baseline; the gates use it. `w2m-vm` writes the three
    `load_average` rows at the run's end (`w2m_vm::host_state`), and `batch_record.rs` holds both.
  - The batch run measured three times in a row by the script into `store/runs/`.
  - Census row 10 from the three records, each with its conditions, beside 2,354,295,320 and the reference's, the
    claim worded to the conditions; WORKING_AGREEMENT says the Rust records before 240 say "warm" falsely (Low 1);
    the watchlist entry annotated.
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

- **The figure:** three records from 15d04a031 by `scripts/batch-run-record.sh`, each conforming, `check_records` 0
  violations, 80,306,813,574 instructions each:

  | record | load at start (1, 5, 15 min) | busy | instructions a second |
  |---|---|---|---|
  | `store/runs/w2m-vm-game-1793364171000000000` | 1.82 2.78 3.08 | none | 2,932,723,144 |
  | `store/runs/w2m-vm-game-1793364267000000000` | 2.58 2.73 3.02 | python 100% | 2,868,493,560 |
  | `store/runs/w2m-vm-game-1793364362000000000` | 2.51 2.68 2.97 | python 100% | 2,875,199,872 |

  2.2% apart; 21.8% to 24.6% above the reference's 2,354,295,320, which ran at load 2.58, 3.29, 3.30 beside five
  idle w2m VMs, warm. No other w2m process ran beside these; the busy `python` process was not w2m's. Every Rust
  measurement is cold, QoS user-interactive, inherited, no core pinned. Census row 10 is met and says so in these
  terms.
- **The label:** `measurement_line` writes `cold`; WORKING_AGREEMENT's measurement row says the Rust records before
  this sprint say `warm` falsely (reviewer-3's Low 1 of its review at 240).
- **The record's arithmetic:** `run_seconds` "host" joins "vm", so the rate checks against the record: each record's
  rate times its seconds is its instructions to one part in a million (`rate_problems`, carried from
  `measurement_problems`).
- **The conditions (reviewer-3's Medium 1, after the card's commit):** `scripts/batch-run-record.sh` writes
  `load_at_start`, `qos` and `cores` into a transient run file's baseline, `harness/.w2m-vm-game-<pid>.toml` (ignored,
  removed on exit), since substrate runs Producers from the run file's directory; the gates use it. `w2m-vm` writes
  `load_average` at the end (`w2m_vm::host_state`, `getloadavg` and `qos_class_self`, declared from the SDK's
  `_stdlib.h` 328 and `sys/qos.h` 130-172). A first trial wrote `load_at_start` as an inline table, which substrate
  stored as Python's repr of a dict; it is one string now. A second trial's `awk` failed on a ternary across lines
  (BSD awk); the empty lists are set in the shell. Both trials wrote under `build/`, never committed.
- **Runs set aside:** three runs before the review lacked the conditions; two finished, 2,858,003,072 and
  2,849,235,541 at load 1.61 and 3.48, and the third was stopped. Their records, never committed, were removed.
- **Breaches:** a lookup command held `uv run python -c 1`, an empty Python program, against the Decision "No Python
  runs", as Sprint 238's `python3 -c 1` was. A `printf >>` added three lines to `.gitignore`; `git checkout` undid it
  and the Edit tool made the change. The card's commit ran a `sed` that replaced a line with itself.
- **A test not run in time:** after 506c03702 changed the record's rows, `record.rs`' unit test, which counts them,
  was not run; it failed when run before the gates, and was fixed in 15d04a031.
- **Departures, files:** `crates/w2m_vm/src/host_state.rs` new, `lib.rs`, `bin/w2m-vm.rs`, `scripts/batch-run-record.sh`
  new, `scripts/gates.sh`, `harness/w2m-vm-game.toml`, `.gitignore`, `WORKING_AGREEMENT.md`, the Drift watchlist.
- **The gates:** `scripts/gates.sh` exit 0 in 658 s on 13a9b584a, "rust=all python=0 samples=1", 247 tests, the
  three records replayed and checked ("records: 3 new").
- **Plant 1** (`measurement_line`'s `ArtifactState::Cold` back to `Warm`): `record.rs`' unit test failed, "left:
  String("warm") right: "cold"", at the exit row's `artifact_state`; the card had named `batch_record.rs`, which reads
  stored records, not the code.
- **Plant 2** (the second record's `instructions_per_second` 2,868,493,559.68 to 2,871,362,053.24, one part in a
  thousand): "instructions_per_second 2871362053.242637 over 27.996163109 s is 80387120387.57402 instructions, where
  80306813574 retired".
- **Plant 3** (the second record's COMPARISON_COMPLETED verdict to "diverges"): "COMPARISON_COMPLETED verdict
  "diverges"", from `every_stored_rust_batch_record_holds_its_rate`.
- Each plant's `git diff --stat` showed its one line changed before its test ran; each was restored by `cp`, and
  `cmp` found the files equal to their copies.
