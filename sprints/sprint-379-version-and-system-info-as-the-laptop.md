# Sprint 379 — The version and system-info answers as the laptop gives them

```yaml
---
id: 379
status: active
phase: 6
pass_kind: functional
---
```

## scope

One path from input to verdict (`WORKING_AGREEMENT.md` 301): w2m's version and system-info answers re-taken from the
laptop, and the three families that read them held to the laptop's rows. Sprint 378 built the route
(`store/family_runs/retaken-2026-11-27.tsv`) and wrote, in its card's result, the laptop runs and rows this card needs:
`version` (`lap-20261013T231059422Z-136bec80-186`, `-187`), `system_info` (`-156`, `-157`), `files_more` (`-084`, `-085`),
16 runs, all build 26200.7840; the rows by which the laptop differs from the runner (product type 1, suite 768, a build
above 26100, shell32's revision 7840 reduced to `version_low_set`, the Windows directory `C:\windows`); and the one
value no stored row holds, GetSystemDirectoryW's string (length 19, not `C:\Windows\system32`).

- **The measurement.** `tools/windows_machine/profile.ps1` records `system_directory` and `windows_directory` in
  `profile.json`, by a call whose path to GetSystemDirectoryW and GetWindowsDirectoryW is cited from Microsoft's
  documentation or .NET's source; one laptop job of the `system_info` probe records them and stores a 26200.9974 run
  of the family. This is a laptop path, committed by card amendment (Sprint 377's rule).
- **The tables.** `tools/guest_runtime/version.c` and `system_info.c` give the laptop's values, each cited to its row;
  physical memory stays 16 GiB (the Decision "The order after Sprint 378").
- **The families.** `contracts/windows.version.version/0.2.0.json`, `windows.system.information/0.2.0.json` and
  `windows.file.files_more/0.2.0.json`, each 0.1.0's with the laptop run's rows and evidence (0.1.0 kept unchanged),
  named in `retaken-2026-11-27.tsv`; `store/family_runs/known-2026-10-29.tsv` regenerated; the two files' entries in
  `crates/guest_runtime/tests/python_image.rs` DEPARTURES, each with its reason.
- **The readers.** Every reader of the family tables reads the new table: `reference_machine.rs` `FAMILY_TABLES`
  first, and the others a search for `joined-2026-11-27.tsv` finds (`family_runs.rs`, `family_known.rs`,
  `runner_families_on_the_laptop.rs`, `windows_machine`'s `lib.rs` and `windows_machine.rs`).
- **Not here:** the spec tables (cards 380, 381). No API id is added.

## prerequisites

- 378 (closed)

## context_files

- `sprints/sprint-378-version-and-system-info-from-the-laptop.md` (its result, "For cards 379-380").
- `tools/guest_runtime/version.c`, `system_info.c`; `probes/version/`, `probes/system_info/`, `probes/files_more/`.
- `contracts/windows.version.version/0.1.0.json`, `windows.system.information/0.1.0.json`, `windows.file.files_more/0.1.0.json`.
- `crates/w2m_vm/src/bin/family_known.rs`, `crates/w2m_vm/tests/family_runs.rs`, `crates/guest_runtime/tests/python_image.rs`.
- `tools/windows_machine/profile.ps1`; `crates/windows_machine/tests/reference_machine.rs`.
- `grep -n "The order after Sprint 378\|Which Windows w2m answers as" BLACKBOARD.md`.
- `WORKING_AGREEMENT.md` lines 137-266 and 301.

## signal contract

### Emits

- None declared: the family runs the judge holds are the gates' existing runs.

### Invariants

- Every run as held; G3's batch runs conform; no Windows run on GitHub's runner; `crates/guest_runtime/src/api_ids.txt`
  unchanged; every 0.1.0 contract byte-equal to HEAD's.

## artifact contract

### Files created

- `contracts/windows.version.version/0.2.0.json`, `contracts/windows.system.information/0.2.0.json`,
  `contracts/windows.file.files_more/0.2.0.json`; the laptop job's `store/probe-runs/<run>/`.

### Files modified

- `tools/guest_runtime/version.c`, `system_info.c`, `tools/windows_machine/profile.ps1` (by amendment),
  `store/family_runs/retaken-2026-11-27.tsv`, `known-2026-10-29.tsv`, `crates/guest_runtime/tests/python_image.rs`,
  the family-table readers; file count recorded (`WORKING_AGREEMENT.md` 301).

### Command exit codes

- `target/release/windows_machine probe system_info` returns 0, its `profile.json` carrying `system_directory`
- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The value table: call, field, w2m before, laptop 7840, laptop 9974 (the new run), w2m after, the row cited.
  - The 16 runs of `version`, `system_info` and `files_more`, each conforming to its 0.2.0 contract.
  - The new `system_info` run's rows against `-156`'s, by Sprint 374's four outcomes.
- **Planted faults:**
  - (1) The product type set back to 3: the `version` and `system_info` judges fail naming the row.
  - (2) A route row removed: the family is held to 0.1.0 and fails naming the row.
  - (3) A laptop run cited only in `retaken-2026-11-27.tsv` changed to a runner's: the cited-runs walk fails naming it.

## done criteria

w2m answers the version and system-info calls as the laptop does, each value cited to a laptop row, and the three
families that read them conform to contracts holding the laptop's rows.

## self-review checklist

- [x] Scope names one verifiable property: the three families conform to the laptop's rows.
- [x] context_files cite Sprint 378's facts, the tables, the contracts, the route and the readers.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command; file count recorded.
- [x] Observation contract: the value table, the runs, the new laptop run; numbered plants.
- [x] One concept: one path from the laptop's rows to the families' verdicts.

## result

GATES-LINE
