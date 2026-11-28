# Sprint 376 — Laptop game runs with the Steam client not running, joining G3's reference set

```yaml
---
id: 376
status: done
phase: 6
pass_kind: functional
---
```

## scope

Sprint 372's three game executions ran with the Steam client signed in on the laptop, unlike the ten reference runs
of 2026-10-14, and are not comparable (their `unity.log` replaces "[Steamworks.NET] SteamAPI_Init() failed" with the
game's Steam lines and the account's player name; kept in `build/game-runs-steam-on/`). The Decision "A laptop game
run's initial state includes the Steam client" requires the next game job to state the Steam client's state and check
it before the game starts. This card makes those runs, the first new laptop runs Sprint 375's ported rules must
admit to G3's reference set (the Decision "The order after Sprint 375").

- **The Steam client in the declared initial state.** `games/hail-to-the-rainbow/state.json`'s declared state gains
  the Steam client's processes, absent; `tools/windows_machine/run-job.ps1`'s game kind stops the Steam client for
  the job, as it stops the Windows Audio service when audio is off, checks it absent with the state's other items
  before each execution, and starts it again after the job, its start and stop in `run-job.log`.
- **The runs.** `windows_machine game --executions 3 --seconds 150 --mode batch --audio off`, as the reference runs
  ran; the laptop boots with `numproc 4` (read 2026-11-27: `bcdedit /enum {current}`), so each run reports the 4
  processors w2m reports. Each execution's `unity.log` names no Steam account (checked before it is stored); stored
  in `store/game-runs/`; put on the record by Sprint 375's `call_compare record-game-runs` (its first records).
- **The comparison.** Each execution's `unity.log` line count and the capture's trace line count beside the ten
  reference runs (build 26200.7840; these run on 26200.9974), the difference in the game's calls between the builds
  reported; `call_compare reference-set` admitting the three or naming the rule that leaves each out.
- **Not here:** the reference machine named exactly (card 377); the tables re-taken (card 378). No API id is added.

## prerequisites

- 375 (closed)

## context_files

- `tools/windows_machine/run-job.ps1` (the game kind: the audio stop and start, the initial-state check) and
  `run-in-desktop.ps1`; `games/hail-to-the-rainbow/state.json`.
- `crates/windows_machine/` (the game kind), `crates/call_compare/` (`reference-set`, `record-game-runs`).
- `store/game-runs/lap-20261014T032442034Z-c8814547-01/` (a reference run, read in full); card 372's result.
- `grep -n "A laptop game run's initial state includes the Steam client\|The order after Sprint 375" BLACKBOARD.md`.
- `WORKING_AGREEMENT.md` lines 137-266.

## signal contract

### Emits

- `EXECUTION_STARTED` and `EXECUTION_COMPLETED` for each of the three stored game executions, through Sprint 375's
  record step.

### Invariants

- Every run as held; G3's batch runs conform; no Windows run on GitHub's runner; no `unity.log` naming the user's
  Steam account enters `store/`; `crates/guest_runtime/src/api_ids.txt` unchanged.

## artifact contract

### Files created

- `store/game-runs/<run>-01/` to `-03/` and their records `store/runs/game-run-<run>-0N/`.

### Files modified

- `tools/windows_machine/run-job.ps1`, `games/hail-to-the-rainbow/state.json`, `crates/windows_machine/src/lib.rs`
  (`steam_free`: an execution whose `unity.log` speaks of Steam without "SteamAPI_Init() failed" is refused before
  anything is stored) and `crates/windows_machine/tests/windows_machine.rs`, committed by this card's amendment
  before the game job, since `windows_machine` sends the laptop only what HEAD holds (`run_game_inner`, `git archive
  HEAD`); with others only where the class requires, reported to main when found. One concept: laptop game runs under the reference's initial state.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree
- `target/release/windows_machine game --executions 3 --seconds 150 --mode batch --audio off` returns 0

## observation contract

