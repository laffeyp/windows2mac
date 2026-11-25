# Sprint 334 — Each gates step timed

```yaml
---
id: 334
status: active
phase: 6
pass_kind: functional
---
```

## scope

Every sprint waits on one `scripts/gates.sh` run, and the run has grown: 4,928 s at Sprint 331, 5,845 s at 332, 6,243 s
at 333. Sprint 332 placed part of it and left three figures unexplained (Deferred (Sprint 332), "The gates' time not
spent in `codesign -d`"): nextest's listing at about 22.5 s a test binary with one 9.85 s signature check each; about
50 minutes between nextest's end and the timing stage's output that the log does not place; and a test binary's first
launch in a series at 15 to 30 s with its signature stamp matching and no `codesign -d` run. The Decision "The gates'
stall, a card after 333" (2026-11-24) orders this card: put a time on each gates step's start and end, and measure each
of the three figures before any change. This card measures; it changes no step's work. The change the figures call for
is the next card's, written from this card's numbers.

- **The work:** a helper in `scripts/gates.sh` that stamps each step's start and end, prints "gates: step NAME N s",
  and appends one row a step to a new tracked file `store/gates/steps.tsv` (the run's UTC start, the sprint, the step's
  name, its seconds); every step of the script wrapped, from `cargo fmt --check` to the last `check_records`, the batch
  run, the guests, the family runs, nextest, the doc tests, the timing stage and each records check each a step of its
  own; at the end, one line "gates: steps N s of the run's M s, unplaced K s", printed before the pass row and kept in
  the same file as a row named `unplaced`. A step whose command exits non-zero still exits the gates as today (`set -e`);
  its end is not stamped, and the printed start names where the run stopped.
- **The three figures, each measured on this Mac with the gates not running and the result quoting the commands and
  their output:** (1) listing: one test binary's `--list` run five times through `scripts/sign_and_run.sh` with its stamp
  matching, and five times bare, each timed; then `cargo nextest list` over the workspace, timed whole, with its binary
  count. (2) the unplaced time: read from this sprint's own gates run, `store/gates/steps.tsv` naming the step or steps
  that hold it. (3) the first launch: a test binary copied fresh (new inode, signed, stamped) launched once and then
  four more times, each timed; and the same for a binary relaunched with no copy; `log show --last 5m --predicate
  'process == "syspolicyd" OR process == "XprotectService"'` read after each first launch, its count of lines quoted.
  Each figure is reported as measured, its step or cause named only where the measurement shows it ("No guesses").
- **Not here:** any change to a step's work, to the runner or to nextest's settings; the vocabulary card (0.13), which
  follows this one.

## prerequisites

- 333 (closed)

## context_files

- `scripts/gates.sh` (all 310 lines: every step to be wrapped, the pass row at 303-310)
- `scripts/sign_and_run.sh` (the runner and its stamp, Sprint 332), `.cargo/config.toml` (the runner's setting)
- `scripts/gated-tree.sh` (`store/gates/` is kept out of the gated tree, so the new file is bookkeeping)
- `crates/signal_report/src/main.rs`: `is_bookkeeping` (`grep -n 'fn is_bookkeeping' crates/signal_report/src/main.rs`)
  and every reader of `store/gates/` (`grep -n 'store/gates' crates/signal_report/src/main.rs`)
- The Deferred entry: `grep -n "The gates' time not spent" BLACKBOARD.md`; the Decision: `grep -n "The gates' stall, a
  card after 333" BLACKBOARD.md`
- Sprint 332's card result, its measurements of `codesign -d` and the runner: `sprints/sprint-332-the-runner-checks-a-binary-once.md`
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands", and its line naming "A sprint closes in its
  work commit" (this sprint is the first to close in its work commit; the builder still stops at a gated tree)
- The last three runs' totals: `tail -3 store/gates/passes.tsv | cut -f1-4`

## signal contract

### Emits

- None; a build script's timing rows, no runtime behavior.

### Invariants

- Every gate the script runs today still runs, in the same order, with the same exit behavior.
- `target/release/signal_report --check` passes on the tree.

## artifact contract

### Files modified

- `scripts/gates.sh`; `store/gates/steps.tsv` (new, tracked); `WORKING_AGREEMENT.md` (one dated note naming the file
  and the printed lines)

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:** this sprint's gates run (every stage, since `scripts/gates.sh` changes) prints a "gates: step"
  line for each step and the "steps ... unplaced K s" line, with K under 60; `store/gates/steps.tsv` holds the same rows,
  their seconds summing with `unplaced` to the pass row's "exit 0 in M s" within 2 s. The three figures as measured,
  each quoted with its command.
- **Planted fault:** (1) one step's helper call taken out (the family runs', the step Sprint 332 could not place by
  log): a gates run selected to that step's path (`GATES_CHANGED=crates/w2m_vm/src/run.rs scripts/gates.sh`, which runs
  the family runs) prints an `unplaced` figure larger by that step's seconds in the unplanted run, within 10%, and no
  "gates: step" line for it. Its reach read before the work: `GATES_CHANGED` sets the change (gates.sh 50-51), and a
  crate change sets `rust=crates`, which runs the family runs (gates.sh 140-190).

## done criteria

The time of every gates step is printed and kept, the run's unplaced time is under a minute, and each of Sprint 332's
three unexplained figures is measured and placed or stated as unplaced.

## self-review checklist

- [x] Scope names one verifiable property: every step's time is recorded and the unplaced time is under 60 s.
- [x] context_files cite the script, the runner, the bookkeeping rule, the Deferred entry, the Decision and Sprint 332's result.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the printed lines held to the kept rows and the pass row; one plant, its reach read.
- [x] One concept: the gates' time placed, before any change to it.

## result

GATES-LINE
