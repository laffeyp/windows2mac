# Sprint 372 — Batch and game jobs on the laptop: `windows_machine batch` and `windows_machine game`

```yaml
---
id: 372
status: active
phase: 6
pass_kind: bridge
---
```

## scope

Sprint 371 built `windows_machine probe` and `binary`, the Mac's half of the laptop path, held to two stored laptop
runs. This card adds the other two kinds the deleted `scripts/windows_machine.py` had, so every Windows run the
project makes goes to the laptop (the user's decision of 2026-11-27, "Every probe run and game run goes to the Windows
laptop, never a GitHub runner"):

- **`windows_machine batch <family> ...`** (the Python's `run_batch` and `batch_items`, read at the tag
  `frozen-python-2026-10-29`): one job running many stored binaries, one profile and one workspace lock for the batch
  and a fresh workspace per binary (`run-job.ps1`'s `batch` kind, unchanged); each distinct binary of each family once,
  from `store/family_runs/known-2026-10-29.tsv` and `store/family_runs/joined-2026-11-27.tsv`; the bundle split into one
  run per binary, `lap-<stamp>-<commit>-<n>`, each with `batch.txt` naming the batch, stored as Sprint 371's `store`
  step stores a run, and the batch's `profile.json`, `job.json` and `run-job.log` kept once in
  `store/probe-runs/<batch>/`, the layout `store/probe-runs/lap-20261013T231059422Z-136bec80` holds.
- **`windows_machine game [--executions N] [--seconds S] [--mode batch|play]`** (the Python's `run_game`): the
  committed `tools/jit_capture` and `games/<game>` as `tree.tar`, `run-job.ps1`'s `game` kind run in the desktop
  session with its limit in minutes as the Python set it (`executions × (seconds + 180) / 60 + 10`), each execution
  stored as `store/game-runs/<run>-<i>/` with `batch.txt` and the batch's `profile.json`, the layout
  `store/game-runs/lap-20261014T032442034Z-c8814547-01` holds. The counter options (`--pmc`, `--pmc-sample`) are
  refused: their summaries ran Python on the laptop (`pmc_summary.py`, `pmc_images.py`).
- **The runs this card makes on the laptop**, its observation:
  - A batch of the nine families whose rows were made on GitHub's runner (`docs/postmortem-windows-runs-off-the-laptop-2026-11-27.md`,
    "What the wrong machine reached"), each taken by its runner run, since eight of them have no row in either family
    table (amendment, reviewer-4's forty-sixth review, High 1): `family@run` naming the binary in
    `store/probe-runs/<run>/probe.sha256` for 41326239822, 45760720489, 45763850660, 45780352907, 45783238565,
    45797800625, 45813040124, 45871497648 and 46053401854; a test holds that the batch's list is exactly those nine
    sha256s. Each binary run twice, stored. Comparing them with the runner's rows is card 374's.
  - A control in the same batch with an October laptop run, so the batch's split and store meet a known answer
    (Medium 2): `sync_objects`, its October laptop runs `lap-20261013T231059422Z-136bec80-154` and `-155`, by `cmp`.
  - One game job under the reference set's own conditions (Medium 1): batch mode, 150 s, audio off, three executions,
    stored, as `lap-20261014T032442034Z-c8814547-01` to `-10` ran (their `exec.json` reads `"seconds": 150`); each
    execution's `unity.log` and call trace beside those ten, the build in `profile.json`. The ten ran on build
    26200.7840 and these run on 26200.9974: the difference in the game's log and trace lines is reported as the first
    measure of what the build change does (Medium 3).
- **Not here, and where each goes** (Decision 369's order, carried by this amendment and recorded as a Decision at this
  card's close; reviewer-4's forty-sixth review, High 2): card 373, the check in code that every evidence run's
  `profile.txt` is the laptop's, with its listed exceptions, which lands before any card compares or relies on a
  row; card 374, the nine families' laptop rows against their runner rows, with the Decision on which Windows w2m
  answers as, widened to say whether the laptop's build is held fixed or each comparison names both builds; card 375,
  the reference set's membership rules ported, so the laptop game runs of 2026-11-27 can join G3's comparison; a game
  run's execution on the record (`record_game_run.py`'s EXECUTION_STARTED and EXECUTION_COMPLETED) with card 375.

## prerequisites

- 371 (closed)

## context_files

- `crates/windows_machine/` (Sprint 371's program, its tests).
- `git show frozen-python-2026-10-29:scripts/windows_machine.py` (`run_batch`, `batch_items`, `run_game`, `main`),
  read in full (read only; no Python runs).
- `tools/windows_machine/run-job.ps1` (the `batch` and `game` kinds) and `run-in-desktop.ps1`, unchanged.
- `store/probe-runs/lap-20261013T231059422Z-136bec80/` and `-001/` (a stored batch), `store/game-runs/lap-20261014T032442034Z-c8814547-01/`
  (a stored game execution), `store/family_runs/known-2026-10-29.tsv` and `joined-2026-11-27.tsv`.
- `docs/postmortem-windows-runs-off-the-laptop-2026-11-27.md`; `WORKING_AGREEMENT.md` lines 137-266.

## signal contract

### Emits

- None: the controller's two kinds and their tests; no emit site changes.

### Invariants

- Every run as held; no Python file and no Python run; the laptop's scripts in `tools/windows_machine/` unchanged; no
  Windows run on GitHub's runner.

## artifact contract

### Files created

- `store/probe-runs/<batch>/` and one `store/probe-runs/<batch>-<n>/` per binary; `store/game-runs/<run>-<i>/` per game
  execution.

### Files modified

- `crates/windows_machine/` (`src/lib.rs`, `src/main.rs`, its tests), `WORKING_AGREEMENT.md` (the two commands), with
  others only where the class requires, reported to main when found. One concept: the batch and game jobs.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree
- `target/release/windows_machine batch <the nine families>` returns 0
- `target/release/windows_machine game --executions 3 --seconds 150 --mode batch --audio off` returns 0

## observation contract

- **Expected values:**
  - The Python's steps for both kinds, quoted from the frozen tag, beside the Rust that does each.
  - The batch: one run per binary, each with two executions exiting 0 and `profile.txt`
    `laptop-x64-10.0.26200.*-Client`; the control `sync_objects`'s rows equal by `cmp` to its October laptop runs.
  - The game: three executions, each `unity.log` and its line count, the capture's trace line count, against the ten
    laptop runs of 2026-10-14 (build 7840); the build in `profile.json` (9974) and the difference reported.
- **Planted faults:**
  - (1) A batch whose split drops the last binary: the program exits non-zero, naming the binary with no executions.
  - (2) A game job's limit in minutes below one execution's seconds: the desktop task stops the job and the program
    exits non-zero with no execution stored.

## done criteria

`windows_machine batch` and `windows_machine game` send the batch and game jobs to the laptop and store their results
in the store's layouts; the nine runner-made families' binaries and the game have run on the laptop and are stored.

## self-review checklist

- [x] Scope names one verifiable property: batch and game jobs sent to the laptop and stored.
- [x] context_files cite the program, the Python read at its tag, the laptop's scripts and the stored layouts.
- [x] Emits: none, with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the laptop runs and their rows; numbered plants.
- [x] One concept: the batch and game jobs.

## result

GATES-LINE
