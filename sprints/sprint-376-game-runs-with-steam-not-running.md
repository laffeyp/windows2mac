# Sprint 376 — Laptop game runs with the Steam client not running, joining G3's reference set

```yaml
---
id: 376
status: active
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

- `tools/windows_machine/run-job.ps1`, `games/hail-to-the-rainbow/state.json`, with others only where the class
  requires, reported to main when found. One concept: laptop game runs under the reference's initial state.

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

GATES-LINE