- **Expected values:**
  - `run-job.log`'s Steam stop, the initial state proved with the Steam client absent, and its start after the job.
  - Each execution's `unity.log` with "SteamAPI_Init() failed" as in the reference runs, and its line count; the
    trace's line count; the build difference against the ten.
  - `call_compare reference-set` on the store with the three, each admitted or left out with its rule's reason; G3's
    batch runs' verdict.
- **Planted faults:**
  - (1) The Steam client left running (the stop removed): the job's initial-state check stops before the game starts,
    naming the Steam client.
  - (2) A stored execution's `profile.json` with 12 logical processors: the set's rule leaves it out with "a machine
    of 12 processors, w2m reports 4".

## done criteria

Three laptop game executions under the reference runs' initial state, the Steam client not running, are stored, on
the record and compared with the ten reference runs, each admitted to G3's set or left out by a named rule.

## self-review checklist

- [x] Scope names one verifiable property: comparable laptop game runs stored and placed by the set's rules.
- [x] context_files cite the executor, the state, the programs and a reference run.
- [x] Emits: the two record tags, for the three new runs.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the Steam state, the logs and the set; numbered plants.
- [x] One concept: laptop game runs under the reference's initial state.

## result

- **The executor and the state** (amendment 24284d8f1, committed before the job since `windows_machine` sends only
  what HEAD holds). `games/hail-to-the-rainbow/state.json`'s state `empty` gains `"steam_processes": 0`, its `about`
  naming the five processes counted: steam, steamwebhelper, steamservice, gameoverlayui, steamerrorreporter.
  `tools/windows_machine/run-job.ps1`'s game kind, when the declared state's `steam_processes` is 0, stops the Steam
  Client Service if it runs and every Steam process it counts, before the first execution, beside the Audio service's
  stop (168-188). Each execution's `$seen` reads `steam_processes` back with the state's other items (257). After the
  job it starts the service and then `steam.exe -silent` again, each only if it stopped it, with the user's TEMP and
  TMP back first. `crates/windows_machine/src/lib.rs`'s `steam_free` and `STEAM_ABSENT`: an execution whose `unity.log`
  speaks of Steam and lacks "SteamAPI_Init() failed" is refused. `store_game` refuses it, and so does `run_game_inner`'s
  check of every execution before any is stored, so the execution stays in `build/`; the refusal quotes nothing from
  the log. In the work, `crates/windows_machine/tests/windows_machine.rs` gains
  `the_game_kind_holds_every_declared_item_and_stops_the_steam_client`. It reads `run-job.ps1` and `state.json`, holds
  every declared item to the `$seen` map, and holds the Steam stop to the processes counted. There is no PowerShell on
  this Mac. At 24284d8f1, `cargo test -p windows_machine`: 23 passed in `tests/windows_machine.rs`, 29 in the crate.
