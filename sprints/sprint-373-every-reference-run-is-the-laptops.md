# Sprint 373 — Every Windows reference run is the laptop's, held by a test

```yaml
---
id: 373
status: active
phase: 6
pass_kind: functional
---
```

## scope

The postmortem's cause 6: "No gate, test or card field asks which machine a probe's rows came from"
(`docs/postmortem-windows-runs-off-the-laptop-2026-11-27.md`). The user's decision of 2026-11-27 sends every probe and
game run to the laptop; KIT_DIARY #117 records that the laptop's earlier Decision, held in prose, was undone for about
forty sprints. This card holds the rule in code, before any card compares or relies on a row (the Decision "The order
after the stop, the cards that take each deferred item").

- **The test**, in the crate the builder settles on the grounds (the store's other judges live in
  `crates/w2m_vm/tests/` and `crates/signal_report/`), walking every run in `store/probe-runs/` and `store/game-runs/`:
  - a run whose `profile.txt` starts `laptop-` passes; a laptop batch's folder, which holds `profile.json` and no
    `profile.txt` (`lap-20261013T231059422Z-136bec80`, `lap-20261127T221622950Z-62b931c2`), passes when its
    `profile.json` names `"machine": "laptop"`;
  - a GitHub runner's run made before the laptop arrived passes by the user's decision of 2026-10-11 ("the runners'
    existing rows stay on the record"): its numeric run id is at most 38311779649, the last runner run of 2026-10-12
    (21:37:24Z, `code_rewrite` on windows-2022);
  - a runner run after that passes only when a list names it by run id with its reason: the nine stored runner runs of
    2026-11-01 to 2026-11-27 (41326239822, 45760720489, 45763850660, 45780352907, 45783238565, 45797800625, 45813040124,
    45871497648, 46053401854), each "made on GitHub's runner against the Decision of 2026-10-16; its laptop re-run is
    card 374's comparison", in a list file the builder names under `store/` with a dated name;
  - any other run fails, naming the run and its profile.
- **The joined table's test** (the supervisor's rerun at Sprint 372's grade: the joined family table dropped from
  `windows_machine`'s batch lookup passed all 20 tests): a test holding that `windows_machine batch named_sync_objects`
  takes its item from `store/family_runs/joined-2026-11-27.tsv`.
- **Not here:** the comparison of laptop and runner rows (card 374), the reference set's membership rules (card 375),
  code-integrity state in `profile.ps1`.

## prerequisites

- 372 (closed)

## context_files

- `docs/postmortem-windows-runs-off-the-laptop-2026-11-27.md` (cause 6, "What the wrong machine reached").
- `crates/windows_machine/src/lib.rs` (the store's laptop refusal, `batch_items`) and `tests/windows_machine.rs`.
- `store/probe-runs/` and `store/game-runs/` (listings; one laptop run, one batch folder, one pre-laptop runner run and
  one listed runner run read in full).
- `grep -n "The order after the stop\|2026-10-11 user\|2026-11-27 user" BLACKBOARD.md`: the Decisions.
- `WORKING_AGREEMENT.md` lines 137-266.

## signal contract

### Emits

- None: a test and a list; no emit site changes.

### Invariants

- Every run as held. No run is moved or rewritten; the test only reads.

## artifact contract

### Files created

- The test file and the dated list of runner runs made after the laptop arrived.

### Files modified

- `crates/windows_machine/tests/windows_machine.rs` (the joined table's test), with others only where the class
  requires, reported to main when found. One concept: the reference machine held in code.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The count of runs the test walks (677 directories today), by kind: laptop, laptop batch, runner before the laptop,
    listed runner, and none failing.
  - The test's failure on each plant, quoted.
- **Planted faults:**
  - (1) A runner run after 2026-10-12 not on the list (one of the nine removed from it): the test fails, naming the run.
  - (2) A laptop run's `profile.txt` changed to a runner's: the test fails, naming the run.

## done criteria

A test fails on any stored Windows run made after the laptop arrived that is not the laptop's and not on a dated list
with its reason; the batch's read of the joined family table is held by a test.

## self-review checklist

- [x] Scope names one verifiable property: every stored Windows run held to the reference machine by a test.
- [x] context_files cite the postmortem, the store's refusal, the stores, the Decisions.
- [x] Emits: none, with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the walk's counts; numbered plants.
- [x] One concept: the reference machine held in code.

## result

GATES-LINE
