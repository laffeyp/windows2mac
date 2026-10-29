# Sprint 216 — The close tied to its run

```yaml
---
id: 216
status: active
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
