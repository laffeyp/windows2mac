# Sprint 377 — The reference machine named exactly, and the runner's Windows workflows retired

```yaml
---
id: 377
status: done
phase: 6
pass_kind: functional
---
```

## scope

The laptop is the reference machine (user decision 2026-11-27: "Every probe run and game run goes to the Windows
laptop, never a GitHub runner"), but the code names it loosely, and the runner path stays open:

- **A run is the laptop's by a prefix.** `windows_machine`'s store steps (`crates/windows_machine/src/lib.rs` 546, 1049,
  1269, 1382) and the reference walk (`crates/windows_machine/tests/reference_machine.rs` 33, 82, 96) accept any
  `profile.txt` starting `laptop-`. They take one pattern instead, `laptop-x64-10.0.26200.<revision>-Client`, the
  laptop's machine, CPU class and Windows build with the revision free (7840 in October, 9974 today); a profile
  outside it is refused with the profile quoted.
- **The machine's code-integrity state is not in its profile.** Smart App Control blocked probe binaries from Sprint
  372 and one build of `handler_trace.dll` in Sprint 376's first game job (CodeIntegrity 3077), and let the next build
  load. `tools/windows_machine/profile.ps1` records the state each run met
  (`HKLM:\SYSTEM\CurrentControlSet\Control\CI\Policy` `VerifiedAndReputablePolicyState`), and each run's
  `profile.json` carries it; a probe or game job whose CodeIntegrity log holds a block during its window names the
  blocked file in its result instead of reporting a missing output as the program's.
- **The host key sits under an old address.** `LINK` (`lib.rs` 47-56) passes `HostKeyAlias=192.168.4.55`, the laptop's
  first address; the key moves under the alias `w2m-laptop` (`~/.ssh/known_hosts`, the alias line written by
  `ssh-keygen -R`/`ssh-keyscan` from the key already held, compared by fingerprint), and `LINK` names that.
- **GitHub's Windows workflows are still active.** `.github/workflows/windows-probe.yml` and `windows-game.yml` are
  retired by a move to `tools/retired-workflows/`, outside the folder GitHub reads, so no file is deleted
  (sdd-kit-3/AGENTS.md: never "Delete files"). Moving them off main's `.github/workflows/` closes their dispatch from any ref: "This event will
  only trigger a workflow run if the workflow file exists on the default branch" (GitHub's events reference,
  docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows, read 2026-11-27); a test reads `.github/workflows/` and fails on any workflow but `ci.yml`
  whose `runs-on` names Windows.
