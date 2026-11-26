# Sprint 340 — A Signal Report sees the records its own commit adds

```yaml
---
id: 340
status: active
phase: 6
pass_kind: functional
---
```

## scope

reviewer-4's eleventh review, High 1, was verified by the supervisor and recorded as the Decision "Where the Signal
Report's fix goes in the order" (2026-11-25).

- **The fault.** `signal-reports/sprint-337-report.md`, as committed, reads "from 0 records". Its trace says "(no
  record)", and all four of its declared tags read "NOT FIRED". `signal_report 337`, regenerated now, reads "from 1
  records".
- **The cause.** Since Sprint 334 a sprint closes in its work commit. So its report is written before the commit that
  adds its record exists, and `sprint_records` (`crates/signal_report/src/main.rs` 172-191) counts only records whose
  baseline names a sprint commit or which a sprint commit added (`added_records`, 147-170).
- **The second case.** Sprint 336's report reads three "NOT FIRED", because its samples record is not kept under
  `store/runs/`.
- **What the check misses.** `emits_check` (237-260) writes "NOT FIRED" as text, and `--check` fails on nothing it
  says, against `AGENTS.md`'s "every tag declared in `## signal contract → Emits` must fire".

- **The work:**
  - **The generator sees the working tree.** When the sprint's close is not committed (the rule `close_commit` and
    `trees_moved` already apply from `FIRST_FOLDED_SPRINT`), `sprint_records` also counts the record directories the
    working tree adds under `store/runs/` over the sprint's base. That covers both staged and untracked directories.
  - **`--check` fails a NOT FIRED tag.** It fails a closed sprint whose card's Emits names a tag the sprint's report
    shows NOT FIRED, from a first sprint chosen on the grounds. Before choosing, read how many closed reports from 171
    on show NOT FIRED, and list them in the result. For each, write whether its Emits named the tag as one that fires
    or only in prose.
  - **`--check` regenerates and compares.** It regenerates each closed sprint's report from the committed tree and
    fails on any difference from the committed file. This runs from a first sprint chosen the same way, after reading
    how many reports differ today and why.
  - **The corrections.**
    - Sprint 337's report is regenerated. It is committed in this sprint's one commit, and the commit message says
      why.
    - Sprint 336's samples record is kept: one run under `store/runs/` from `harness/vocabulary-samples.toml`,
      replayed and checked, and its report regenerated. Or, if a kept samples record breaks a rule the records hold,
      its card's Emits is annotated with the reason, dated and naming this card.
  - **Sprint 339 needs no correction.** It made no record, which its tail entry says.
- **Not here:**
  - Any change to the gates' records check (`check_records`).
  - Any change to what a sprint's Emits must name.

## prerequisites

- 339 (closed-partial)

## context_files

- `crates/signal_report/src/main.rs`, read whole where named:
  - `sprint_commits` (136), `added_records` (147), `sprint_records` (172), `emits_check` (237), `report` (412);
  - the `--check` path (around 1487-1520) and `unquoted_plants` (546);
  - `close_commit`, `trees_moved` and `FIRST_FOLDED_SPRINT` (`grep -n` each).
- `crates/signal_report/tests/check.rs`: its fixture repositories, `folded()` among them (Sprint 333).
- `signal-reports/sprint-336-report.md`, `signal-reports/sprint-337-report.md`, and `sprints/sprint-336-vocabulary-0-13.md`
  (its Emits).
- `harness/vocabulary-samples.toml` and `scripts/gates.sh` 258-286 (how the samples run today, under a temporary root).
- The Decision: `grep -n "Where the Signal Report's fix goes in the order" BLACKBOARD.md`.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands", with "A sprint closes in its work commit"
  and "no gate row is dropped".

## signal contract

### Emits

- None of its own; a change to the records' checks. If Sprint 336's samples record is kept, that record fires every
  live harness kind 336's Emits names.

### Invariants

- `signal_report --check` passes on the committed tree, the corrected reports included.
- No Python file is added or changed.

## artifact contract

### Files modified

- `crates/signal_report/src/main.rs`, `crates/signal_report/tests/check.rs`, `signal-reports/sprint-337-report.md`,
  and `signal-reports/sprint-336-report.md`. Also either a kept samples record under `store/runs/` or Sprint 336's
  card annotated.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - A test builds a folded fixture whose prepared close adds a record under `store/runs/`, and holds the report
    written before the commit to read "from 1 records" with its tags fired.
  - A second fixture's committed report reads "(no record)" over a kept record, and `--check` fails it by name.
  - A third fixture's declared tag shows NOT FIRED, and `--check` fails it by name.
  - Quote the count of closed reports that differ when regenerated, and the first sprint each new rule starts from,
    in the result.
- **Planted faults:**
  - (1) The working tree's records left out of `sprint_records`. The first fixture's report reads "from 0 records",
    and the test fails.
  - (2) The NOT FIRED rule taken out. The third fixture passes `--check`, and the test fails.
  - (3) The regenerate-and-compare taken out. The second fixture passes `--check`, and the test fails.

  Before the work, read each judge's reach: each test runs `signal_report` against its fixture, which reaches the
  planted function.

## done criteria

A Signal Report written before its sprint's commit counts the records that commit adds. `--check` fails a declared tag
that did not fire, and a committed report that differs from its regeneration. Sprints 336 and 337's reports are
corrected.

## self-review checklist

- [x] Scope names one verifiable property: the report counts its commit's records and `--check` fails on NOT FIRED and on a stale report.
- [x] context_files cite the generator, the check, the folded rules, the fixtures, the two reports and the Decision.
- [x] Emits: none of its own.
- [x] Artifact contract gradable by command.
- [x] Observation contract: three fixtures held, the counts quoted; three numbered plants, their reach read.
- [x] Sweet spot: `main.rs` and `check.rs` plus the corrected reports; one concept, a report true to its records.

## result

GATES-LINE
