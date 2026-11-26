# Sprint 340 — A Signal Report sees the records its own commit adds

```yaml
---
id: 340
status: done
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

- **The generator** (`crates/signal_report/src/main.rs`, `sources`): from Sprint 334 the commit that holds the report,
  the sprint's work commit (`close_commit`), is left out of the commits it lists, since a file cannot name the commit
  that holds it; the records that commit adds are the sprint's, read from the commit once made and, while the close
  is prepared, from `working_tree_records`: the directories `git diff --name-only --diff-filter=A <previous close> --
  store/runs` and `git ls-files --others --exclude-standard -- store/runs` name, staged and untracked. Line 3 names
  "the commit that holds this report" only when it adds a record, so a report whose close adds none reads as before.
  Before Sprint 334 the generator is unchanged. The record directories and their baselines are read once (`Runs`,
  38,706 records, about 2 s), so `--check` regenerates several reports at that cost.
- **Reports that differ when regenerated:** all 232 reports from 108 to 339, regenerated on 2026-11-25 by the new
  generator into the scratchpad and compared with `diff`: 5 the same (334, 335, 336, 338, 339), 227 different. 208
  differ only by a commit made after the report (the "Sprint N closed:" commit that holds it, which it could not
  name); 15 (108-122) by `scripts/signal_report.py`'s line 3; 150 by a record Sprint 152 removed
  (`store/runs/case-runner-1792916257296645000`); 205 and 222 by annotations written into the report by hand; 337 by
  the record its close added. Under the old generator every folded report (334-339) differed, by its work commit.
  `--check` regenerates and compares from **334** (`FIRST_REGENERATED_SPRINT`), the first sprint that closes in its
  work commit, where the fault began; from 334 on only 337's differed, now regenerated.
- **Reports that show NOT FIRED, 171 to 339:** eleven, 30 lines. As a tag that fires: 171 (SESSION_INIT, on a
  temporary root), 210 (all six, the batch run's record under `build/`), 211 (SERVICE_UNIMPLEMENTED, the same record),
  214 (COMPARISON_COMPLETED), 217 (COMPARISON_COMPLETED, SESSION_COMPLETED), 218 (EXECUTION_COMPLETED,
  COMPARISON_COMPLETED, SESSION_COMPLETED, in its annotation), 219 (all three), 336 (all three, the samples on a
  temporary root), 337 (all four), 339 (MEASUREMENT_RECORDED, its record not made, closed-partial). Only in prose:
  214's TRACE_INGESTED ("only if"), 218's SERVICE_UNIMPLEMENTED ("It writes no"), 223's three ("rebuilt from the
  records'"). `--check` fails NOT FIRED from **340** (`FIRST_FIRED_SPRINT`): 339's line is true and its card needs no
  correction, and 336's stands with its card annotated. It reads the committed report's "- `TAG`: NOT FIRED" lines of
  every card `status: done` or `closed-partial`.
- **The corrections:** `signal-reports/sprint-337-report.md` regenerated: "from 0 records" became "from 1 records ...
  or which those commits or the commit that holds this report added", the trace
  `w2m-vm-measurements-1795656377000000000: SESSION_INIT 1, MEASUREMENT_RECORDED 44, CAPABILITY_PROBED 3,
  SESSION_COMPLETED 1`, and its four tags fired 3, 44, 1 and 1 times. Sprint 336's samples record is not kept: a
  samples run into the dated record `vocabulary-samples-1795660404000000000` replayed at level 2 (128 frames), and
  `check_records --since 2026-10-17` exited 1, "1 records; 18 violations; in records made on or after 2026-10-17: 18
  held, 0 known" (T2 1, T3 3, T4 2, S2 8, S4 2, S8 2); its baseline also names no commit, so it would be 340's
  record. Card 336's Emits is annotated (2026-11-25, Sprint 340) and its report regenerated, its three tags still NOT
  FIRED and the annotation in its Expected.
- **The fixtures** (`crates/signal_report/tests/check.rs`): `folded` takes a first sprint (334 unless given), writes
  each report by `signal_report N --write` on the prepared close over a two-tag vocabulary, and can add the grouped
  record `store/runs/class-fixture-1795600000000000000`. `a_report_made_before_its_commit_counts_the_records_the_commit_adds`
  (340-341, prepared): the report reads "from 1 records", "class-* 1 records: SESSION_INIT 1" and "- `SESSION_INIT`:
  fired 1 time"; `--check` exits 0 before the commit and after it. `a_committed_report_that_misses_its_record_is_named`
  (committed, report made with the record moved away): "sprint 341: signal-reports/sprint-341-report.md differs from
  its regeneration, `signal_report 341`, first at line 3". `a_declared_tag_not_fired_is_named`: "sprint 341: its
  card's Emits names SESSION_INIT, and signal-reports/sprint-341-report.md shows it NOT FIRED". The Sprint 333
  fixtures pass with real reports in place of "# report". `cargo nextest run -p signal_report`: 46 passed.
  `target/release/signal_report --check` on the tree: exit 0, "every closed sprint from 340 fired each tag its
  card's Emits names", "every closed sprint's report from 334 is its regeneration".
- **Plant 1** (`main.rs` 257, `(true, None) => BTreeSet::new(),`): the judge failed, "panicked at
  crates/signal_report/tests/check.rs:755:5", the report "Made by `signal_report 341` from 0 records under
  store/runs/".
- **Plant 2** (`main.rs` 668, `closed_sprints(root, u32::MAX)`): the judge failed, "left: (Some(0), \"\")", against
  "right: (Some(1), \"sprint 341: its card's Emits names SESSION_INIT, ...\")".
- **Plant 3** (`main.rs` 701, `if false && made != kept`): the judge failed, "left: (Some(0), \"\")", against "right:
  (Some(1), \"sprint 341: signal-reports/sprint-341-report.md differs from its regeneration, ...\")".
  Each plant was one `sed` line from a cp backup, its judge `cargo nextest run -p signal_report --test check <test>`
  (exit 100), restored by cp, cmp clean, before the gates.
- **The gates:** one run on the restored tree, "gates: steps 199 s of the run's 206 s, unplaced 7 s", "gates: all
  passed"; its row:

exit 0 in 206 s	default	2026-11-26T02:55:46Z	sprint 340	293c26c5ccd4a56056feb444cf18b02d31e98d68	7d76b4f1f3068f2644ae6fbc7cb2f2c3e0021ef5186ed2dd1811b00592ac624b	root=cf2ea111fce282257d32ac7eedd66bc170f8656f	rust=crates python=0 samples=0	base=1842378f76de8b789b2224298539632c0195d117
