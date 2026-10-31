# Sprint 273 — The gates judge the sprint's change

```yaml
---
id: 273
status: active
phase: 6
pass_kind: functional
---
```

## scope

reviewer-3's High 1 at 272, found in Sprint 272 first (its Observation 5; KIT_DIARY #111): `scripts/gates.sh` takes
the change as the paths that differ from `git merge-base HEAD origin/main` (45-46), and from Sprint 267 each work
commit was pushed before its gates, so the passes of 267 to 272 saw only the uncommitted bookkeeping and ran
`signal_report`'s dependants; 272's first pass, at 05ae6da85, passed a change the full gates then failed three times.
This card makes the change the sprint's own: every path that differs from the previous sprint's close commit, pushed
or not, and holds a closed entry's pass row to that base, so a pass cannot again be read from an empty change.

- **Where it stands, by lookup (2026-10-31):**
  - `gates.sh` 43-52 the change and its first line; 59-81 the selection from it; 296-300 the pass row, ten columns,
    no base among them; `GATES_CHANGED` stands in for the change in the selection's plants.
  - `signal_report --check` (`crates/signal_report/src/main.rs` 700-745) finds a closed entry's pass row by its "exit 0
    in N s" and sprint, and holds its trees (`trees_moved`), the close (`close_moved`) and the stages it ran; it
    finds a sprint's close commit (`close_commit`).
  - WORKING_AGREEMENT 254 orders the record check before the push for a close and says nothing of a work commit.
- **The work:**
  - `gates.sh`: the base is the newest commit, an ancestor of `HEAD`, whose subject opens "Sprint N closed:"; with
    none, the merge base with `origin/main` as now. The first line names the base and its subject; the pass row gains
    an eleventh column, `base=<commit>`.
  - `signal_report --check`: from Sprint 273, a closed entry's pass row names a base, and it is the previous sprint's
    close commit; a row with no base or another is named.
  - WORKING_AGREEMENT: the change is the sprint's whatever was pushed, so a push before the gates loses nothing; the
    order rule stated once.
  - Tests: the base found on a history with a close and without one, by `gates.sh --plan` in a scratch repository; the
    check refusing a row whose base is another commit and one with none.
- **Not here:** two vCPUs (274).

## prerequisites

- 272 (closed)

## context_files

- `scripts/gates.sh`; `crates/signal_report/src/main.rs` (the closed entry's pass row, `close_commit`)
- WORKING_AGREEMENT 254; KIT_DIARY #111; BLACKBOARD's Drift watchlist entry of 2026-10-31

## signal contract

### Emits

- None; the gates' selection and its record, held by tests.

### Invariants

- A gates run selects at least what it did before for the same uncommitted change.

## artifact contract

### Files modified

- `scripts/gates.sh`, `crates/signal_report/src/main.rs`, WORKING_AGREEMENT.md, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** this card's own gates, run after its work commit is pushed, name Sprint 272's close as their
  base and select the crates this card changed, `signal_report` and every crate a `scripts/` change runs.
- **Planted faults:**
  - (1) the base taken from `origin/main` again: the base test names the work commit as the base.
  - (2) the check's base test turned off: its unit test names a row whose base is another commit as passing.

## done criteria

The gates select from the sprint's whole change whether or not it was pushed, their row records the base, and the
record check refuses a closed entry whose pass was selected from any other.

## self-review checklist

- [x] Scope names one verifiable property: the gates' change is the sprint's.
- [x] context_files cite the gates, the check and the agreement.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: the gates' base.

## result
