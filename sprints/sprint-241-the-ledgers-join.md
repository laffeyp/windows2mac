# Sprint 241 — The ledgers' join

```yaml
---
id: 241
status: done
phase: 6
pass_kind: functional
---
```

## scope

Plan step 4's amendment of 2026-10-25 ("one Rust ledger reads both keys and the Python's file is its derivation"),
census row 9's open half ("the ledgers' join, at the step's close"), and the Decision "Sequence after Sprint 214" as
annotated at 240.

- **Where it stands, by lookup (2026-10-30):**
  - Two ledgers. The Rust judges', `store/case_runner/discrepancy-ledger.json`, is `ledger::rebuild` over the records
    whose first frame's baseline names a `judge`, by defect, written and checked by `case_runs --write-ledger` and
    `--check-ledger` (`case_runs.rs` 127-145; the gate, `scripts/gates.sh` 231).
  - The frozen one, `store/runs/discrepancy-ledger.json` (1,295 bytes, three entries, last written 2026-10-10 by
    92d49163d), is `differential.rebuild_ledger` (105-134 at the tag frozen-python-2026-10-29) over the records whose
    baseline names a `differential`, in run-start order, keyed by discrepancy id, each entry `differential`, `unit`
    (the first divergence before its first colon), `contract_id`, `reported_in`, `first_divergence`, `repair_path`,
    `resolved_in`, `resolving_ref`; written as `json.dumps(ledger, indent=1, sort_keys=True)` and a newline (282),
    the form `ledger::text` already writes. Ten records under `store/runs/` name a `differential`: three decoder,
    three semantics, three vm-oracle, one guest-state. No program derives or checks the file since the Python left.
- **The readers, by search (2026-10-30; WORKING_AGREEMENT, a card lists its readers first):**
  - `ledger.rs` (its module comment, 4-6, names the Python file and `tests/test_differential.py`, removed in 226).
  - `case_runs.rs` 127-145 (`--write-ledger`, `--check-ledger`), `scripts/gates.sh` 231.
  - `crates/w2m_vm/tests/ledger.rs` (the judges' ledger by defect).
  - Census row 9; WORKING_AGREEMENT's ledger row (83).
  - No program reads `store/runs/discrepancy-ledger.json`.
- **The work:**
  - `ledger.rs`: one scan of the records' first frames gives both keys' records in run-start order; `rebuild` takes
    the `judge` records, and `rebuild_differential` the `differential` ones in the frozen form.
  - `case_runs --write-ledger` writes both files and `--check-ledger` holds both, naming the file that differs.
  - The module comment, census row 9 and WORKING_AGREEMENT's ledger row say so.
- **Not here:** a change to either ledger's content: the derivation is checked against the file as it stands.

## prerequisites

- 240 (closed)

## context_files

- `crates/w2m_vm/src/ledger.rs`, `crates/w2m_vm/src/bin/case_runs.rs`, `crates/w2m_vm/tests/ledger.rs`
- `src/w2m/differential.py` 95-140, 281-282 at the tag; `store/runs/discrepancy-ledger.json`
- `docs/step-4-census-2026-10-28.md` row 9; `docs/rust-port-plan-2026-10-24.md` step 4

## signal contract

### Emits

- None: a derivation of stored records, no run. The ten `differential` records are read as they stand.

### Invariants

- The judges' ledger unchanged, byte for byte; the frozen file equal to its derivation, or the difference named.

## artifact contract

### Files modified

- `crates/w2m_vm/src/ledger.rs`, `crates/w2m_vm/src/bin/case_runs.rs`, `docs/step-4-census-2026-10-28.md` (row 9),
  `WORKING_AGREEMENT.md`

### Command exit codes

- `scripts/gates.sh` returns 0; `case_runs --check-ledger` returns 0

## observation contract

- **Expected values:** `rebuild_differential` over `store/runs/` gives the three entries of the file, byte for byte;
  the judges' file unchanged.
- **Planted faults:**
  1. One character of `store/runs/discrepancy-ledger.json` changed: `--check-ledger` names that file.
  2. `rebuild_differential` reading the `judge` key: `--check-ledger` names the file.
  3. A unit test's record whose report a later record resolves, the resolution dropped: the unit test names the entry.

## done criteria

One Rust ledger module reads both keys, and the frozen file is its derivation, checked by the gate.

## self-review checklist

- [x] Scope names one verifiable property: the frozen ledger file derived and checked by Rust.
- [x] context_files cite the frozen code, the file, the Rust ledger and the census row.
- [x] Emits: none, stated with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the ledgers' join.

## result

- **The join:** `ledger::started` reads every record's first frame once and gives each record whose baseline names a
  `judge` or a `differential`, in run-start order, then by root, as the frozen `sorted(started)` orders them.
  `rebuild_judged` is the judges' ledger by defect, unchanged; `rebuild` is it over `started`.
  `rebuild_differential` is `differential.rebuild_ledger`: a report keyed by its id, overwriting one of the same id as
  the frozen dict assignment does, a route or resolution read only for an id already reported.
- **The derivation:** over `store/runs/`, `rebuild_differential` gives the frozen file's 3 entries from the 10
  `differential` records, byte for byte: `case_runs --check-ledger` returned 0 on the first build, and
  `--write-ledger` rewrote both files with `cmp` finding each equal to its copy ("4 discrepancies in the judges'
  ledger, 3 in the differentials'").
- **The gate:** `case_runs --check-ledger` holds both files, the differentials' at `<runs>/discrepancy-ledger.json`
  beside its records, as `differential.LEDGER` (44) put it; no script sets `W2M_RUNS`, so the gates read the store's.
- **The test:** `a_differentials_report_is_resolved_as_the_frozen_ledger_resolves_it`: a report and its route in one
  record, a judge's record between, the resolution in a later record; the entry's fields and the file's form.
- **Records:** census row 9 met; WORKING_AGREEMENT's ledger row (83) names the derivation; `ledger.rs`' module comment
  no longer names `tests/test_differential.py`, removed in 226.
- **Departures:** none in files.
- **The gates:** `scripts/gates.sh` exit 0 in 468 s on ec0ba8905, "rust=crates python=0 samples=0", 107 tests of
  `w2m_vm` and its dependants, `case_runs --check-ledger` among the gates.
- **Plant 1** (`"repair_path": "contract_revision"` to `contract_revisioN` in `store/runs/discrepancy-ledger.json`):
  "store/runs/discrepancy-ledger.json is not the ledger the records give; `case_runs --write-ledger` rebuilds it",
  exit 1. The `sed` changed the string in all three entries, where the card said one character.
- **Plant 2** (`rebuild_differential` reading `s.judge` for `s.differential`): the same message, naming the
  differentials' file, exit 1.
- **Plant 3** (`rebuild_differential`'s resolution, line 246, dropped): "the later record's resolution closes the
  report", `resolved_in` null, in `a_differentials_report_is_resolved_as_the_frozen_ledger_resolves_it`.
- Each plant's `git diff` showed it before its verdict; each was restored by `cp`, and `cmp` found the files equal
  to their copies.
