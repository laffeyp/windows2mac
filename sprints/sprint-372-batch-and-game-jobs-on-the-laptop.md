# Sprint 372 — Batch and game jobs on the laptop: `windows_machine batch` and `windows_machine game`

```yaml
---
id: 372
status: done
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
in the store's layouts; the nine runner-made families' binaries have run on the laptop, each stored or recorded as refused by the laptop's code-integrity policy with its event, a refused binary rebuilt on the laptop and stored or recorded refused again; the game has run on the laptop, its runs stored when comparable with the reference set and otherwise kept outside `store/` with the cause recorded (amended at the close: Smart App Control refused two binaries, and the game ran with Steam signed in).

## self-review checklist

- [x] Scope names one verifiable property: batch and game jobs sent to the laptop and stored.
- [x] context_files cite the program, the Python read at its tag, the laptop's scripts and the stored layouts.
- [x] Emits: none, with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the laptop runs and their rows; numbered plants.
- [x] One concept: the batch and game jobs.

## result

- **The program.** `crates/windows_machine/src/lib.rs` 620-1328, `src/main.rs` 154-255, `tests/windows_machine.rs`
  (20 tests, all passing). `windows_machine batch [<family> | <family>@<run> ...] [--repeat N] [--pin ...] [--window
  hidden|minimized|normal]` and `windows_machine game [--game G] [--exe E] [--executions N] [--seconds S] [--mode
  batch|play] [--audio on|off] [--input NAME] [--record] [--person] [--capture on|off] [--initial-state S]
  [--plant-state]`; `--pmc` and `--pmc-sample` refused, exit 2. Each step is the frozen Python's
  (`scripts/windows_machine.py` at frozen-python-2026-10-29):
  - `batch_items` 150-169 (one item per distinct binary of each family, a repeated binary's run appended to `runs`,
    the DLL's name from `probe.json`) is `batch_items` (lib.rs 751), reading `KNOWN` then `JOINED`, a row of both
    counted once. `<family>@<run>` (`run_item`, lib.rs 820) takes the binary `store/probe-runs/<run>/probe.sha256`
    names, for the eight families neither table holds (card amendment 62b931c25).
  - `run_batch` 173-200 (the lock, HEAD under it, the stamp, `execution_id` `<family>-<stamp>-<n:03>`, the spec,
    `tree.tar` of `-- probes`, `bin/<sha256>.exe|.dll` once each) is `begin` (906), `run_batch_inner` (986),
    `batch_inputs` (848) and `Batch::json` (879) in the Python's `json.dumps(spec, indent=1)` form (`Py`, 630:
    `ensure_ascii`'s escapes, one space a level).
  - `run_batch` 201-209 (scp, `run-in-desktop.ps1` by ssh, `out/` to `build/batches/<run>/`, `job.json` beside it, the
    laptop's folder removed) is `send` (936), shared with the game job.
  - `run_batch` 213-228 (each item's folder to `build/probe-runs/<run>-<n:03>/traces-<execution>/`, `batch.txt`, the
    verdict, the short ones named, `code or (1 if short else 0)`) is `split_batch` (1082). Its line names the binary's
    sha256 as well as the family and first run.
  - `store_probe_runs.py`'s `KEPT` with `batch.txt` and `BATCH_FILES` once in `store/probe-runs/<batch>/` is `store`
    (527), which checks the batch's three files are there before writing anything.
  - `run_game` 235-272 (`limit = executions * (seconds + 180) // 60 + 10`, the spec in the Python's key order with
    `pmc` and `pmc_sample` false, `tree.tar` of `-- tools/jit_capture games/<game>`, ssh bounded by `limit*60 + 600`)
    is `Game::limit_minutes` and `Game::json` (1148) and `run_game_inner` (1224).
  - `run_game` 276-289 (`<i>/` to `build/game-runs/<run>-<i:02>/` with `batch.txt` and the job's `profile.json`, an
    execution without `exec.json` named and skipped) is `split_game` (1286), and `store_game` (1310) copies each
    execution to `store/game-runs/<run>-<i:02>/`, refusing a profile not the laptop's.
- **Held to the record:** the October batch's `job.json`, 195 items and 75,329 bytes, is the program's byte for byte.
  `batch_items` for late_tls and sync_objects gives the October batch's four items, DLLs included. The batch's list is
  the nine runner runs' `probe.sha256` binaries and the control's two, and `named_sync_objects` by its table row is the
  same item as by its run. The October runs -154 and -155, split from a bundle and stored, keep their 8 files equal,
  `batch.txt` excepted. The frozen Python's `write_text(run)` has no newline; the October file has one. A game bundle
  of `c8814547-01` splits and stores 19 files equal; a runner's profile is refused.
