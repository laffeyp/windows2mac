# Sprint 223 — The ledger by defect

```yaml
---
id: 223
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 214" as annotated at Sprint 222's close (reviewer-3's Medium 1 of its review at
222's close).

- **Where it stands, by lookup (2026-10-29):**
  - The judges' ledger (`crates/w2m_vm/src/ledger.rs`, `store/case_runner/discrepancy-ledger.json`) opens one entry
    per DISCREPANCY_REPORTED, keyed by its id.
  - It holds 20 open entries, for two defects WORKING_AGREEMENT 82 names: FCOS and FSIN, the x87 results one unit
    off in the last digit.
  - 16 of the entries come from eight `case-runner-rust` sessions, one per session per class. The other 4 come from
    the one `class-judge` record, two per class: ST0 and FSW's C1.
  - Resolving FCOS would mean closing 10 entries, and the open count grows with every run.
  - The two judges name the same first case for ST0: program 88781aa2b72d for FCOS and 04fdb6d6870c for FSIN, in
    `case-runner-rust-1793213020000000000` and `class-judge-1793324063000000000`. ST0 and FSW share that program, so
    the field is part of the key.
  - They write the field differently: "FCOS: FCOS st0: …" (`case_runs`) and "FCOS: managed_vm: FCOS st0: …"
    (`class_judge`). The executor's name is not part of the field.
  - `case_runs` reports a class's most frequent difference only, ST0. FSW comes from the class judge alone.
- **The work:**
  - An entry is a defect: the unit and the difference's field (the text after the unit, "FCOS st0", the executor's
    name stripped). A later report of the same defect, by any judge, adds its record to that entry's reports.
  - The first case's program hash is not in the key: it depends on which runs a judge reads and in what order
    (`case_runs` every run on a profile, `class_judge` the 21 reference runs), and the two agree today by that order
    alone (reviewer-3's Medium 2 of its refreshed review at 222). Each report keeps its first case as data.
  - `open_on` and DISCREPANCY_RESOLVED close the entry once.
  - A gate holds the open entries to the distinct defects the records give, 4 today: FCOS's and FSIN's ST0, and
    their FSW C1.
- **Not here:** fixing FCOS and FSIN.

## prerequisites

- 222 (closed)

## context_files

- `crates/w2m_vm/src/ledger.rs`, `crates/w2m_vm/src/bin/case_runs.rs` (`--write-ledger`, `--check-ledger`,
  `open_on`), `crates/w2m_vm/src/bin/class_judge.rs`
- `store/case_runner/discrepancy-ledger.json`; WORKING_AGREEMENT 82-83; `src/w2m/differential.py` `rebuild_ledger`
  (105-139)

## signal contract

### Emits

- None new; the ledger is rebuilt from the records' DISCREPANCY_REPORTED, DISCREPANCY_ROUTED and DISCREPANCY_RESOLVED.

### Invariants

- No record changes. Every discrepancy id a record reports is named in exactly one entry.

## artifact contract

### Files modified

- `crates/w2m_vm/src/ledger.rs`, `store/case_runner/discrepancy-ledger.json`, WORKING_AGREEMENT's ledger row

### Command exit codes

- `cargo nextest run -p w2m_vm`, `case_runs --check-ledger` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** open entries, against 20 today; each entry's reports; the distinct defects the records give.
- **Planted faults:**
  1. The field dropped from `ledger.rs`'s key: the records still give 4 defects and the ledger 2, and the gate names
     the merge. (A changed record would move the records' count with the ledger's, and the gate could not fire;
     reviewer-3's Low 1.)
  2. A resolution of one report: the whole entry closes, or the gate names the entry left open.
  3. A report dropped from the ledger file: `--check-ledger` names it.

## done criteria

The judges' ledger has one entry per distinct defect, holding every report of it. A gate holds the open count to
the distinct defects the records give.

## self-review checklist

- [x] Scope names one verifiable property: one ledger entry per defect.
- [x] context_files cite the ledger, its writers and its rule.
- [x] Emits: none new, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the ledger's unit of identity.

## result
