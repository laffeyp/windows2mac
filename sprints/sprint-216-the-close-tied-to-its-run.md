# Sprint 216 — The close tied to its run

```yaml
---
id: 216
status: done
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 214" (reviewer-3's Medium 3 of its review of Sprints 212-214). Sprint 212's
`ci_held` (signal_report) holds a closed tail entry to the CI run a later entry cites for it, as `store/gates/ci.tsv`
records that run's conclusion; it does not hold the run to the sprint's close, nor to the `ci` workflow, and its waiver
for a failed run reads the whole entry. WORKING_AGREEMENT's "CI on the closing commit" says the run of the closing
commit.

- **The head:** a cited run's head (ci.tsv's second column) must hold the sprint's close commit ("Sprint N closed:" or
  "closed-partial:"), `git merge-base --is-ancestor <close> <head>`; a run of another tree is named.
- **The workflow:** ci.tsv keeps each run's workflow name (`gh run view --json workflowName`), and only `ci` counts.
- **The waiver:** a failed run passes only when the clause that names the run's id also says it failed.
- **Pending, not a heading:** the review's "closed (CI pending)" heading is not taken. Only the latest close can be
  pending, since the next sprint's entry cites its run before that sprint closes, and `ci_held` holds it from then;
  a heading that changes after its close would be an edit to a closed entry for every sprint.

## prerequisites

- 215 (closed)

## context_files

- `crates/signal_report/src/main.rs` (`ci_held`, `closed_too_early`, `close_moved`); `store/gates/ci.tsv`;
  WORKING_AGREEMENT ("CI on the closing commit")

## signal contract

### Emits

- None: the judge is `signal_report --check`.

### Invariants

- Every closed entry from Sprint 199 passes, or is annotated with the run it stood on.

## artifact contract

### Files modified

- `crates/signal_report/src/main.rs` and its tests, `store/gates/ci.tsv` (a workflow column), WORKING_AGREEMENT

### Command exit codes

- `signal_report`'s tests, `signal_report --check` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** every cited run from Sprint 199 holds its sprint's close and is a `ci` run; a waiver reads its own
  clause.
- **Planted faults:** (1) a run whose head predates the close: `--check` names it; (2) a run of another workflow: named;
  (3) "failed" in another clause than the run's id: the waiver refused, named.

## done criteria

A close stands on the `ci` run of a tree that holds it, read from the tracked file, or names the run it stood on.

## self-review checklist

- [x] Scope names one verifiable property: the run holds the close.
- [x] context_files cite the rule and the code.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the close's run.

## result

- **The workflow:** `store/gates/ci.tsv` keeps each run's `workflowName` in a fifth column, read again with `gh run view`
  for the 16 runs on file: all `ci`. `ci_held` names a cited run of another workflow.
- **The head:** `ci_held` takes the sprint's close commit (`close_commit`, "Sprint N closed:" or "closed-partial:" by
  `git log`) and names a cited run whose head does not hold it (`git merge-base --is-ancestor`). Every cited run from
  Sprint 199 on the board holds its close.
- **The waiver:** read from the clause that names the run's id (clauses end at ";", ")" and ". "); Sprint 210's
  annotation passes, its clause naming 40936387557 saying "failed".
- **Pending:** the "closed (CI pending)" heading is not taken, as the scope says.
- **Plant 1** (every head made not to hold its close): `--check` named "sprint 214: its CI run 40952329895 ran on
  309ef13adcc8d2c21136a6e37b59c9b9150a5203, which does not hold the sprint's close", and 213. Restored byte for byte.
- **Plant 2** (Sprint 213's run recorded as windows-probe in ci.tsv): `--check` named "sprint 213: its CI run 40948787264
  is of the workflow \"windows-probe\", not ci". Restored byte for byte.
- **Plant 3** (the waiver read from the whole entry): the unit test failed, "\"failed\" in another clause waives
  nothing: []". Restored byte for byte.
- **Departures:** none from the card's files; ci.tsv was rewritten from `gh run view`'s output, as Sprint 212 wrote it.
