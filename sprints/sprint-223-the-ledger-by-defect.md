# Sprint 223 — The ledger by defect

```yaml
---
id: 223
status: done
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

- **The ledger by defect:** `ledger::rebuild` keys an entry by `defect`, the unit and the difference's field. The
  field is the first colon-ended segment of the first divergence that begins with the unit and a space, so
  `case_runs`' "FCOS: FCOS st0: …" and `class_judge`'s "FCOS: managed_vm: FCOS st0: …" are both "FCOS st0". Each
  entry holds its reports (id, judge, contract, record, first divergence, repair path, resolution). A resolution
  resolves the report it names, and the entry closes when every report in it is resolved. A later report reopens it.
  `open_on` returns a judge's unresolved report ids on a unit, which its next acceptance resolves.
  - The first form closed the entry on any one resolution. reviewer-3's Medium 1 of its review at 223 found that one
    judge's acceptance on its own profile would then close another judge's report from another profile: `case_runs`
    judges one profile a session, `class_judge` the 21 reference runs. Resolution is per report now, and a judge
    resolves only what it reported.
  - The contract is not in the key, and `ledger.rs`' header says why: `case_runs` and `class_judge` report one
    instruction's result under two contracts (its Low 1).
- **The count:** 20 reports in 4 defects, all open: FCOS st0 (9 reports), FSIN st0 (9), FCOS FSW (1), FSIN FSW (1).
  The ledger file had 20 entries.
- **The gate:** `tests/ledger.rs` reads the judged records' reports without `rebuild` and holds the stored ledger to
  them: its entries are the distinct defects, and each report is in exactly one entry. `case_runs --check-ledger`
  still holds the file to `rebuild`. `first_frame` became public, so the test reads only judged records whole (21.7 s
  from 36.7 s).
- **Plant 1** (the key the unit alone, `first.split(':')`): the ledger held 2 entries, and the gate failed, "the
  ledger's entries are not the defects the records report", left `{"FCOS", "FSIN"}`, right `{"FCOS FSW", "FCOS
  st0", "FSIN FSW", "FSIN st0"}`. Restored byte for byte. Its first `sed` was single-quoted around `':'` and did not
  compile; nothing ran on it, and it was set again in double quotes.
- **Plant 2** (a resolution that does not close the entry): `a_discrepancy_is_resolved_by_the_run_that_fixed_it`
  failed, "left: Null, right: \"case-runner-2\"". Restored byte for byte.
- **Plant 3** (one of FCOS st0's reports dropped from the stored file): `case_runs --check-ledger` exited 1, "is not
  the ledger the records give", and the gate failed, "a report is in no entry", 19 against 20. Restored byte for
  byte.
- `class_judge.rs`' ledger check reads the class judge's reports inside each entry. WORKING_AGREEMENT's ledger row
  states the new identity.
- **A limit recorded, then resolved here:** under the first form, a defect resolved and reported again handed
  `open_on` its earlier, resolved ids too. With resolution per report, `open_on` returns only reports no record has
  resolved, and the Deferred entry (Sprint 223) is annotated resolved.
- **Plant 4** (any one resolution closes the entry, `if true`): `a_discrepancy_is_resolved_by_the_run_that_fixed_it`
  failed, "a defect another judge reported stays open", left `[]`, right `["ADD CF"]`. Restored byte for byte.
- **Plant 5** (`defect` finds no field, `{unit}_`): the ledger split into 6 entries, and the gate failed, "ledger entry
  \"FCOS: FCOS st0: 10 cases; … \" is not \"FCOS <field>\"" (reviewer-3's Low 2: the key's form is held in the gate,
  since the gate's defects come from `defect` itself). Restored byte for byte.
- **Departures:** `crates/w2m_vm/tests/ledger.rs` (new) and `crates/w2m_vm/tests/class_judge.rs`, outside the card's
  file list: the gate and the one other reader of the ledger's shape.
- **Departure, `scripts/gates.sh`:** the second gates run spent its Python stage on the frozen Python's checks, because
  the ledger under `store/case_runner/` matched the Python trigger; no Python had changed. On the user's Decision of
  2026-10-29, "No Python runs", the run was stopped with no pass line, and the gates lost every Python stage: the
  trigger, ruff, mypy, the 2,547 pytest tests, the multi-vCPU stage's four frozen-host tests, `check_replay.py` and
  the Python records check. The Rust records check stands alone.