- **The game's TEMP** (amendment 33b6423b3, after the first job's runs showed the game reading another TEMP than the
  reference runs'; below).
  - `run-job.ps1`'s game kind gives the game the user's own TEMP, as the job found it (`$tempDir =
    $userTemp.TrimEnd('\')`, `C:\Users\w2m\AppData\Local\Temp`). That is the path the reference runs of 2026-10-14 read
    and `w2m-vm` gives (`TEMP=C:\Users\w2m\AppData\Local\Temp`). Sprint 102's `C:\w2m\temp` is no longer used.
  - Before each execution the game kind removes the files the game makes there, the `state.json` top-level
    `temp_files_made`, `cur*.tmp` (GetTempFileNameW's prefix; the reference strings rows 3423 and 3496). It reads
    back `temp_path` and `temp_game_files` in `$seen`, and the proof records `temp_files_made`. It does not empty the
    TEMP, which the session's other programs share (Sprint 102).
  - `state.json`'s `empty` declares `"temp_path": "C:\\Users\\w2m\\AppData\\Local\\Temp"` and `"temp_game_files": 0`
    where it declared `"temp_entries": 0`.
  - `lib.rs`'s `temp_declared` refuses, in `store_game` and before any execution is stored, a run whose `state.json`
    records no TEMP path, or whose strings row after a GetTempPathW call names another folder.
  - Tests: the script test holds the game's TEMP to the declared path and to `w2m-vm.rs`'s `TEMP=`. New test
    `a_run_whose_game_read_another_temp_is_not_stored`: the reference run's strings pass with the user's TEMP, are
    refused at row 3423 against `C:\w2m\temp`, and a proof with no TEMP path is refused. The split-and-store test writes
    a proof and stores 20 files.
  - `cargo test -p windows_machine --test windows_machine`: 24 passed at HEAD 33b6423b3.
- **The files beyond the card's list, which the admitted runs required** (reported to main). Each test below reads the
  reference set, and `Windows::load` now reads fourteen runs.
  - `crates/call_compare/tests/call_compare.rs`:
    - `frozen_hosts_store` links the 44 stored game runs the frozen host saw (the eleven and `left_out()`'s 33) with
      their records, `files.json`, the contract and `specs/signatures.json` under a root of the test's own.
    - `the_rules_choose_the_frozen_hosts_reference_set` and the 24 varying values of
      `the_frozen_hosts_stream_conforms_to_windows_as_its_record_says` are held over that store, their claims unchanged.
    - New test `the_store_admits_the_three_steam_off_runs`: the eleven and `lap-20261128T014720644Z-33b6423b-01` to
      `-03` in name order, each admitted, the same 33 left out, and `Windows` citing the fourteen.
    - The stream test also holds today's set: "conforms", 0 defects, 25 varying values, and the one value below by text.
    - The record step holds the frozen host's 38 runs and the three new ones.
  - `crates/w2m_vm/tests/batch_record.rs` holds the batch run's COMPARISON_COMPLETED `reference_ref` to
    `call_compare::Windows::load(root).reference_ref()`, the rules' set today (the Decision "The reference set's
    choices"), where it was the frozen record's eleven; its synthetic tests keep the reference record's. With it,
    `crates/w2m_vm/Cargo.toml` gains the dev-dependency `call_compare`, a dev-dependency cycle (call_compare reads
    `process::PROCESSORS`), and `Cargo.lock` changes.
  - With the first job's runs in the store, `cargo test --release -p call_compare` failed three tests: "runs the rules
    let in that the frozen host left out: [lap-20261128T010500472Z-24284d8f-01, -02, -03]"; "the stored game runs on
    the record left: 41 right: 38"; "values Windows varies over the frozen host's stream left: 34 right: 24".
  - `cargo nextest run -p call_compare -p w2m_vm -p windows_machine` on the finished tree: 309 tests run, 309 passed,
    02:12:39-02:15:44 UTC.
- **The first game job, not stored.** `target/release/windows_machine game --executions 3 --seconds 150 --mode batch
  --audio off`, exit 0, 01:05:00-01:15:48 UTC, HEAD 24284d8f1: run `lap-20261128T010500472Z-24284d8f`, "3 of 3
  executions back; run-job exit 0".
  - `run-job.log`: `01:05:40.431 Steam client: 7 processes stopped (steam, SteamService, steamwebhelper), 0 running`;
    the initial state `empty` proved before each execution; `01:15:41.557 Steam Client Service: Running`;
    `01:15:41.573 Steam client started again: c:/program files (x86)/steam/steam.exe -silent`.
  - Each unity.log held "SteamAPI_Init() failed" once and no account line; 180 lines; traces 3,499, 3,498 and 3,499.
    The three were stored and put on the record, at 01:26:38-01:26:39 UTC, and the rules admitted them.
  - Their game read `C:\w2m\temp\` (strings rows 3424 and 3496), the job's own folder since Sprint 102 (cd6cac645,
    2026-10-16), where the ten of 2026-10-14 read `C:\Users\w2m\AppData\Local\Temp\` (rows 3423 and 3496). Admitted,
    they turned nine of the game's values into values Windows varies: GetTempPathW calls 0-2 ret, WideCharToMultiByte
    calls 14 and 15 (a3, a5, ret) and WriteFile call 23 a2 (0xc against 0x20, 0x2c against 0x40), all three new runs
    against all eleven. They ran under another TEMP than the reference runs' state, the class of the Steam client, so
    they were moved, untracked as they were, to `build/game-runs-job-temp/game-runs/` and their records to
    `build/game-runs-job-temp/runs/`, at 01:41:48 UTC.
  - Smart App Control refused that job's laptop-built `handler_trace.dll` in each execution. CodeIntegrity events 3033,
    3077 and 3118 at 01:07:50.116Z, 01:10:27.414Z and 01:13:05.396Z: "Code Integrity determined that a process
    (\Device\HarddiskVolume6\w2m\game\UnityCrashHandler64.exe) attempted to load
    \Device\HarddiskVolume6\w2m\game\handler_trace.dll that did not meet the Enterprise signing level requirements or
    violated code integrity policy (Policy ID:{0283ac0f-fff1-49ae-ada1-8a933130cad6})". So the five
    `jit_capture.bin.handler.*` files were absent: 15 files a run.
  - The handler did start. The game's CreateProcessW at trace line 39 returned 1. Strings rows 27-29 name its events
    and row 39 its command line, `"C:\w2m\app\UnityCrashHandler64.exe" --attach <pid> ...` (pids 2104, 520, 10076).
    The events name it as the process that attempted the load. `run-job.log` has no line about it.
- **The game job.** `target/release/windows_machine game --executions 3 --seconds 150 --mode batch --audio off`, exit
  0, 01:47:20-01:58:07 UTC, HEAD 33b6423b3: run `lap-20261128T014720644Z-33b6423b`, "3 of 3 executions back; run-job
  exit 0", the limit 26 minutes. `run-job.log`:
  - `01:47:59.272 Windows Audio service: Stopped`
  - `01:48:00.651 Steam client: 7 processes stopped (steam, SteamService, steamwebhelper), 0 running`
  - `01:50:08.434 execution 1: initial state empty proved`, then 01:52:46.813 for execution 2 and 01:55:25.190 for
    execution 3. Each run's `state.json` reads `"temp_path": "C:\Users\w2m\AppData\Local\Temp"`,
    `"temp_game_files": 0`, `"steam_processes": 0` and `"differs": []`, the game tree 297 files.
  - `01:58:01.560 Steam Client Service: Running`
  - `01:58:01.574 Steam client started again: c:/program files (x86)/steam/steam.exe -silent`

  Each execution ran 153.03-153.04 s, `exited_by_itself` false, with the arguments `-batchmode -nographics -logFile
  "C:\w2m\unity.log"`, on `laptop-x64-10.0.26200.9974-Client`, `profile.json` `cpu.logical` 4. No CodeIntegrity
  event 3033, 3077 or 3118 fell in the job's window, and each run holds the five handler files (20 files).
- **Checked before storing.** Each `unity.log` holds "[Steamworks.NET] SteamAPI_Init() failed" once, and none of Sprint
  372's "STEAMWORKS!!!" or "Final Player Name" lines. Each game read `C:\Users\w2m\AppData\Local\Temp\`: rows 3422 and
  3495, 3419 and 3492, 3425 and 3496. `steam_free` and `temp_declared` passed each before `store_game` stored it, as
  `store/game-runs/lap-20261128T014720644Z-33b6423b-01` to `-03`.
- **Against the ten reference runs** (`lap-20261014T032442034Z-c8814547-01` to `-10`, build 26200.7840):
  - `unity.log`: 180 lines in each new execution, 180 in each of the ten. The non-blank lines the new logs hold and
    `-01`'s lacks are five timings each ("- Completed reload", two "UnloadTime", two "Total").
  - The call trace: 3,498, 3,495 and 3,499 lines, against 3,489 to 3,501 across the ten. Sprint 372's Steam-on
    executions had 3,612 to 3,616.
  - The handler's trace: `traced_calls` 411 in each new run, 411 in nine of the ten and 417 in `-03`.
  - The build difference, 7840 to 9974, by caller image, DLL and function. In the trace, the first 32 calls of each
    function, no function's count in all three new runs lies outside the ten's range. In the whole-run counts
    (`jit_capture.bin.imports.tsv`, 843 rows in each run), one row lies outside it in all three: UnityPlayer.dll's
    WaitForSingleObjectEx, 326,757 to 329,435, against 330,520 to 361,510. The first job's other three such rows
    (CloseHandle, QueryPerformanceCounter, VirtualFree) are within the ten's range or not in all three here.
- **On the record.** `target/release/call_compare record-game-runs .`, exit 0, 02:08:48-02:08:49 UTC: "3 stored game
  runs to put on the record; the rules read the record of 11, each on it". It wrote
  `store/runs/game-run-lap-20261128T014720644Z-33b6423b-01`, `-02` and `-03`, each SESSION_INIT, EXECUTION_STARTED,
  EXECUTION_COMPLETED and SESSION_COMPLETED. Its tree had every tracked file equal to HEAD: the six other changed files
  (`call_compare.rs`, `w2m_vm/Cargo.toml`, `batch_record.rs`, `Cargo.lock`, this card, `store/gates/steps.tsv`) were
  copied to `scratchpad/hold-376/`, given HEAD's text by `git show HEAD:<path>`, and restored by `cp` after, each `cmp`
  equal. A second run: "0 stored game runs to put on the record; the rules read the record of 14, each on it".
- **The reference set.** `target/release/call_compare reference-set .`, exit 0, admits the three: "reference set
  38216081440, lap-20261014T032442034Z-c8814547-01, ... -10, lap-20261128T014720644Z-33b6423b-01,
  lap-20261128T014720644Z-33b6423b-02, lap-20261128T014720644Z-33b6423b-03", 14 runs. The 33 runs it leaves out are
  left out for the reasons they were before.
- **What the three change in G3's judge.** The frozen host's stream of 2026-10-21 still "conforms", with 0 defects,
  against the fourteen runs. 25 of its values vary, against 24 over the eleven. The one value that varies over the
  fourteen and not over the eleven (`call_compare::SetDivergence::text`):

  | value | Windows | w2m | disagree | seq |
  |---|---|---|---|---|
  | UnityPlayer.dll!SwitchToThread call 31 ret | 0x1 | 0x0 | `33b6423b-01` alone, 13 agree | 3074 in `-01` |

  It differs among the three 26200.9974 runs themselves (`-01` returns 1; `-02`, `-03` and the eleven 0): Windows'
  run-to-run variance, not a build difference.
- **Plant 1** "the game kind does not stop the Steam client's processes it counts:", `cargo test -p windows_machine
  --test windows_machine the_game_kind_holds` exit 101; live 01:28:38-01:28:46 UTC:
  - judged locally, as main directed, so that no faulty script is committed: `windows_machine` sends the laptop only
    what HEAD holds, so the laptop variant would need the fault committed;
  - backup `scratchpad/plants/376/run-job.ps1.backup`;
  - the stop removed by Edit (the four lines `$running | ForEach-Object { ... Stop-Process -Id $_.Id -Force ... }`); a
    grep then found 0 such `Stop-Process` lines;
  - the test failed as quoted, at `tests/windows_machine.rs:765` (log `scratchpad/plants/376/plant1.log`);
  - restored by `cp`; `cmp` equal;
  - run on 24284d8f1's script. The Steam stop it removed is the same in 33b6423b3: `git diff 24284d8f1 HEAD`
    changes no line of it, only two comments that name Steam.
- **Plant 2** "run lap-20261128T014720644Z-33b6423b-02 not in the set: a machine of 12 processors, w2m reports 4",
  `target/release/call_compare reference-set .` exit 0, the set then 13 runs; live 02:10:21-02:10:32 UTC:
  - backup `scratchpad/plants/376/profile.json.backup2`;
  - `store/game-runs/lap-20261128T014720644Z-33b6423b-02/profile.json` `cpu.logical` changed from 4 to 12 by Edit;
    the grep found `"logical": 12,` at line 19;
  - the set's line as quoted (log `scratchpad/plants/376/plant2b.log`);
  - restored by `cp`; `cmp` equal;
  - a first run on the first job's `-02`, 01:28:52-01:29:02 UTC, gave the same line for that run.
- **The laptop after the jobs:** the Steam client and its service running again, as before them; job folders left
  `C:\w2m\jobs\lap-20261014T000913357Z-08872d13` and `lap-20261017T030501303Z-d8d187af`, from October.
- **Gates stopped by hand.** A first gates run at about 01:31 UTC was stopped before any step row, when the frozen-set
  test was found failing. A second, from 01:40 UTC, was stopped at its batch-run step at about 01:41 UTC. Its rows,
  `cargo-clippy`, `linux-check` and `runner-binaries-build` under 2026-11-28T01:40:39Z, are kept in
  `store/gates/steps.tsv`. Its batch run (substrate, `w2m-vm` and its child, writing
  `build/batch_run/record-1795830055000000000`) was left running and was killed.
- **G3 in these gates.** Both batch runs' COMPARISON_COMPLETED: verdict "conforms", `first_divergence` null. Their
  `reference_ref` names the fourteen: "game-run-38216081440 + game-run-lap-20261014T032442034Z-c8814547-01 + ... +
  game-run-lap-20261014T032442034Z-c8814547-10 + game-run-lap-20261128T014720644Z-33b6423b-01 +
  game-run-lap-20261128T014720644Z-33b6423b-02 + game-run-lap-20261128T014720644Z-33b6423b-03". The records are
  `build/batch_run/record-1795832183000000000` and `record-2-vcpus-1795832282000000000`.
- **G3's two-vCPU batch run in these gates:** `run_seconds` 133.761917125 (variant vm,
  `build/batch_run/record-2-vcpus-1795832282000000000`); `load_at_start` "load averages 2.40 3.34 2.97; busy:
  mds_stores 98%; other w2m: none".
- **The new records under the record check:** "T4 applied 3 violations 3" ("0 ingests of sha256:..."), "in records
  made on or after 2026-10-17: 0 held, 0 known". This is the frozen host's own game-run records' result
  (`check_records` on `game-run-lap-20261014T032442034Z-c8814547-01`: T4 1, violation 1, 0 held). A game run's
  EXECUTION_COMPLETED names its capture's trace, which nothing ingests.
- **Gates:** `exit 0 in 622 s	default	2026-11-28T02:26:25Z	sprint 376	33b6423b37639efcb4fd724823defaad83b4dffd	7dc94169e49b316228238b9af6e93847a3a8057fa000b8282eeb6b5e894f5478	root=10bd4102e573dc85af962cb36dffa70f8a9a10b5	rust=all python=0 samples=0	base=70fc4dccf60656c8e61d434911d3092156ed592f`
  (copied from `tail -1 store/gates/passes.tsv`).
- **Gates at the close:** `exit 0 in 601 s	default	2026-11-28T03:04:37Z	sprint 376	33b6423b37639efcb4fd724823defaad83b4dffd	6dd26f94de87c1f7969096aaece132ae49bba879d511f83b70d911604a7b55f5	root=d772763262ef003ea75077dfcedc5c553b44c531	rust=all python=0 samples=0	base=70fc4dccf60656c8e61d434911d3092156ed592f`,
  after `crates/signal_report/src/main.rs` named 24284d8f1 and 33b6423b3 in `BETWEEN_SPRINTS`.