- **The batch on the laptop.** `target/release/windows_machine batch locked_race@41326239822
  thread_values_race@45760720489 thread_slots_race@45763850660 tls_free_reach@45780352907
  tls_expansion_first@45783238565 wait_closed_handle@45797800625 wait_closed_kinds@45813040124
  mutex_abandoned@45871497648 named_sync_objects sync_objects` ran 22:16:22-22:17:31 UTC, 69 s. It made run
  `lap-20261127T221622950Z-62b931c2` (HEAD 62b931c25), 11 binaries, all on `laptop-x64-10.0.26200.9974-Client`, and
  exited **1**: "11 binaries, 9 with 2 executions; run-job exit 0". `-003` thread_slots_race (1c778788...) and `-006`
  wait_closed_handle (f0ae78e5...) each have execution 1 with exit 4551, "'D:\a\windows2mac\windows2mac\probe.exe' was
  blocked by your organization's Device Guard policy". The laptop's CodeIntegrity log (events 3077 and 3033, read by
  main) shows Smart App Control refusing the two unsigned binaries: at 22:17:12.611Z and 22:17:19.576Z, events 3118
  "Smart App Control Block Deteails" [sic], 3077 "Code Integrity determined that a process
  (\Device\HarddiskVolume3\Windows\System32\cmd.exe) attempted to load
  \Device\HarddiskVolume6\a\windows2mac\windows2mac\probe.exe that did not meet the Enterprise signing level
  requirements or violated code integrity policy (Policy ID:{0283ac0f-fff1-49ae-ada1-8a933130cad6})", and 3033.
  Smart App Control stays on (`VerifiedAndReputablePolicyState` 1). On main's word the nine whole runs were stored
  one by one with `windows_machine store <run>` (`-001`, `-002`, `-004`, `-005`, `-007`, `-008`, `-009`, and the
  control's `-010`, `-011`), 8 files each, with the batch's `profile.json`, `job.json` and `run-job.log` once in
  `store/probe-runs/lap-20261127T221622950Z-62b931c2/`. `-003` and `-006` are recorded as blocked and are not stored;
  their bundles stay in `build/`.
  - The control: `-010` and `-011` (sync_objects a564e428... and 530e281c...) `body-1.jsonl` and `body-2.jsonl` each
    equal `store/probe-runs/lap-20261013T231059422Z-136bec80-154` and `-155`'s by `cmp`, 1,558 bytes each, build 9974
    against October's 7840.
- **The game job.** `target/release/windows_machine game --executions 3 --seconds 150 --mode batch --audio off`, exit
  0, 22:38:51-22:49:55 UTC: run `lap-20261127T223851657Z-62b931c2`, "3 of 3 executions back; run-job exit 0", the
  limit 26 minutes. The laptop-built launcher was not blocked. Each execution ran 153.03 s, `exited_by_itself` false,
  on build 26200.9974 (`profile.json` `ubr`); the game tree was verified, 297 files, and the initial state `empty`
  proved before each. The card's first 180 s was corrected to the reference set's conditions: its runs (`c8814547-01`
  to `-10`) read `"seconds": 150, "audio": "off", "mode": "batch"` in `exec.json`, and the comparison is against them.
  - Lines against the ten runs of 2026-10-14 (build 26200.7840): `unity.log` 193 in each execution, against 180 in each
    reference run; the call trace (`jit_capture.bin.trace.tsv`) 3,616, 3,615 and 3,612, against 3,489 to 3,501.
  - **Not comparable with the reference set.** The Steam client was running and signed in on the laptop. October's
    "[Steamworks.NET] SteamAPI_Init() failed" is gone from `unity.log`, and "STEAMWORKS!!!" lines naming
    "GameApp 1699230", "[SteamDeckCheck] Is Steam Deck: False" and the signed-in account's player name stand in its
    place. So the 13 log lines and about 115 trace lines measure Steam's presence together with the build change, not
    the build alone. The declared initial state (`games/hail-to-the-rainbow/state.json`, `run-job.ps1`'s checks) does not
    cover the Steam client. The game is not run again in this sprint.
  - The three executions name the user's Steam account in `unity.log`, so they are not in `store/`. They are kept,
    untracked, in `build/game-runs-steam-on/lap-20261127T223851657Z-62b931c2-01` to `-03`, with the bundle in
    `build/batches/lap-20261127T223851657Z-62b931c2/`.
- **The two blocked families, built on the laptop** (`windows_machine probe`, Sprint 371's kind, from the committed
  source at HEAD 62b931c25):
  - `probe thread_slots_race`, exit 1, 22:50:54-22:51:43 UTC, run `lap-20261127T225054359Z-62b931c2`: "run-job exit 1,
    1 of 2 executions". Execution 1 exit 4551, "'D:\a\windows2mac\windows2mac\probe.exe' was blocked by your
    organization's Device Guard policy"; CodeIntegrity 3077 at 22:51:36.677Z, the batch's text. The rebuild's sha256
    45fb85ac70707199b709418d6d74316b7d60bb5f18a890a13636465604bdc14b differs from the runner binary's
    1c778788fa460b630e132042801ff2f03511102e618837d25e613e5d951a5cc6. Not stored; the family waits on the user's answer
    on Smart App Control.
  - `probe wait_closed_handle`, exit 0, 22:51:50-22:52:39 UTC, run `lap-20261127T225150596Z-62b931c2`: "2 of 2
    executions", both exit 0, stored, 10 files, profile `laptop-x64-10.0.26200.9974-Client`. Its binary
    21c64b309941d27ed5c1357de2d4e8afa0119da2b5f1ec7582d0e37746574aa1 differs from the runner binary's
    f0ae78e5b96405f18e8a3dc0666dba2d8e085f2832df418202486dcdc35e96e1, so card 374's comparison for this family is
    between two builds of one source. A note for card 374: its `body-1.jsonl` and `body-2.jsonl` equal runner run
    45797800625's `body-1.jsonl` by `cmp`, 2 rows. The binary is the store's 291st image: its walk counts (293
    instructions, 69 block starts) were appended to `store/translation-cache/walk-fixtures-2026-10-30.json`, and
    `crates/image_walk/tests/fixtures.rs`'s count was raised to 291 images and 648,036 instructions.
- **Plant 1** "short binaries: ["lap-b-002 sync_objects (38051400336, binary
  530e281c3c7800ab121fa13cf1ed9f1d15e95bbef4bb7e806ff04bd961c4482c): 0 of 2 executions"]",
  `cargo test -p windows_machine --test windows_machine a_batch_is_split_and_stored` exit 101; live 22:22:17-22:22:18
  UTC (the program on a laptop batch is held with the laptop's jobs; this test is the replacement judge):
  - backup `scratchpad/plants/372/lib.rs.backup`;
  - `split_batch`'s copy changed by Edit to `if src.is_dir() && n + 1 < items.len()`, so the last binary's folder is
    not copied; the grep found it at line 1097;
  - the test failed naming the second binary with no executions (log `scratchpad/plants/372/plant1b.log`); a first
    run at 22:21:40-22:21:52 failed on the statuses alone, `[("lap-b-001", 0), ("lap-b-002", 1)]`, and the test's
    assertion was made to print the line;
  - restored by `cp`; `cmp` equal.
- **Plant 2** "assertion `left == right` failed: 3 x 150 s ... left: 2 right: 26" and "assertion `left == right`
  failed: 3 x 150 s: limit 2 minutes left: Err("the job's limit, 2 minutes, is below one execution's 150 s: the
  desktop task would stop it before its first execution ends") right: Ok(2)", `cargo test -p windows_machine --test
  windows_machine -- the_game_limit_is_the_pythons a_limit_below_one_execution_is_refused` exit 101; live
  22:56:05-22:56:06 UTC:
  - the laptop variant was not run. Its fault, the same Edit, was live 22:53:31-22:53:50 UTC. Its judge, `windows_machine
    game --executions 1 --seconds 150 --mode batch` on the laptop with audio on (a stopped job then leaves the
    Audio service running), was refused by the session's permission check at 22:53 UTC, before anything was built or
    sent; restored by `cp`, `cmp` equal;
  - the program now refuses such a limit before any job is sent (`Game::checked_limit`, called first in
    `run_game_inner`), and the two tests hold the limit to the Python's formula (26 minutes for 3 × 150 s) and to that
    refusal. Backup `scratchpad/plants/372/lib.rs.backup2`;
  - `limit_minutes`' body changed by Edit to `2`; the grep found it at line 1152;
  - both tests failed as quoted (log `scratchpad/plants/372/plant2-local.log`). A first attempt at 22:55:48 UTC
    ran no test: cargo refused two test names before `--`;
  - restored by `cp`; `cmp` equal; the release binary rebuilt.
- **Laptop job folders left as they were:** `C:\w2m\jobs\lap-20261014T000913357Z-08872d13` and
  `lap-20261017T030501303Z-d8d187af`, from October.
- **G3's two-vCPU batch run in these gates:** 129.37 s to the stop (`run_seconds` 129.3664901,
  `build/batch_run/record-2-vcpus-1795820322000000000`, `load_at_start` "load averages 1.90 1.67 1.55; busy: mds_stores
  82%; other w2m: none").
- **Gates:** `exit 0 in 607 s	default	2026-11-27T23:06:59Z	sprint 372	62b931c2506dc7cc245f8731416b78fcc75d2ec2	a9aa1f0069f2a994859c793cb7fde5d67df82616c0a2707b6983cd92bdff8ff5	root=b2713289b9104bb52b695d23c8d8313a9eabc51b	rust=all python=0 samples=0	base=221b06efbe3fd908f6dd6a8832981b0c88c41656` (copied from `tail -1 store/gates/passes.tsv`).
