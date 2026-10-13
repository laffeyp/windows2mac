# Sprint 067 — the Windows reference machine

```yaml
---
id: 067
status: open
phase: 5
pass_kind: functional
---
```

## scope

The laptop arrived on 2026-10-13 (Decision 2026-10-11, user). This sprint makes it the Windows reference
machine that `docs/windows-reference-machine.md` plans, and routes probe runs and game runs to it. Sprint 066 stays
open with item 4 and its last divergence, HeapReAlloc 3268, which item D below decides (Decision 2026-10-13, user).

The machine as found (`docs/windows-reference-machine.md` §12): an HP Victus 15-fa2xxx; Intel Core i5-13420H, four
performance and four efficiency cores, 12 threads; 31.6 GB; NVIDIA GeForce RTX 3050 6 GB Laptop GPU beside Intel UHD
Graphics; Windows 11 Home 25H2, build 26200.7840, running under Hyper-V with virtualization-based security on.

Work items, in the plan's order (§10, steps A-G):

- **A. Arrival.** Done: OpenSSH Server with key-only login (a password login refused), port 22 from the local
  subnet only, the controller's `w2m-laptop` reaching `windowsmachine.local`; McAfee, HP's apps and services, OneDrive
  and 33 preinstalled packages removed; `C:\w2m` excluded from Defender; PowerShell 7, Git, zstd, uv, Python 3.12
  and LLVM 20.1.8 (the runners' clang, run 38304235237) installed. Left: the local administrator `w2m` (§9), pwsh as
  the SSH shell, sleep and lid-close off on AC, `tools/windows_machine/SETUP.md` with every command, and
  `profile.json` (R1) from a collector the executor also runs.
- **B. Executor.** `tools/windows_machine/run-job.ps1` and `scripts/windows_machine.py`: a probe job from inputs
  named by sha256, built and run as `windows-probe.yml` does, returning the stored layout. The game tree reaches
  the laptop from the controller's `~/w2m-depots/hail-to-the-rainbow` over SSH, checked against `files.json`.
- **C. Calibration** (§6): every family's stored probe binary run twice; rows sorted into equal, machine-decided and
  real differences; ten batch game executions from the empty initial state.
- **D. Reference sets** (§7): `call_compare.py` and `family_run.py` judge w2m against every Windows execution of a
  question.
- **E. Initial state** (§5) for the game.
- **F. Play mode:** a process in the interactive session started from SSH (Q2), the GPU pinned, a frame captured at
  the main menu.
- **G. Routing:** a Decision that new probe and game runs default to the laptop.

## prerequisites

- 066 (open; item 4 and HeapReAlloc 3268 wait on this sprint's item D)

## context_files

- `docs/windows-reference-machine.md`, `.github/workflows/windows-probe.yml`, `.github/workflows/windows-game.yml`,
  `scripts/store_probe_runs.py`, `scripts/record_game_run.py`, `scripts/call_compare.py`, `scripts/family_run.py`,
  `games/hail-to-the-rainbow/files.json`; v0.2 §3.1, §11.1, §14

## signal contract

### Emits

- Each laptop game run's `EXECUTION_STARTED` with `profile_kind` `windows_reference` through `record_game_run.py`;
  the families' item runs as today.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- created: `tools/windows_machine/` (run-job.ps1, profile collector, SETUP.md), `scripts/windows_machine.py`
- modified: `scripts/store_probe_runs.py`, `scripts/call_compare.py`, `scripts/family_run.py`
- gates 0 and CI `success` on the closing commit

## observation contract

- **A probe family's rows from the laptop pass `store_probe_runs.py`** and are sorted against the runners' rows.
- **Ten game executions on the laptop** write `unity.log` and a call trace the comparison reads.
- **The frontier's comparison runs against a set** of Windows executions and names, per divergence, how many of
  them disagree.

## done criteria

Steps A-E and G of the plan done; F done or moved by a Decision with its blocker named.
