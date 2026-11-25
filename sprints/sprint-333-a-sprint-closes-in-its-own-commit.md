# Sprint 333 — A sprint closes in its own commit

```yaml
---
id: 333
status: active
phase: 6
pass_kind: functional
---
```

## scope

The user's decision of 2026-10-16 cut "bookkeeping as separate commits (each closure folds into its sprint's commit)";
since then every sprint has closed in a commit of its own, "Sprint N closed: its Signal Report; BLACKBOARD", 63 of them
in step 6 alone (reviewer-4's Medium 3 at 331; the Decision "A sprint's close folds into its sprint's commit",
2026-11-24). This card makes the project's tools and its working agreement accept, and its record check hold, a sprint
made of two commits: its card commit, and one commit "Sprint N: ..." that carries the work and the close together (the
card's status done, the Signal Report, BLACKBOARD's tail entry and the gates' rows), made after `scripts/gates.sh`
passed on the working tree. Read before this card: `signal_report`'s `trees_moved` takes the newest "Sprint N: "
commit as the work commit and compares its tree less `BOOKKEEPING` with the gates' `root=`, and `close_moved` judges
only a commit titled "Sprint N closed:"; so the tools may already accept the folded shape. The card's first act is to
find, by reading and by a test, every rule that does not: in `crates/signal_report/src/main.rs` (`commits_walked`,
`close_moved`, `closed_too_early`, `trees_moved`, the CI rule that names a sprint's closing commit), in
`scripts/commit.sh` and `scripts/pre-commit.sh`, and in WORKING_AGREEMENT's text, which orders the separate close.

- **The work:** each rule found changed to accept the folded commit and to keep refusing what it refuses today (a
  commit between a card and the next card that is neither the sprint's work nor its close); WORKING_AGREEMENT's close
  procedure rewritten to the folded order (the gates on the working tree; the close prepared; `design_check` and
  `signal_report N --check` on the tree; one `scripts/commit.sh` commit "Sprint N: ..."; `signal_report N --check`
  on that commit; the push), each old rule's text kept with a dated note naming this card; the separate close still
  accepted for sprints through 331, whose history is pushed and is not rewritten.
- **With it, one more rule in the same close checks (reviewer-4's Low 1 at its fifth review at 331):** a sprint whose
  change adds a file under `store/binaries/` without changing `store/translation-cache/walk-fixtures-2026-10-30.json`
  is refused by name, so the walk fixture's count is never first found failing in the gates (Sprint 331); its plant,
  the rule taken out, a fixture adding a binary alone accepted.
- **Not here:** README's Status (card 334); the "multi-vCPU: not run" phrase and the affirmation check (reviewer-4's
  Low 2 and Medium 2 at 331), each a card of its own after this one.

## prerequisites

- 332 (closed)

## context_files

- `crates/signal_report/src/main.rs`: `commits_walked` (1234), `close_moved` (945), `is_close` (939),
  `is_bookkeeping` (1006), `closed_too_early` (645), `trees_moved` (1285), and every rule naming "closed" or a
  closing commit (`grep -n 'closed\|closing' crates/signal_report/src/main.rs`)
- `crates/signal_report/tests/check.rs` (its fixture repositories, the form a new test takes)
- `scripts/commit.sh`, `scripts/pre-commit.sh`, `scripts/gates.sh`, `scripts/gated-tree.sh`
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands" (the close's order, lines 247 and 256 by
  reviewer-4's count, and every line naming the close commit)
- The user decision of 2026-10-16: `sed -n 176p BLACKBOARD.md`; the Decision "A sprint's close folds into its
  sprint's commit": `grep -n "folds into its sprint" BLACKBOARD.md`

## signal contract

### Emits

- None; a change to the record's checks and the working agreement, no runtime behavior.

### Invariants

- `signal_report --check` passes on the tree as it stands (every sprint through 331, separate closes included).
- A commit between a card and the next card that is neither the sprint's work nor its close is still refused.

## artifact contract

### Files modified

- `crates/signal_report/src/main.rs` and `crates/signal_report/tests/check.rs` where a rule must change;
  `scripts/commit.sh` or `scripts/pre-commit.sh` where one refuses the folded commit; `WORKING_AGREEMENT.md` (the close
  procedure)

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:** a test in `crates/signal_report/tests/check.rs` builds a fixture repository whose last sprint is
  a card commit and one folded "Sprint N: ..." commit (its card done, its report, its tail entry headed closed, a
  gates row whose `root=` is that commit's tree less bookkeeping) and holds `signal_report N --check` to exit 0; the
  same fixture with a third commit between the card and the next card, titled neither, held to its refusal by name.
- **Planted faults:** (1) `trees_moved` taking the card commit's tree in place of the newest "Sprint N: " commit's: the
  folded fixture refused (its gates row's tree is not the card's); (2) the walk accepting any subject: the third-commit
  fixture accepted. Each judge's reach read before the card's work: the test runs `signal_report --check` against the
  fixture, which calls the planted function on the planted commit. If the folded fixture passes with no change to the
  tools, the result says so: the tools already accepted the shape, and the card's change is WORKING_AGREEMENT's text
  and the test that now holds it.

## done criteria

A sprint can close in one commit with its work, and the record check holds that shape and still refuses a stray commit.

## self-review checklist

- [x] Scope names one verifiable property: the folded close passes the record check.
- [x] context_files cite the rules, their tests, the scripts, the working agreement's lines and the two Decisions.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: a fixture held to exit 0 and a stray commit to its refusal; two plants; reach by reading.
- [x] One concept: the close folded into the sprint's commit.

## result

- **Found by test** (the folded fixture run on the unchanged tools before any change): the committed history was refused
  by name, "sprint 335: sprint 334 has no close commit to base its gates on" and "sprint 334: its CI run 11 is cited,
  and no commit \"Sprint 334 closed:\" is found to hold it to". Both came from `close_commit`, through `base_problem` and
  `ci_held`. With the second close prepared on the tree, `trees_moved` added "sprint 335: no commit \"Sprint 335: ...\"
  to compare its gates run's trees with". **Found by reading:** `scripts/gates-base.sh` (only "closed:" is a base) and
  `scripts/pre-commit.sh` (refuses zero active cards, which is the folded commit's state). `commits_walked` and
  `close_moved` already accepted the folded commit, and CI's workflow names no close.
- **The work:** `close_commit` takes, from Sprint 334 (`FIRST_FOLDED_SPRINT`), the newest "Sprint N: " commit whose tree
  holds card N done or closed-partial; `gates-base.sh` takes the same commit. `trees_moved` holds the gates' tree to the
  working tree, staged as `gated-tree.sh` stages it, while a 334+ close is not committed. `commits_walked` names a
  "Sprint N closed:" commit from 334. `pre-commit.sh` takes the sprint from the newest card when none is active and that
  card is done or closed-partial. `binaries_unwalked` covers sprints from 254, when the walk fixture began; of those
  sprints only 331 added a binary, and it changed the fixture too. WORKING_AGREEMENT gains "A sprint closes in its work
  commit", in the folded order, and dated notes on lines 144, 247, 253, 255, 256, 257 and 259.
- **Expected values:** `a_sprint_closes_in_its_work_commit`: two folded sprints exit 0, both committed and with 335's
  close prepared on the tree; a "tidy" commit is named; a separate 335 close is named. `a_binary_added_needs_its_walk_fixture_entry`:
  a binary alone is named, committed and prepared, and passes once the fixture changes. `signal_report --check` on the
  tree exits 0.
- **Plant 1** (`trees_moved` takes the card commit): the folded fixture is refused, "sprint 335: its gates ran on a tree
  whose crates its work commit 837a826f8 does not hold".
- **Plant 2** (the walk accepts any subject): the third-commit fixture is accepted, "left: Some(0) right: Some(1)".
- **Plant 3** (the binaries rule taken out): the binary alone is accepted, "left: (Some(0), \"\")" against the named
  refusal.
- **Gates:** `exit 0 in 6243 s	all	2026-11-25T21:04:53Z	sprint 333	f3ff9047eef3850c3d6c9e9baadb8a306311eefe	766896834ea3da549306d7b00e3d8692e2c362bc85aa9dd90431fb3b5bea97ee	root=9292648da27ca817a81ff7892fefd8db5f28eafc	rust=all python=0 samples=1	base=b6a7e723c803bf1a01ad4ffe900b41eab596fb9d`
  (copied from `tail -1 store/gates/passes.tsv`; --all, since `gates-base.sh` changed; 362 tests passed, 1 skipped).
