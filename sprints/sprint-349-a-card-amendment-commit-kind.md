# Sprint 349 — A card amendment is a named kind of commit inside its sprint

```yaml
---
id: 349
status: active
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-4's eighteenth review, Low 1, recorded in the Decision "Two cards from the eighteenth review". Two rules now
recur together:
- the user's Decision of 2026-10-18 has a run-stopping defect fixed in the card that finds it;
- the Windows runner builds only pushed commits.

So a card that finds such a defect and needs Windows rows for it pushes a probe partway through its sprint. Sprint
347 did, in commit 003a0c1a9, "Sprint 347 card amendment: ...". `signal_report`'s `commits_walked` refused that
commit, "stands between its card and the next card, and is neither its work nor its close", until it was named by
hand in `BETWEEN_SPRINTS`. Each later case would add another hand-named entry. This card replaces that with one
general rule.

- **The rule.** Within sprint N, `commits_walked` accepts a commit whose subject begins "Sprint N card amendment:"
  when its change, against its parent, touches only `sprints/sprint-N-*.md` (card N itself) and paths under
  `probes/`. Any other path in such a commit is named, with the commit and the path. A "Sprint M card amendment:"
  commit inside sprint N, where M is not N, is named as today.
- **`BETWEEN_SPRINTS`.** Its two entries stay as history. Say whether 003a0c1a9 now passes under the rule, which it
  should: it changed card 347 and `probes/tls_expansion_first/`. Keep its entry with a note that the rule now covers
  it, or drop it from the list on the grounds the code's own comment gives. Record which, with its grounds.
- **The other readers of a sprint's commits.** Find every place that walks a sprint's commits or picks its base, and
  make each accept the kind or state why it need not:
  - `scripts/gates-base.sh`;
  - `scripts/pre-commit.sh`;
  - `signal_report`'s `close_commit`, `trees_moved` and `base_problem`;
  - any other found.

  A card amendment must not be taken as a sprint's work or its close.
- **The working agreement.** "A sprint closes in its work commit" (`WORKING_AGREEMENT.md`) gains one dated sentence
  naming the kind, when to use it (a probe the Windows runner must build partway through a sprint), and what it may
  change.
- **The judge.** `crates/signal_report/tests/check.rs`'s folded fixture (`folded`, `a_sprint_closes_in_its_work_commit`)
  gains:
  - a card amendment commit that changes the card and a probe, which passes;
  - one that also changes a file under `crates/`, which is named by its path.
- **Not here:** any other change to the record rules.

## prerequisites

- 348 (closed)

## context_files

- `crates/signal_report/src/main.rs`: `commits_walked`, `BETWEEN_SPRINTS`, `close_commit`, `trees_moved`,
  `base_problem`, `is_close`, `FIRST_FOLDED_SPRINT`.
- `crates/signal_report/tests/check.rs`: `folded` and the tests that use it.
- `scripts/gates-base.sh` and `scripts/pre-commit.sh`.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands", with "A sprint closes in its work commit".
- Sprint 347's tail entry (`grep -n '^### Sprint 347' BLACKBOARD.md`) for the case.

## signal contract

### Emits

- None; a record rule.

### Invariants

- `target/release/signal_report --check` passes on every sprint already recorded, 347 included.

## artifact contract

### Files modified

- `crates/signal_report/src/main.rs`, `crates/signal_report/tests/check.rs`, `WORKING_AGREEMENT.md`, and either
  script only if it walks a sprint's commits.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - Each reader of a sprint's commits, with what it does with a card amendment.
  - The two new fixture tests passing.
  - `signal_report --check` on the whole record passing.
- **Planted faults:**
  - (1) The rule taken out. The passing fixture is named, "stands between its card and the next card".
  - (2) The path limit taken out. The fixture whose amendment changes `crates/` passes, and its test fails.

  Before the work, read each judge's reach: the passing fixture failing on today's code.

## done criteria

A commit "Sprint N card amendment:" that changes only card N and `probes/` passes the record check inside sprint N,
any other path in it is named, and no sprint needs a hand-named exception for a probe pushed partway through.

## self-review checklist

- [x] Scope names one verifiable property: one named commit kind, limited to card N and `probes/`.
- [x] context_files cite the record rules, their fixtures, the scripts and the working agreement.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the readers, two fixtures, the whole record; two numbered plants, their reach read.
- [x] Sweet spot: `signal_report`'s rule and its test; one concept, a commit kind.

## result

GATES-LINE
