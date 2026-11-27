# Sprint 373 — Every Windows reference run is the laptop's, held by a test

```yaml
---
id: 373
status: done
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

- **The walk.** `crates/windows_machine/tests/reference_machine.rs` (new, 183 lines), test
  `every_stored_windows_run_is_the_laptops_or_on_the_list` (118). `machine` (63) holds each folder of
  `store/probe-runs/` and `store/game-runs/` to one of four kinds: `profile.txt` starting `laptop-`; a laptop batch's
  folder, no `profile.txt` and `profile.json` naming `"machine": "laptop"`; a `gha-` profile under a numeric run id at
  most 38311779649 (`LAST_RUNNER_BEFORE_THE_LAPTOP`, 17); a `gha-` profile above it named in the list (`LISTED`, 19).
  A `profile.json` beside a `profile.txt` must name the laptop as the text does, as every one of the 43 stored does.
  Anything else fails, the message naming `<store>/<run>` and its profile, every failure listed at once. The list's
  rows must each name a stored runner run above the cut, each once with a reason, and the runner's runs before the
  laptop are held at 419 (`RUNNER_RUNS_BEFORE_THE_LAPTOP`, 24): a closed set, since no later runner run has a smaller
  id and no stored run is removed.
- **The list.** `store/reference_machine/runner-runs-after-the-laptop-2026-11-27.tsv`: run, family, the sprint whose
  commit stored it (from `git log --diff-filter=A` of each `profile.txt`), and the reason "made on GitHub's runner
  against the Decision of 2026-10-16; its laptop re-run is card 374's comparison", for the nine runs 41326239822
  (locked_race, 331), 45760720489 (344), 45763850660 (345), 45780352907 and 45783238565 (347), 45797800625 (350),
  45813040124 (352), 45871497648 (356) and 46053401854 (369), all `gha-windows-2025-x64-10.0.26100.*-Server`.
- **The walk's counts** (`cargo test -p windows_machine --test reference_machine -- --nocapture`, exit 0): "walked 677
  runs: laptop 247, laptop batch 2, runner before the laptop 419, listed runner 9, failing 0; not a run:
  store/probe-runs/semantics-037-cases.bin". By store: `probe-runs` 633 folders, 215 `lap-` (213 laptop runs and the
  batch folders `lap-20261013T231059422Z-136bec80` and `lap-20261127T221622950Z-62b931c2`) and 418 numeric (409 before
  the laptop, the nine listed); `game-runs` 44, 34 `lap-` and 10 numeric, every one at most 38275716702. The 428
  numeric runs carry `gha-windows-2022` (190) or `gha-windows-2025` (238) profiles; no run lacks both profile files.
- **The joined table's test.** `crates/windows_machine/tests/windows_machine.rs` 402-451,
  `the_batch_reads_the_joined_table`: in a scratch root whose known table is the tree's with the joined table's
  families' rows taken out, `batch_items(["named_sync_objects"])` is the tree's one item, the joined row's run
  46053401854 and binary 446f77bc..., no DLL; with the joined table cut to its header the family is "name no family
  named_sync_objects". Held against Sprint 372's grade fault: `batch_items`' `for table in [KNOWN, JOINED]` changed by
  Edit to `[KNOWN]` (lib.rs 753, live 23:20:20-23:20:29 UTC, backup `scratchpad/plants/373/lib.rs.backup`), `cargo
  test -p windows_machine --test windows_machine` exit 101, 20 passed and this one failed: "called `Result::unwrap()`
  on an `Err` value: "store/family_runs/known-2026-10-29.tsv and store/family_runs/joined-2026-11-27.tsv name no
  family named_sync_objects; name one of its stored runs as named_sync_objects@<run>"" (log
  `scratchpad/plants/373/joined-check.log`); restored by `cp`, `cmp` equal. `windows_machine.rs` 21 tests pass.
- **Plant 1** "1 stored runs not the laptop's and not on the record: store/probe-runs/45871497648: profile
  gha-windows-2025-x64-10.0.26100.33828-Server: a GitHub runner's run made after the laptop arrived (id above
  38311779649) and not named in store/reference_machine/runner-runs-after-the-laptop-2026-11-27.tsv", `cargo test -p
  windows_machine --test reference_machine` exit 101; live 23:20:35-23:20:41 UTC:
  - backup `scratchpad/plants/373/runner-runs.tsv.backup`;
  - the row of 45871497648 (mutex_abandoned) removed by Edit; `grep -c 45871497648` on the list gave 0;
  - the test failed as quoted, its count line "listed runner 8, failing 1" (log `scratchpad/plants/373/plant1.log`);
  - restored by `cp`; `cmp` equal.
- **Plant 2** "1 stored runs not the laptop's and not on the record:
  store/probe-runs/lap-20261013T231059422Z-136bec80-001: profile gha-windows-2025-x64-10.0.26100.33828-Server: neither
  the laptop's (laptop-...) nor a GitHub runner's (gha-...) under its numeric run id", `cargo test -p windows_machine
  --test reference_machine` exit 101; live 23:20:53-23:21:04 UTC:
  - backup `scratchpad/plants/373/profile.txt.backup` of the stored run's `profile.txt`
    (`laptop-x64-10.0.26200.7840-Client`, 33 bytes, no newline), a laptop run with no `profile.json`;
  - its text changed by Edit to `gha-windows-2025-x64-10.0.26100.33828-Server`; `cat` showed it;
  - the test failed as quoted, "laptop 246, ... failing 1" (log `scratchpad/plants/373/plant2.log`);
  - restored by `cp`; `cmp` equal; `git status --short store/probe-runs` empty, the store as before.
- **No laptop run, nothing sent** to the laptop or to GitHub's runner.
- **G3's two-vCPU batch run in these gates:** 131.78 s to the stop (`run_seconds` 131.777147752,
  `build/batch_run/record-2-vcpus-1795821845000000000`, `load_at_start` "load averages 3.46 2.63 2.55; busy: Python
  94%, mds_stores 92%; other w2m: none").
- **Gates:** `exit 0 in 625 s	default	2026-11-27T23:32:37Z	sprint 373	95d2bf6f1a88df2774717a0fbe69117fc04519d3	5b884fd6654d60e7a2ff68473d1d6da7198236e9e343f571f19a8dbfcdfcab9a	root=1a6ac90305aaeb36c55e6ae5b52a88b7d57e7501	rust=all python=0 samples=0	base=36a4b5e4fb0bec9304a00f041955b322d18602b1` (copied from `tail -1 store/gates/passes.tsv`).
