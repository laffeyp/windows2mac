# Sprint 380 — The system messages re-taken from the laptop's stored rows

```yaml
---
id: 380
status: active
phase: 6
pass_kind: functional
---
```

## scope

`specs/system_messages.json` (version 0.1.0, `windows_build` 10.0.26100) holds FormatMessage's text for each system
error code, taken from GitHub's Server runner: `store/probe-runs/38120951889/body-1.jsonl`, probe `format_message`,
rows "dump". The laptop ran the same binary on 2026-10-13: `store/probe-runs/lap-20261013T231059422Z-136bec80-086`
(`probe.sha256` 4224a823... in both), build 26200.7840. This card re-takes the table from the laptop's rows (the
Decision "Which Windows w2m answers as, the reference being the laptop"; the user's rule "Match the reference",
BLACKBOARD 160), one path from input to verdict (`WORKING_AGREEMENT.md` 301).

- **The table.** `specs/system_messages.json` at 0.2.0 from `-086`'s "dump" rows, 0.1.0 kept; each code whose text
  differs between the runner and the laptop listed in the result with both texts, and each code present on one machine
  only.
- **Its readers.** Every program and test that reads the table follows the new version (a search for
  `system_messages` finds them); the `format_message` family, if it is frozen, held to the laptop's rows through
  `store/family_runs/retaken-2026-11-27.tsv` as Sprint 379 held three families.
- **Committed with this card:** `crates/signal_report/src/main.rs` names Sprint 379's close 3b17c1b78 in
  `BETWEEN_SPRINTS` (it carried the work and the close under "Sprint 379 closed-partial:", where "Sprint 379: " was due
  from Sprint 334), and a named close passes the close-apart, bookkeeping-path and work-commit checks; 51 of 51
  `signal_report` tests pass. This card's gates run holds it.
- **Not here:** `specs/windows_exports.json`: its probe, `exports`, has no stored laptop run, and Smart App Control
  refuses new laptop probe runs (Sprint 379; blocked on the user). `recorded_folders.json` and the display calls, card
  381. No API id is added.

## prerequisites

- 379 (closed-partial; its open question does not touch this table)

## context_files

- `specs/system_messages.json` (its head and its readers), `store/probe-runs/lap-20261013T231059422Z-136bec80-086/` and
  `store/probe-runs/38120951889/` (their first lines).
- `store/family_runs/retaken-2026-11-27.tsv`, `crates/w2m_vm/src/bin/family_known.rs` (the route).
- `grep -n "The version and system-info answers re-taken\|The order after Sprint 378" BLACKBOARD.md`.
- `WORKING_AGREEMENT.md` lines 137-266 and 301.

## signal contract

### Emits

- None declared: the change is to a guest table; the runs that judge it are the gates' existing runs.

### Invariants

- Every run as held; G3's batch runs conform, or a moved call is named; no Windows run on GitHub's runner;
  `crates/guest_runtime/src/api_ids.txt` unchanged; `specs/system_messages.json` 0.1.0 kept byte-equal.

## artifact contract

### Files created

- the table's 0.2.0 file, beside 0.1.0 as the project keeps a spec's versions (found from how `specs/` holds other
  versioned tables).

### Files modified

- the table's readers, and the `format_message` family's route row if the family is frozen; file count recorded.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The count of codes in each table, the codes whose text differs with both texts, the codes on one machine only.
  - The family runs that read the table, each conforming.
  - G3's verdict on one and two vCPUs, and any call the new text moves.
- **Planted faults:**
  - (1) One code's laptop text replaced by the runner's: the family judge or the table's test fails naming the code.
  - (2) The table's source changed to a runner run: a test fails naming it.

## done criteria

w2m's FormatMessage text is the laptop's, each code cited to the laptop's row, and the families and G3 that read it
conform.

## self-review checklist

- [x] Scope names one verifiable property: the table equal to the laptop's rows.
- [x] context_files cite the table, both runs, the route and the Decisions.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: counts, differing codes, families, G3; numbered plants.
- [x] One concept.

## result

GATES-LINE
