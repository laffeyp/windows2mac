# SIGNAL_REPORT — Sprint 333 — A sprint closes in its own commit

Made by `signal_report 333` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 333.

## 1. Observed

### scope_confirmation

> The user's decision of 2026-10-16 cut "bookkeeping as separate commits (each closure folds into its sprint's commit)";
> since then every sprint has closed in a commit of its own, "Sprint N closed: its Signal Report; BLACKBOARD", 63 of them
> in step 6 alone (reviewer-4's Medium 3 at 331; the Decision "A sprint's close folds into its sprint's commit",
> 2026-11-24). This card makes the project's tools and its working agreement accept, and its record check hold, a sprint
> made of two commits: its card commit, and one commit "Sprint N: ..." that carries the work and the close together (the
> card's status done, the Signal Report, BLACKBOARD's tail entry and the gates' rows), made after `scripts/gates.sh`
> passed on the working tree. Read before this card: `signal_report`'s `trees_moved` takes the newest "Sprint N: "
> commit as the work commit and compares its tree less `BOOKKEEPING` with the gates' `root=`, and `close_moved` judges
> only a commit titled "Sprint N closed:"; so the tools may already accept the folded shape. The card's first act is to
> find, by reading and by a test, every rule that does not: in `crates/signal_report/src/main.rs` (`commits_walked`,
> `close_moved`, `closed_too_early`, `trees_moved`, the CI rule that names a sprint's closing commit), in
> `scripts/commit.sh` and `scripts/pre-commit.sh`, and in WORKING_AGREEMENT's text, which orders the separate close.

### work_performed

- 2917b3b26 Sprint 333: a sprint's close may fold into its work commit; a binary needs its walk fixture
- f3ff9047e Sprint 333 card: a sprint closes in its own commit

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a change to the record's checks and the working agreement, no runtime behavior.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 333: its Rubber Duck Pass and close.
