# Sprint 301 — The pre-commit checks a script

```yaml
---
id: 301
status: active
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Low 1 at 300: WORKING_AGREEMENT's "Before a work commit" list is typed by hand at each commit and was
amended at Sprints 247, 277, 281, 285, 286, 288 and 298; the hand-typed chain failed at 281 (joined by `;`, a lint
failure committed), 285 (`set -e` in a zsh subshell, which does not stop) and 299 (piped into `tail`, "exit 0" over a
failed lint). The gates, a script under `/bin/sh -e`, had none of these faults.

- **The work:** `scripts/pre-commit.sh`, `#!/bin/sh` with `set -eu`, running the list as WORKING_AGREEMENT's
  amendment at 298 leaves it: `cargo fmt --all --check`; clippy with the gates' flags; the Linux check; `design_check`
  and `signal_report <the last closed sprint> --check`, the sprint read from the one active card's id; the doc tests of
  every crate with a library. No pipe and no `echo` after the last check, so its exit status is the last check's; the
  amendments' reasons kept in its comments. WORKING_AGREEMENT's rule one line: run `scripts/pre-commit.sh` and read its
  whole output before a work commit, and again after any edit.
- **The doc tests of every crate with a library,** where Sprint 292's amendment names the changed crates': wider, by
  design, so the script holds no selection of its own to drift from the gates'; its time on the tree is in the
  result, and the gates' `touched` rule can narrow it if that time matters (reviewer-3, by message at 300).
- **Not here:** the tests and the batch run, which run once, in the gates (Sprint 298's amendment); a hook refusing
  `git commit`, which the user installs or not.

## prerequisites

- 300 (closed)

## context_files

- WORKING_AGREEMENT "Before a work commit" and its amendments; `scripts/gates.sh` (its `/bin/sh -e` form, the
  record check's call)
- BLACKBOARD Surfaced, the review at 300

## signal contract

### Emits

- None; a procedure made a script.

### Invariants

- Every check the list held, run in its order.

## artifact contract

### Files modified

- `scripts/pre-commit.sh` new, `WORKING_AGREEMENT.md`

### Command exit codes

- `scripts/pre-commit.sh` returns 0 on the tree as committed; `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the script's output naming each check in order and its exit 0; its time on the tree.
- **Planted faults**, each judged by the script's exit status and its output:
  - (1) a line of a source file indented by one extra space: the script must exit non-zero at the format check,
    naming the file, and run no check after it.
  - (2) the active card's status set to `done` with another left active, or none active: the script must exit
    non-zero naming the cards it found, not run the record check against a guessed sprint.

## done criteria

A work commit's checks are one command whose exit status is theirs.

## self-review checklist

- [x] Scope names one verifiable property: the list one command with its status.
- [x] context_files cite the rule, its amendments and the gates' form.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what its judge reads.
- [x] One concept: two files.

## result

