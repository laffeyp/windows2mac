# Sprint 349 — A card amendment is a named kind of commit inside its sprint

```yaml
---
id: 349
status: done
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

- **Gates:** `scripts/gates.sh` exit 0 in 322 s, kept in `store/gates/passes.tsv` as sprint 349 at 2026-11-26T09:38:31Z on f93d38e38 with root=ac6a905ac and "rust=crates python=0 samples=0", base=c3ce9bb4d, Sprint 348's commit; 57 tests run, 57 passed, 104 skipped; multi-vCPU: not run (Python gates not run)
- **The rule** (`crates/signal_report/src/main.rs`): `is_card_amendment(subject, n)` is a subject opening "Sprint N card
  amendment:"; inside sprint N's walk `commits_walked` passes it and names each path `amendment_outside` finds, the
  commit's `git diff-tree --no-commit-id --name-only --no-renames -r` against its parent less `sprints/sprint-NNN-*.md`
  (no further `/`) and `probes/...`, as "sprint N: commit H \"S\" amends its card and changes P, outside card N and
  probes/". A "Sprint M card amendment:" with M not N falls through to "stands between its card and the next card",
  as before.
- **`BETWEEN_SPRINTS`:** 003a0c1a9 passes under the rule (`git show --stat`: `probes/tls_expansion_first/probe.json`
  and `tls_expansion_first.c`, nothing else) and is dropped, leaving c2fe0bbc8 alone; the constant's comment keeps that
  003a0c1a9 stood there from 347 to 349. Grounds: the comment's own, "a lapse stands here by name rather than by a
  looser rule", and a commit a rule admits is no lapse; and with it dropped the record itself judges the rule, plant 1
  failing sprint 347 on the tree.
- **The readers of a sprint's commits:**
  - `commits_walked`: accepts the kind, held to card N and `probes/`.
  - `close_commit`: takes "Sprint N closed:"/"closed-partial:", or from 334 the newest "Sprint N: " with card N done in
    its tree; an amendment opens neither, so it is never the close.
  - `trees_moved`: holds the gates' tree to the newest "Sprint N: " commit, or the working tree while the close is
    prepared; never the amendment.
  - `base_problem`: compares the row's `base=` with `close_commit(n - 1)`; never the amendment.
  - `close_moved`, `ci_held`, `binaries_unwalked`, `working_tree_records`: find the close through `is_close` or
    `close_commit`; `binaries_unwalked`'s change, base to close, holds the amendment's paths as the sprint's change.
  - `sprint_commits` (the report's list and records): every "Sprint NNN" commit, the card's and the amendment's
    alike; 347's committed report lists 003a0c1a9 and is its regeneration.
  - `scripts/gates-base.sh`: greps `^Sprint [0-9]+(:| closed(-partial)?:)`; "Sprint N card amendment:" has " card"
    after the number, so it is never a base. Unchanged.
  - `scripts/pre-commit.sh`: reads the cards' status, no commit; an amendment made through `scripts/commit.sh` with
    card N active checks the record against N-1, as any card commit. Unchanged.
  - Searched: `scripts/*.sh`, `.github/workflows/*.yml` and every crate but `signal_report` for a subject read or
    "Sprint {n}" formatted; none other.
- **Fixtures** (`crates/signal_report/tests/check.rs`, `folded`'s `Shape` gains `amendment` and `amendment_crates`):
  `a_card_amendment_of_its_card_and_a_probe_passes_inside_its_sprint` (card 335 and `probes/p/p.c` pass, close
  committed and prepared; "Sprint 336 card amendment:" inside 335 named "stands between") and
  `a_card_amendment_that_changes_another_path_is_named` (`crates/a/extra.rs` named): both passing; `signal_report`'s
  tests 7, 13, 2, 19, 5 and 2 passed. Their reach before the work: the first failed on today's code, `left: (Some(1),
  "sprint 335: commit 550318886 \"Sprint 335 card amendment: the probe p\" stands between its card and the next card,
  and is neither its work nor its close")`.
- **The whole record:** `target/release/signal_report --check` exit 0 on the tree, 347 included.
- **Plant 1** (the rule taken out, line 1573 `if false && is_card_amendment(...)`): "sprint 335: commit 7b7a8f0f4 \"Sprint 335 card amendment: the probe p\" stands between its card and the next card, and is neither its work nor its close"; the record check on the tree failed too, "sprint 347: commit 003a0c1a9 \"Sprint 347 card amendment: the probe tls_expansion_first, ...\" stands between its card and the next card"; restored, `cmp` equal.
- **Plant 2** (the path limit taken out, line 1534 `false && !(is_card || ...)`): `a_card_amendment_that_changes_another_path_is_named` failed, "left: Some(0) right: Some(1)", the `crates/` amendment passing; restored, `cmp` equal.
- **The working agreement:** "A sprint closes in its work commit" gains one sentence, dated 2026-11-26.
