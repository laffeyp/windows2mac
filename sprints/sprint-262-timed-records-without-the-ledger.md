# Sprint 262 — Timed records without the ledger

```yaml
---
id: 262
status: active
phase: 6
pass_kind: functional
---
```

## scope

reviewer-3's Medium 1 at 261 (Deferred, Sprint 259, as annotated at 261's close): a timed batch record keeps the exit
ledger, whose per-exit hashing grows with exits, so a comparison that changes the exit count is confounded; Sprint
257's "1.1% slower" for the host's install rests on such records, 1,091 against 150,065 `hvc 1`. This card makes a
timed record carry neither the whole-block judge nor the ledger, holds that by test, and measures card 4's time again
on records made alike.

- **Where it stands, by lookup (2026-10-30):**
  - `harness/w2m-vm-game.toml` 22 runs `w2m-vm --game … --exit-ledger build/exit_ledger/rust.tsv --judge-blocks …`;
    `scripts/batch-run-record.sh` in timed mode strips ` --judge-blocks …` (62), refuses a command that keeps it
    (77-78), and writes `block_judge = "off: …"` into the baseline; it leaves `--exit-ledger`.
  - `batch_record.rs` reads a record's baseline (426-479) and holds no timed record to the judge's or the ledger's
    absence.
  - Sprint 259, judge and ledger off: the VM's run seconds 23.06 to 23.67 with the cache against 23.90 to 24.34
    without. Sprint 257's six like records, ledgered: the rate 2.922 to 2.933 billion with the cache against 2.946 to
    2.970 without; the host's `hvc 1` 0.290 to 0.294 s against 0.047 to 0.049.
  - Census row 10's rate (Sprint 240's three records, 21.8% to 24.6% above the reference) came from ledgered records.
  - Searched for `batch-run-record.sh`, `batch_record.rs`, "ledger" in Deferred, Surfaced and
    `docs/surfaced-archive-2026-10-30.md`: Deferred (Sprint 259), this item; Surfaced, the review at 261.
- **The readers:** card 5's gate; the Decision "The translation cache's shape"; the design's card 4; census row 10.
- **The work:**
  - The script in timed mode strips ` --exit-ledger …`, writes `exit_ledger = "off: …"`, and refuses a command that
    keeps it. `batch_record.rs` holds every timed record made from this card on (its run id after
    1793415127000000000) to a baseline saying both off.
  - Six timed records from one tree, three with the cache and three without, alternating, run alone; the rate, the
    VM's run seconds, the host's `hvc 1` seconds and the VM's compile time, each pair beside the other.
  - The Decision and the design's card 4 annotated with the result whichever way it falls (the design in a work
    commit); census row 10 noted (its records ledgered, its figure the lower); the design's card 5 gate reading
    ledger-free records only.
- **Not here:** the installer.

## prerequisites

- 261 (closed)

## context_files

- `scripts/batch-run-record.sh`; `harness/w2m-vm-game.toml` 22; `crates/w2m_vm/tests/batch_record.rs` 420-480, 630-700
- `docs/design-translation-cache-2026-10-30.md` (card 4, card 5); `docs/step-4-census-2026-10-28.md` row 10

## signal contract

### Emits

- Six timed batch-run records, ledger-free: the sequence of Sprint 257's with `exit_ledger` "off" in the baseline.

### Invariants

- The gates' batch run keeps the ledger and the judge; only timed records leave them.

## artifact contract

### Files modified

- `scripts/batch-run-record.sh`, `crates/w2m_vm/tests/batch_record.rs`, `docs/design-translation-cache-2026-10-30.md`,
  `docs/step-4-census-2026-10-28.md`, `BLACKBOARD.md`; six records

### Command exit codes

- `scripts/gates.sh` returns 0; each record's `check_records` 0 violations

## observation contract

- **Expected values:** each pair's rate, "vm" seconds, host `hvc 1` seconds and compile time; the host install's net
  time against no cache, named whichever way it falls.
- **Planted faults:**
  - (1) The script's ledger strip left out: the script refuses the run.
  - (2) A timed record's baseline without `exit_ledger`: `batch_record.rs` names the record.

## done criteria

A timed record carries neither judge nor ledger, held by the script and the record test, and card 4's time is on the
record from six records made alike.

## self-review checklist

- [x] Scope names one verifiable property: timed records free of the judge and the ledger, and card 4's time on them.
- [x] context_files cite the script, the harness, the record test, the design and the census.
- [x] Emits: six records.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: what a timed record measures.

## result