- **The store's TEMP check reads the proof, not the declaration.** `temp_declared` (`lib.rs` 1333-1370) compares the
  game's TEMP with the proof's `seen.temp_path`; it compares with `games/<game>/state.json`'s declared `temp_path`
  instead (Sprint 376's Rubber Duck Observation 2).
- **A laptop card's amendment is a lapse by name.** `windows_machine` sends the laptop only what HEAD holds, so a card that
  changes the executor or a declared state before its laptop job commits that change mid-sprint; `signal_report`
  admits such a commit only by a row in `BETWEEN_SPRINTS` (24284d8f1 and 33b6423b3, Sprint 376), whose own comment
  says "a commit a rule admits is not a lapse" (`crates/signal_report/src/main.rs` 1499). A rule, as Sprint 349 made
  for `probes/`: a card amendment whose paths outside its card lie under `tools/windows_machine/`,
  `crates/windows_machine/` or `games/<game>/state.json` passes when its card's exit codes name a
  `target/release/windows_machine` job; the two rows stay as history.
- **Not here:** the tables re-taken (card 378); renaming `Py` (`lib.rs` 630), a separate concept, numbered at 377's
  close (sdd-kit-3/AGENTS.md hard rule 6). No API id is added.

## prerequisites

- 376 (closed)

## context_files

- `crates/windows_machine/` (src/lib.rs, src/main.rs, tests/), `tools/windows_machine/profile.ps1`, `run-job.ps1`.
- `.github/workflows/` (all three files).
- `grep -n "GitHub's Windows workflows retired at the source\|A laptop game run's initial state is the reference runs'" BLACKBOARD.md`.
- `docs/postmortem-windows-runs-off-the-laptop-2026-11-27.md`.
- `WORKING_AGREEMENT.md` lines 137-266.

## signal contract

### Emits

- None: no run is recorded by this card's changes; the probe job that reads the profile's new field is a laptop
  run's `profile.json`, not a vocabulary event.

### Invariants

- Every run as held; G3's batch runs conform; no Windows run on GitHub's runner; `crates/guest_runtime/src/api_ids.txt`
  unchanged; every stored laptop run's profile matches the pattern.

## artifact contract

### Files modified

- `crates/windows_machine/{src/lib.rs, tests/*.rs}`, `tools/windows_machine/{profile.ps1, run-job.ps1}`, `.github/workflows/`
  (two moved to `tools/retired-workflows/`), with others only where the class requires, reported to main when found. One concept: the reference
  machine named exactly, and no other machine able to stand in for it.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree
- `target/release/windows_machine probe apcs_and_waitable_timers` returns 0, its `profile.json` carrying the
  code-integrity state and its bundle `code-integrity.json`

## observation contract

- **Expected values:**
  - Every stored laptop run's `profile.txt` against the pattern (count); the walk's result.
  - The laptop's code-integrity state as `profile.json` records it; the CodeIntegrity check's line for a job.
  - `ssh -G w2m-laptop` naming the alias, and the key's fingerprint before and after.
  - `.github/workflows/` holding `ci.yml` alone; the test's output.
- **Planted faults:**
  - (1) A stored run's `profile.txt` set to `laptop-x64-10.0.26100.1-Server`: the walk fails naming it.
  - (2) A workflow with `runs-on: windows-2025` added under `.github/workflows/`: the test fails naming it.
  - (4) A card amendment of a laptop card touching `crates/w2m_vm/`: `signal_report --check` refuses it, naming the
    path.
  - (3) A run's proof with `seen.temp_path` `C:\w2m\temp` and strings to match: the store refuses it against the
    declared path.

## done criteria

Only the laptop, named by its machine, CPU class and Windows build, can make a stored Windows run; GitHub's Windows
workflows no longer sit where GitHub runs them; each run records the code-integrity state it met.

## self-review checklist

- [x] Scope names one verifiable property: the reference machine named exactly.
- [x] context_files cite the program, the scripts, the workflows and the Decisions.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three numbered plants.
- [x] One concept.

## result

- **One pattern for the laptop.** `crates/windows_machine/src/lib.rs` 61-74: `LAPTOP_PROFILE`,
  `laptop-x64-10.0.26200.<revision>-Client`, and `laptop_profile`, which holds a `profile.txt`'s whole text to it, the
  revision one or more digits. The four store steps take it in place of `starts_with("laptop-")`: `store` 575, the
  batch's check before any run is stored 1089, the game's 1318, `store_game` 1508, each refusal quoting the profile and
  the pattern. The class held it in two tests too: the reference walk (`tests/reference_machine.rs` `machine`, both
  reads) and `tests/runner_families_on_the_laptop.rs`' laptop run. Over the store the walk reads "walked 681 runs:
  laptop (laptop-x64-10.0.26200.<revision>-Client) 251, laptop batch 2, runner before the laptop 419, listed runner 9,
  failing 0": every stored laptop run matches, 200 probe runs at 7840 and 14 at 9974, 34 game runs at 7840 and 3 at
  9974. `crates/call_compare/src/lib.rs` 1363 `laptop_run` keeps its prefix: it ports the frozen host's membership rule
  and reads only the stored runs, which the walk holds to the pattern (reported to main).
- **The code-integrity state** (amendment 6fa622231, committed before the job, since `windows_machine` sends only what
  HEAD holds). `tools/windows_machine/profile.ps1` 16-19 and 44-46: `profile.json` `code_integrity`, the values of
  `HKLM:\SYSTEM\CurrentControlSet\Control\CI\Policy` (`VerifiedAndReputablePolicyState`, `SAC_PreviousState`,
  `SAC_EnforcementReason`) and Defender's `SmartAppControlState`. `tools/windows_machine/run-job.ps1` 34 opens the
  job's window. Its last `finally` (455-476) writes `out\code-integrity.json`: `from`, `to` and each event 3077 of
  `Microsoft-Windows-CodeIntegrity/Operational` in between (time, file, process, policy). A window with no event raises
  `NoMatchingEventsFound`, and any other failure is written as `error`. Measured on the laptop first: its log held six
  3077s, each a job's file under policy `VerifiedAndReputableDesktop` (three probe.exe loads by cmd.exe, Sprint 372;
  three handler_trace.dll loads by UnityCrashHandler64.exe, Sprint 376). Each had a 3033 beside it at the same
  instant, and the log's other 403 3033s name vendor DLLs (McAfee, DTrace, GPU drivers). Both scripts parsed on the
  laptop's pwsh 7.6.10 with 0 errors, and the fragment ran there over an empty window and over Sprint 376's three
  blocks. `lib.rs` 1367-1420: `CODE_INTEGRITY` and `code_integrity`. A job whose window holds a block, whose file is
  missing or whose read failed is refused before its verdict: probe 534, batch 1063, game 1293. The line names each
  blocked file, the process that loaded it, the policy and the time, "the job's missing output is the block's, not the
  program's". The file is kept: in `KEPT` for a probe run, once in a batch's folder, in each game execution by
  `split_game`.
