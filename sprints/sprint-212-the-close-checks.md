# Sprint 212 — The close checks

```yaml
---
id: 212
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 211" (reviewer-3's High 2 and Medium 5 of its review of Sprint 211). Two rules
that live in prose failed twice running:

- **A close on its own CI:** WORKING_AGREEMENT's "CI on the closing commit" says a sprint is closed when the `ci` run
  of its closing commit concludes success. The tail writes "closed" at the close commit, before that run exists, and
  cites the previous sprint's run. Sprint 210 stood closed on run 40936387557, which failed; the run's result was read
  once from a watch command's trailing `echo` and reported as a pass. The check: each CI conclusion, read with
  `gh run view <id> --json conclusion,headSha`, is kept in a tracked file, `store/gates/ci.tsv` (run id, head,
  conclusion, read time); `signal_report --check` fails a tail entry headed closed or closed-partial whose closing
  commit, or a later commit holding it, has no run in that file concluding success, once the next sprint has closed
  (the entry names it in "Closed:"); a close may be headed "closed (CI pending)" until then.
- **Every quoted `sed`:** `quoted_edits` (signal_report) reads backtick spans only, so the two multi-line `sed`s of
  Sprints 210 and 211, quoted in double quotes, were never classed by `edit_guard`; and one owning phrase exempts every
  command in the entry. The check: every `sed -i` command in a closed entry, in backticks or double quotes, goes to
  `edit_guard::refusal`, and a refused one passes only when the sentence quoting it owns it ("a multi-line edit outside
  the Edit tool" in that same clause).

## prerequisites

- 211 (closed)

## context_files

- `crates/signal_report/src/main.rs` (`quoted_edits`, `run`, the close rules), `crates/signal_report/tests/check.rs`,
  `crates/edit_guard`, WORKING_AGREEMENT ("CI on the closing commit"), BLACKBOARD tail entries 209-211

## signal contract

### Emits

- None: the judge is `signal_report --check`, run by the gates and CI.

### Invariants

- No vocabulary change; every closed entry from 199 passes the new checks, or the check names it and this card records
  the annotation it needs.

## artifact contract

### Files modified

- `crates/signal_report/src/main.rs`, `crates/signal_report/tests/check.rs`, `store/gates/ci.tsv` (new, with the runs
  of Sprints 199-211 read by `gh run view`), WORKING_AGREEMENT ("CI on the closing commit")

### Command exit codes

- `signal_report`'s tests, `signal_report --check` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** `store/gates/ci.tsv` holds a row per run cited in the tail from Sprint 199, run 40936387557
  "failure" among them; `--check` names Sprint 210's close unless its entry carries the annotation of the failed run
  and the later run of 211 that holds its commits; the two double-quoted `sed`s of 210 and 211 are classed and pass
  only as owned.
- **Planted faults:** (1) a closed entry whose CI row reads failure, without the annotation: `--check` names it; (2) a
  double-quoted multi-line `sed` not owned: `--check` names it; (3) one owning sentence beside a second, unowned
  breach in the same entry: `--check` names the second.

## done criteria

A sprint cannot read closed in BLACKBOARD on a CI run that did not pass, and every quoted multi-line `sed` is classed
and owned where it is quoted.

## self-review checklist

- [x] Scope names one verifiable property each.
- [x] context_files cite the rules and the code.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the close's own checks.
