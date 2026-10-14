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
- **H. One procedure.** `run-job.ps1` re-implements `windows-probe.yml`'s build and run steps, and the two have
  drifted (the workflow hashes its outputs; the executor did not). One script that the workflow and the laptop
  both call (review of 2026-10-13, finding 7).

The review of 2026-10-13 (agent review of items A-C as built) found failed runs losing their evidence, the
executor's own differences in 192 of the first 216 runs (session 0, the job folder's path), the core type
unrecorded and read at the wrong offset, Windows Update unpaused, the firewall rule on every profile, tool folders
writable by every user, and no laptop run on the record. Repaired in cd964062 and 3f57569b
(`windows_machine.py` verdict and copies, `run-job.ps1` notes and lock limit, `cpusets.ps1`, `profile.ps1`,
`provision.ps1`); the 234 runs from the old executor are set aside and calibration reruns on the repaired one.

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

## observed

### Calibration, step 2 (2026-10-13)

Batch `lap-20261013T231059422Z-136bec80` (commit 136bec80): every family's stored runner binaries, 195 in all,
each run twice on the laptop in w2m's desktop session from `D:\a\windows2mac\windows2mac`, one profile for the
batch, 2 minutes 46 seconds of executions. Judged by `scripts/laptop_calibration.py` with family_run's own
comparison (each family's derivation and exclusions, rows keyed by `s`). On the record: `store/probe-runs/`
(the batch's profile, job and log once in `store/probe-runs/lap-20261013T231059422Z-136bec80/`) and one
`windows_reference` session per run in `store/runs/probe-run-<run>`.

164 binaries equal, 31 differ, 0 failed. The 31, sorted:

| Cause | Families (rows) | Kind |
|---|---|---|
| Time zone: Pacific Standard Time against the runners' UTC | locale (5 rows, every result 8 hours off; `local.first_day` fails with error 87 because 1601-01-01 00:00 local is before 1601), time_and_error_mode (`time_zone`, `time_zone.names`, `local_time`) | machine setting |
| Edition and build: Windows 11 Home 26200 against Server 26100/20348 | version (`get_version.*`, `verify.product_*`: workstation, build above 26100, suite 768 against 400), system_info `version_ex` (product type 1 against 3) | machine |
| The Windows directory is `C:\windows`, lower case, on the laptop | files_more `windows.*`, system_modules `file_name`, temp_path `temp.none`, system_info `system_directory` | machine (installation) |
| Hardware | processor_topology `machine` (12 logical, 8 cores, 5 L2), process_queries `ipfp.89` (feature 89 against CPUID), com (an audio device: endpoints and defaults answer), display and dxgi (1920x1080 at 120 DPI, two adapters) | machine |
| The workspace volume: directory "." reports size 0, on the runners 4096 | file_attributes `ex.dot`, `ex.dot.backslash` | machine (volume); unconfirmed |
| Session state | windows: `msg.game.9` (WM_ACTIVATEAPP's lParam null: no other application in the foreground, under hidden, minimized and normal consoles), `class.register` (`after_first` changes between executions: the session's atom table) | executor, open |
| Windows' own variance | late_tls `late.blocks` (the laptop's two executions of one binary disagree; equal in the next batch) | the reference set (item D) |

The windows family's other rows follow from the display (work area, frame sizes, the first window's position).

Executor differences found and removed by calibration: the session (STARTUPINFO desktop name), the workspace path
(the module path's length), and the error mode a probe inherits (0x8001 against 0x8003; run-job.ps1 now sets
0x8003). Each was the executor's, not the machine's, and is the class to look for first when a family differs.

Speed: one job per binary took about 25 s each, 80 minutes for the 195; one batch takes about 5 minutes.

### Audit of 2026-10-13

An agent audit for the review's defect classes across the repository found 18; fixed in 307b6024 and 3372c2a4
(failures that passed in run_frontier.py, the laptop verdict and calibration store; git errors read as a clean
tree; untracked runtime sources; a case file unchecked against its run; unbounded scp, ssh and task waits; a
probe without a time limit; unrecorded frontier settings; the third execution dropped by store_probe_runs.py;
positional row naming). Open, each needing a Windows game run to test:

- `tools/jit_capture/call_trace.h:335` writes its logs every 5 s and never at exit, and windows-game.yml stops
  the game with `Stop-Process -Force`, so up to 5 s of the reference's last calls can be missing unmarked.
- `tools/jit_capture/jit_capture.c:176-177`: a failed VirtualAllocEx, WriteProcessMemory or QueueUserAPC leaves
  the crash handler untraced with nothing recorded.
- `scripts/family_run.py:2828`: `max_steps` 2^40 with a per-command `timeout_ms`, so a spinning probe in the VM
  has no wall-clock limit.

### Play mode and input (2026-10-13)

- Play mode runs on the laptop's RTX 3050 (UserGpuPreferences GpuPreference=2) in w2m's desktop session, the screen
  saved every 10 s: at 62 s the first scene (a stairwell, the prompt "Tab - RUN IIS"), the first reference frame
  for Phase 6 (batch lap-20261014T002654472Z-793c09a5). The trace reached 4,244 calls and unity.log 176 lines in
  60 s.
- Gameplay input (R5): `tools/windows_machine/input.cs` plays an input script (games/<game>/inputs/<name>.jsonl:
  seconds since the game started, key scan codes, relative mouse moves, buttons, wheel) with SendInput and records
  the raw keyboard and mouse (RIDEV_INPUTSINK). The round trip (batch lap-20261014T010547560Z-bbf0041b): the
  11-event script `round-trip`, played and recorded in one run, came back equal in every value, each event within
  112 ms of its time; this checks input.cs's structure offsets. A person's play is recorded only when a run needs
  the most realistic input or a defect needs shaking out (user, 2026-10-13).
- Two defects found on the way: a script error ended run-job with nothing logged and exit 0 (now trapped, logged,
  exit 1), and .NET resolved the screenshot path from its own directory.
