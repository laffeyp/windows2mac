# Sprint 262 — Timed records without the ledger

```yaml
---
id: 262
status: done
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

- **The rule** (ff1926fdc): `scripts/batch-run-record.sh` in timed mode strips ` --exit-ledger …` as it strips
  ` --judge-blocks …`, writes `exit_ledger = "off: …"` into the baseline, and refuses a command that keeps either;
  `batch_record.rs`' `unmeasured_problems` holds every timed record named after 1793415127000000000 to a baseline with
  both off, with its unit test `a_timed_record_with_its_judge_or_ledger_on_is_named`.
- **The records:** six from ff1926fdc, alternating, run alone, load at start 2.71 to 4.08 (the machine busier than at
  257): without the cache `store/runs/w2m-vm-game-1793415256000000000`, `-1793415309000000000`,
  `-1793415362000000000`; with it `-1793415283000000000`, `-1793415336000000000`, `-1793415389000000000`; each
  conforms, `check_records` 0 violations; each baseline says the judge and the ledger off.

  | | without the cache | with it |
  |---|---|---|
  | rate, instructions a second | 3.065, 3.117, 3.079 billion | 3.182, 3.185, 3.174 billion |
  | "vm" run seconds | 23.60, 23.41, 23.71 | 22.74, 22.72, 22.79 |
  | host `hvc 1` | 0.042 s | 0.206 to 0.212 s |
  | the VM's compile time | 1,276 to 1,278 ms | 589 to 596 ms |

  The host's install is 3.0% faster on the means, the ranges apart (3.117 against 3.174): the VM gains about 0.82 s
  and the host spends about 0.17 s more. Sprint 257's "1.1% slower" came from records that wrote the ledger, whose
  hashing at each exit fell on the cached run's 150,065 against 1,091.
- **The record annotated:** the design's card 4 and card 5's gate (6319fbdfc, 5e59fccf5), the Decision, census row 10
  (its Sprint 240 records ledgered, its figure the lower).
- **The gates:** `scripts/gates.sh` exit 0 in 596 s on 5e59fccf5, copied from `store/gates/passes.tsv`; the timing stage 651 ns an exit at load 3.93.
- **KIT_DIARY #109:** a confounder removed, its neighbour left: the fix of a timed run lists every piece of host work done per exit.
- **Plants,** run on 5e59fccf5, each restored by `cp` and checked by `cmp`, the planted record removed.
- **Plant 1,** `scripts/batch-run-record.sh` refused the run, "batch-run-record.sh: the timed run's command still
  writes the exit ledger", exit 2: the ledger's strip replaced by a statement that does nothing. A first try put `0`
  there, which awk refused as a statement ("awk: illegal statement"), a fault of the plant, not a catch.
- **Plant 2,** `every_stored_rust_batch_record_holds_its_rate` failed, "a timed record whose baseline's exit_ledger is
  null, not off": the baseline's `exit_ledger` line left out and one timed record made with it. A first run of the
  test was filtered by a wrong name and ran nothing; the record was made again and the test run by its name.
