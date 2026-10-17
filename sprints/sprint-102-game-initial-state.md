# Sprint 102 — the game's initial state on the laptop, proved before each run

```yaml
---
id: 102
status: closed
phase: 5
pass_kind: functional
---
```

## scope

The fourth of the five entries deferred that should not have been (user decision 2026-10-16): Sprint 067's step E,
the game's initial state (docs/windows-reference-machine.md §5). `tools/windows_machine/run-job.ps1` already resets
most of it before each game execution: the tree mirrored and checked against files.json, the PlayerPrefs key
`HKCU\Software\NSS\Hail to the Rainbow` and the LocalLow folder deleted, `C:\w2m\unity.log` deleted, leftover game
processes stopped after the run. §5 asks three things more, and none was done: TEMP emptied; a proof of each item in
the run's output; a run whose reset state differs from the declared one stops before the game starts. The declared
states (`games/<game>/state.json`) do not exist.

The one declared state is `empty`, a first launch, the state every run so far meant to start from. A non-empty state
(a save restored from an export) is what a run that starts partway into the game needs (user, 2026-10-16: "How can we
open games up to not the beginning"); §5 already names it, and it is declared when a run needs it.

## prerequisites

- 101.

## context_files

- docs/windows-reference-machine.md §5; tools/windows_machine/run-job.ps1 (the game job); scripts/windows_machine.py
  (the copies back); games/hail-to-the-rainbow/.

## signal contract

### Emits

- None new: the proof is a file of the run's output (`state.json` beside `exec.json`), as `exec.json` is.

### Invariants

- Vocabulary unchanged.

## artifact contract

- `games/hail-to-the-rainbow/state.json` (the company and product from app.info, the `empty` state); run-job.ps1:
  TEMP emptied, leftover game processes stopped before the start too, each §5 item read back after the reset and
  written to `out/state.json` with the files.json digest, a difference from the declared state an error before the
  game starts; `scripts/gates.sh` exit 0; CI `success` on the closing commit.

## observation contract

- **Judge:** a batch game execution on the laptop returns `state.json` equal to the declared `empty` state.
- **Plant:** a value written to the PlayerPrefs key after the reset (job.json `plant_state`): the job must stop before
  the game starts, naming the key.

## done criteria

Every laptop game run starts from a declared initial state it proves in its output; CI green.

## observed

- **Made** (90093828, cd6cac64): games/hail-to-the-rainbow/state.json (company NSS and product Hail to the Rainbow
  from app.info; the `empty` state); run-job.ps1 resets, reads back and proves the state in `state.json`, and stops
  before the game on a difference; windows_machine.py `--initial-state` and `--plant-state`.
- **First run, stopped by the check** (lap-20261017T065012358Z-90093828): "temp_entries 4, declared 0". The user's
  TEMP is shared: Steam now runs on the laptop in the job's session, with Defender's session helper. Of the five
  entries listed afterwards (all written at the job's minute: `steam`, `remoteIpMoProxy_ConfigDefender_...`,
  `79A1.tmp`, `tmp_sfyc2iip.opk`, a GUID-named `.tmp`), four deleted and the `.tmp` was held open by a process not
  identified. The game's TEMP became
  the job's own `C:\w2m\temp` (cd6cac64; GetTempPath reads TMP first); §5's row says so.
- **Judge** (lap-20261017T065856365Z-cd6cac64): state.json equal to the declared state: registry key and LocalLow
  absent, 0 TEMP entries, no unity.log, 0 game processes, 297 tree files checked against files.json (sha256
  c4c7b444...); the game then ran its 60 s (63 s, stopped by the job).
- **Plant** (lap-20261017T070416493Z-cd6cac64, `--plant-state`): stopped before the game with "initial state empty:
  registry_key present, declared absent", no exec.json written.