- **The laptop job.** `target/release/windows_machine probe apcs_and_waitable_timers`, exit 0, 03:24:03-03:24:54 UTC,
  HEAD 6fa622231: "lap-20261128T032403626Z-6fa62223: CodeIntegrity: no block from 2026-11-28T03:24:33.3824007Z to
  2026-11-28T03:24:49.8556651Z"; "2 of 2 executions"; stored, 11 files, profile laptop-x64-10.0.26200.9974-Client,
  binary `4d655151...` (Sprint 371's, already stored). Its `profile.json` `code_integrity`:
  `{"verified_and_reputable_policy_state": 1, "sac_previous_state": 2, "sac_enforcement_reason": 1,
  "smart_app_control": "On"}`; its `code-integrity.json` `"blocks": []`. New test
  `every_laptop_run_from_377_records_its_code_integrity` (`tests/reference_machine.rs` 189-240): every stored laptop
  run from that one on carries both, 1 held.
- **The host key under the alias.** Before: `~/.ssh/known_hosts` lines 13-15 under `192.168.4.55`, ED25519
  `SHA256:xsrU4PoaBpi5+Vk94B9t5cIoV/qEKw0oetfrOGmudK8`, RSA `SHA256:Y9Ft+/701V4dti/h7sLrC0DBBrbdmmHxV/rcbc4aN4k`,
  ECDSA `SHA256:n3Tq4wFo0ZLvJNUBRuPnClrRof2kljlvWVv+PhwzCj8`; `ssh-keyscan windowsmachine.local` read the same three;
  `w2m-laptop` held none. The supervisor copied the three lines under `w2m-laptop` from the held key. After:
  `ssh-keygen -F w2m-laptop -l` gives the same three fingerprints, and `ssh -G` with `LINK`'s options reads
  `hostname windowsmachine.local`, `hostkeyalias w2m-laptop`. `lib.rs` 47-57: `LINK` passes `HostKeyAlias=w2m-laptop`.
  The `192.168.4.55` lines stay until this sprint's close commit carries the new `LINK`, `ssh-keygen -R 192.168.4.55`
  then. `tools/windows_machine/SETUP.md` section 10 says so. New test `the_link_checks_the_key_under_the_laptops_alias`.
- **GitHub's Windows workflows retired.** `.github/workflows/windows-probe.yml` and `windows-game.yml` moved by `mv`
  to `tools/retired-workflows/`, byte-identical to af000b0d4's; no file deleted. New `tests/github_workflows.rs`:
  `no_workflow_github_runs_makes_a_windows_run` reads every file of `.github/workflows/`. It fails on a `runs-on` that
  names Windows, on one that is an expression (`windows-probe.yml`'s `${{ inputs.runner }}`, default windows-2025), and
  on a call to another repository's workflow. It holds the two retired files to af000b0d4's. Its output:
  ".github/workflows: ci.yml; Windows jobs: 0". `the_reader_names_each_way_to_a_windows_runner` holds the reader to the
  two retired files, ci.yml, the list and flow forms and a comment. `run-job.ps1`'s two comments naming
  `.github/workflows/` now name `tools/retired-workflows/`.
- **The store's TEMP check holds the declaration.** `lib.rs` 1440-1453 `declared_temp`: `games/<game>/state.json`'s
  `temp_path` for the job's state, at the commit the job sends, read before the job is sent. `temp_declared` (1459)
  holds each GetTempPathW strings row to it, then the proof's `seen.temp_path` to it; `store_game` (1505) takes it. The
  test `a_run_whose_game_read_another_temp_is_not_stored` now refuses a run whose proof and strings both name
  `C:\w2m\temp` at row 3423 against the declared path. It refuses a proof naming that folder beside the reference
  strings at the proof, and still a proof with no TEMP path. New test `the_declared_temp_is_the_states_at_the_commit`.
- **A laptop card's amendment by rule.** `crates/signal_report/src/main.rs` 1558-1604: `LAPTOP_PATHS`
  (`tools/windows_machine/`, `crates/windows_machine/`, `games/<game>/state.json`), `laptop_path`, and `laptop_card`.
  The last reads card N at the amendment commit and finds a `target/release/windows_machine probe|binary|batch|game`
  line in its `### Command exit codes`. `commits_walked` (1677) admits those paths for a laptop card, and a path outside
  them is named with the admitted set. 24284d8f1 and 33b6423b3 stay in `BETWEEN_SPRINTS` as history. This sprint's
  amendment 6fa622231 changes the card, `profile.ps1` and `run-job.ps1` alone. Tests: the unit test
  `the_laptop_paths_are_the_executor_the_sender_and_a_games_state`, and the fixture
  `a_laptop_cards_amendment_of_the_laptops_paths_passes` (`tests/check.rs`). The fixture's laptop-path amendment
  passes committed and prepared; with `crates/w2m_vm/` it is named; with no laptop job in the card,
  `tools/windows_machine/` is named.
- **Plant 1** "store/probe-runs/lap-20261017T040446837Z-dc268176: profile.json names machine \"laptop\" and profile.txt
  is Some(\"laptop-x64-10.0.26100.1-Server\"): they do not both name the laptop", `cargo test -p windows_machine --test
  reference_machine every_stored` exit 101; live 03:25:31-03:25:38 UTC:
  - backup `scratchpad/plants/377/plant1-profile.txt`; the file's text set by the Edit tool and read back
    (`laptop-x64-10.0.26100.1-Server`); restored by `cp`, `cmp` equal.
- **Plant 2** ".github/workflows/plant.yml line 6: runs-on windows-2025 names Windows", `cargo test -p windows_machine
  --test github_workflows no_workflow` exit 101; live 03:25:47-03:25:47 UTC:
  - `.github/workflows/plant.yml` written by the Write tool, `ls` showing `ci.yml plant.yml`; moved out by `mv` to
    `scratchpad/plants/377/plant2-plant.yml`, `ls` showing `ci.yml` alone.
- **Plant 3** "left: Err(\"the game's TEMP is C:\\\\w2m\\\\temp\\\\ (strings row 3423, after GetTempPathW), where its
  state declares C:\\\\Users\\\\w2m\\\\AppData\\\\Local\\\\Temp\\\\\") right: Ok(())", `cargo test -p windows_machine
  --test windows_machine a_run_whose` exit 101; live 03:25:53-03:26:06 UTC:
  - backup `scratchpad/plants/377/plant3-windows_machine.rs`; the test's passing case given, by the Edit tool, a proof
    and strings that both name `C:\w2m\temp`; landed (grep 1); restored by `cp`, `cmp` equal.
- **Plant 4** "sprint 335: commit a53a8a969 \"Sprint 335 card amendment: the probe p\" amends its card and changes
  crates/w2m_vm/src/lib.rs, outside card 335, probes/ and the laptop's paths (tools/windows_machine/,
  crates/windows_machine/, games/<game>/state.json)", `cargo test -p signal_report --test check a_laptop` exit 101,
  `signal_report --check` on the fixture exit 1; live 03:26:12-03:26:22 UTC:
  - backup `scratchpad/plants/377/plant4-check.rs`; `crates/w2m_vm/src/lib.rs` added by the Edit tool to the laptop
    amendment's paths in the passing case; landed (grep 1); restored by `cp`, `cmp` equal.
- **Gates:** `exit 0 in 612 s	default	2026-11-28T03:38:10Z	sprint 377	6fa62223165cd532d6b5db6bbc987de5045e505b	f29f97db7273e834c478b50b7b6c7004c6794a08a8bd632908c162197f162ba2	root=46355b40d10a10133eace69f45cfc322b65f9c87	rust=all python=0 samples=0	base=8911d38fef663f2f43b7fd0723a2dca0d5e64f32`
- **G3's two-vCPU batch run in these gates:** `run_seconds` 130.871097877 (variant vm,
  `build/batch_run/record-2-vcpus-1795836591000000000`); `load_at_start` "load averages 1.86 1.96 2.60; busy:
  mds_stores 84%; other w2m: none".
