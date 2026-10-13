# The Windows reference machine

Status: plan, 2026-10-12. The machine arrives 2026-10-13. Decision 2026-10-11 (user) makes it the Windows reference
for probe runs and game runs; this document is how. Its hardware is unknown until it arrives, and every value marked
*on arrival* is recorded then.

## 1. Why the reference moves

Every judgement w2m passes is a comparison with Windows. Today that Windows is GitHub's hosted runners, under
Decision 2026-10-05 (agent-resolved): rented x86-64 machines, `windows-2025` and `windows-2022`, driven by
`.github/workflows/windows-probe.yml` and `windows-game.yml`.

The runners have carried the project a long way: 259 family runs (134 on Server 2025, 125 on Server 2022), 410 probe
runs in `store/probe-runs/`, ten game runs in `store/game-runs/`. Their limits now set the pace and the quality of
the evidence:

| Limit | Where it shows |
|---|---|
| A game run stops at 180 s, in batch mode, with no GPU and no audio device | `windows-game.yml` `seconds: 180`, `-batchmode -nographics`; plan rows 069 (audio) and Phase 6 (first frame) have no reference |
| The CPU is assigned per job: AMD EPYC 7763, 9V45, 9V74 or Intel Xeon 8370C, 6973P-C | `scripts/probe_on_intel.py` dispatches in batches until an Intel CPU answers |
| Each question waits on a queue, a checkout, a build and an artifact download | a probe run takes 43 s end to end (run 38304235237), a game run 15 min (run 38216081440), each before its download |
| The profile is one line | `profile.txt`: `gha-windows-2025-x64-10.0.26100.33438-Server`; v0.2 §11.1 asks for the build, CPU features, library versions, GPU and driver, hashes, initial state and external inputs |
| Paths and the user are the runner's | Sprint 060 junctioned the game to `C:\w2m\app`; Decision 2026-10-11 (user) makes w2m report `C:\Users\runneradmin` |
| One game run was taken as the reference | today's comparison: four stored Windows runs differ from run 38216081440 by 36, 59 and 60 calls, while w2m differs by 1 |

The last row is the most serious. A single run of a multithreaded program is one sample of the executions Windows
allows. v0.2 §14 says it plainly: "Nondeterministic execution is recorded as choices constrained by the model,
rather than mistaken for an unexplained test failure." Sprint 066 spent a measurement and a design choice on
divergences that Windows itself produces from run to run (HeapReAlloc 3268: three in-place growths in run
38203364352, four in the other three).

## 2. Requirements

Each requirement names its source.

- **R1. A full reference profile** (v0.2 §11.1). Every run records: Windows edition, build and UBR; CPU model,
  family, stepping, microcode and CPUID feature flags; core and thread counts; RAM; GPU, driver version and
  Direct3D feature level; the audio endpoint; display resolution and DPI; the user profile path; the hashes of every
  input; the initial state (R3); and whether a hypervisor is present.
- **R2. Native x86-64** (v0.2 §11.1, constraint E4). Runs execute on the laptop's own CPU, not in a VM, unless a
  run says otherwise. Decision 2026-10-05's revisit condition ("a probe's observation differs between a virtualized
  and a bare-metal run") becomes testable for the first time, and calibration (§6) tests it.
- **R3. A declared initial state** (v0.2 §14: "reproducible with respect to their declared initial state").
  The game writes PlayerPrefs to the registry (run 38216081440 seq 3196-3199 and 3478-3486: RegCreateKeyW,
  RegSetValueExA, RegDeleteValueA under HKCU) and files under its company folder. A second run that starts from the
  first run's state is a different experiment. Each run starts from a stated state, and the executor proves it.
- **R4. Several executions per question** (v0.2 §14; today's finding). A reference is a set of runs, and a
  divergence is a w2m behaviour outside what the set shows.
- **R5. Controlled external inputs** (v0.2 §3.1). Clock-dependent and input-dependent behaviour is recorded with the
  inputs that produced it: wall-clock start, keys sent and their times, the window's size, network state.
- **R6. One controller, authenticated** (security). Only the Mac commands the machine.
- **R7. No change to what the Mac reads.** Results land in the layouts `store/probe-runs/<run>/` and
  `store/game-runs/<run>/` already hold, so `family_run.py`, `call_compare.py`, `store_probe_runs.py` and
  `record_game_run.py` keep working.
- **R8. The runners stay.** Their 259 family runs remain evidence for their machines (Decision 2026-10-11, user:
  "the runners' existing rows stay on the record"). A second machine distinguishes Windows from one machine.

## 3. Architecture

```
Mac (controller)                                      Laptop (executor)
  store/, scripts/, the record                          C:\w2m\  (nothing kept between jobs)
  scripts/windows_machine.py  ── ssh / scp ──▶          job folder from inputs named by hash
     job: probe | game, inputs by sha256                 reset state, verify state
     ◀── result bundle: outputs, profile.json,           run, capture, hash outputs
         state digests, exec.json                        delete job folder
```

**The controller holds everything.** Inputs, results, profiles and the record stay on the Mac. The laptop holds
an operating system, a pinned toolchain, the game's unpacked tree (checked against `games/<game>/files.json` before
each run), and nothing else that a run can read.

**A job is data.** A job names its kind (probe or game), its inputs by sha256 (the probe source or binary, the
launcher, the game tree's manifest), its arguments, its repetition count and its initial state. The controller
copies the inputs, the executor checks each hash before use, and a mismatch stops the job.

**The executor is a script, not a service.** `C:\w2m\bin\run-job.ps1` (committed under `tools/windows_machine/`)
creates `C:\w2m\jobs\<job id>\`, resets the state (§5), records the profile, runs each execution, hashes the outputs,
writes the bundle and deletes the job folder. The same script runs a probe exactly as `windows-probe.yml`'s build and
run steps do, and a game exactly as `windows-game.yml`'s install and run steps do, so the two machines' results
differ only in the machine.

**The bundle has the stored layout.** A probe job returns `body-<i>.jsonl`, `exec-<i>.json`, `cpu.txt`,
`probe.sha256`, `profile.txt`; a game job returns the `jit_capture.bin.*` files, `unity.log`, `exec.json`, `cpu.txt`,
`profile.txt`. Each adds `profile.json` (R1) and `state.json` (§5). `profile.txt` takes the form
`laptop-<build>-<cpu>` so `store_probe_runs.py`'s image test can tell the machines apart; that script and the
family `Run` record gain a machine field in place of the runner image.

**Run identities.** A laptop run's id is `lap-<UTC timestamp>-<job hash prefix>`; a runner's stays its Actions run
id. Both go through `record_game_run.py` as `EXECUTION_STARTED` with `profile_kind` `windows_reference` (vocabulary
0.7; no new tags).

## 4. Control channel

The Mac reaches the laptop through Microsoft's OpenSSH Server, part of Windows since Windows 10 build 1809 as a
feature on demand ([Microsoft Learn: OpenSSH for Windows overview](https://learn.microsoft.com/en-us/windows-server/administration/openssh/openssh-overview)).

- Key-based login only: the Mac's public key in `C:\ProgramData\ssh\administrators_authorized_keys`, its ACL
  Administrators and SYSTEM only; `PasswordAuthentication no` in `C:\ProgramData\ssh\sshd_config`
  ([Microsoft Learn: key-based authentication](https://learn.microsoft.com/en-us/windows-server/administration/openssh/openssh_keymanagement),
  which notes that password login across systems "becomes vulnerable to brute force intrusions").
- The firewall rule for port 22 admits the home network's subnet only.
- PowerShell 7 as the default shell; `scp` for inputs and bundles.
- Not a self-hosted GitHub runner: the repository is public, and GitHub's guidance is that self-hosted runners
  "should almost never be used for public repositories" because any pull request can run code on them
  ([GitHub: security hardening for Actions](https://docs.github.com/en/actions/security-for-github-actions/security-guides/security-hardening-for-github-actions)).

**Batch runs and play runs need different sessions.** A process started from an SSH session runs without the
interactive desktop. Batch mode (`-batchmode -nographics`) needs none. Play mode needs the logged-in user's desktop
for its window, its GPU device and `SendKeys`. The executor therefore starts play jobs inside the console session
of the logged-in `w2m` user; the mechanism is open question Q2 and is settled on day one, before any play run.

## 5. Initial state

A run's state is everything the game or probe can read that a previous run could have changed.

| State | Reset | Proof in `state.json` |
|---|---|---|
| The game tree | verified against `files.json` before each run; any difference reinstalls | the listing's digest |
| PlayerPrefs, `HKCU\Software\NSS\Hail to the Rainbow` | key deleted, then restored from the declared state's export (empty for a first-launch run) | the key's export digest before the run |
| `%USERPROFILE%\AppData\LocalLow\NSS\Hail to the Rainbow` | folder deleted, then restored from the declared state | the folder's listing digest |
| `C:\Users\w2m\AppData\Local\Temp` (TMP, TEMP) | emptied | its listing digest |
| `C:\w2m\unity.log`, crash handler output | deleted | absence |
| Leftover processes from the game folder | stopped, as `windows-game.yml` does, and waited for | process list from the game folder, empty |
| Windows itself | updates paused for a measurement series; the build recorded | build and UBR in `profile.json` |

The company and product names come from the game's `Hail to the Rainbow_Data/app.info` (`NSS`, `Hail to the
Rainbow`); `games/<game>/state.json` keeps them with the declared initial states. A run that finds its post-reset digests
unequal to the declared state's stops before the game starts.

## 6. Calibration: is the laptop a sound judge

Before the laptop's rows count, it reproduces what the record already holds. Each step's result goes into the
sprint card.

1. **Profile.** `profile.json` on the bare machine. Answers which CPU vendor the laptop is (for the classes whose
   rows differ between AMD and Intel, Sprint 037: 0F 0D register forms).
2. **Probes.** Every family's probe binary from `store/binaries/`, unchanged, run twice on the laptop. Rows equal to
   the stored rows confirm the laptop; rows that differ are sorted into machine-decided values (CPU name, topology,
   paths, user, display, time zone) and real differences. A real difference is a finding against R2 and is
   resolved by a Decision before the laptop's rows replace any runner row.
3. **The game, batch, as the runners ran it.** The same launcher, arguments, junction and paths; ten executions from
   the empty initial state. `unity.log` against the runners' 168 lines; the call trace against the four stored
   runner runs.
4. **The spread.** For each (image, function, call) pair, the set of values the fourteen Windows executions show (ten
   laptop, four runner). This is the first measurement of what Windows itself varies.

## 7. The reference as a set

`scripts/call_compare.py` compares w2m with one run today. With the laptop it compares w2m with a set.

- **Per call:** w2m's value conforms when some reference execution shows the same value under the signature's
  relation (value, pointer, handle as now). A call w2m misses conforms when some reference execution also lacks it.
- **Per thread pair:** pairing stays as now (names, first appearance, agreement), applied to each reference
  execution.
- **Report:** each divergence says how many reference executions disagree with w2m, of how many. A value no
  execution shows is a divergence; a value some show and some do not is a choice Windows allows (v0.2 §14), recorded
  and not chased.
- **Families too:** `family_run.py` already holds two executions per run and two runs per family; the same rule
  applies, and an `open_questions` line that names a varying value becomes a set the rows accept.

This is the change the user approved on 2026-10-12 ("go"). It applies to the runners' four game runs at once and
closes Sprint 066's last divergence, HeapReAlloc 3268, as a value Windows varies.

## 8. What the laptop makes possible

Ordered by what the plan needs next.

1. **Fast probe rounds.** A probe built and run without a queue or a checkout; `probe_on_intel.py`'s batching becomes
   unnecessary for the laptop's vendor.
2. **Variance as routine** (§7): ten executions of a question as cheaply as one.
3. **Game runs past 180 s and in play mode.** The game's main menu, scene loads and frames with a real GPU: the
   reference for Phase 6 (milestone B, first frame) and Phase 7.
4. **Audio rows** (plan row 069): an output device the runners lack.
5. **Frames as evidence.** Screenshots or GPU captures at fixed inputs, for milestone B's judge; tool choice is
   open question Q5.
6. **Instruction-level ground truth where the call trace is too coarse.** Microsoft's Time Travel Debugging records a
   process's full execution, can restrict recording to named modules (`-module`), and slows the recorded process
   5x-20x ([Microsoft Learn: TTD.exe](https://learn.microsoft.com/en-us/windows-hardware/drivers/debuggercmds/time-travel-debugging-ttd-exe-command-line-util)).
   It answers questions such as which code path chose the second delete buffer, with the instruction stream, not an
   inference from calls. It perturbs timing, so a TTD run is a separate profile and never a reference for timing.
7. **Longer and repeated captures** for the decoder and semantics differentials (`game_code_differential.py`).

## 9. Machine setup

Done once, on arrival, and recorded in `tools/windows_machine/SETUP.md` with every command run.

- **Accounts.** One local administrator `w2m`, so the profile path is `C:\Users\w2m` and matches w2m's
  `USER_ENVIRONMENT`. Decision 2026-10-11 (user) has w2m report `C:\Users\runneradmin`; the laptop's runs carry
  `C:\Users\w2m`, and a Decision picks one when calibration step 3 shows the difference (Q3).
- **Windows.** Edition, build and UBR recorded. Updates paused during measurement series. Sleep and display-off
  disabled on AC power. Defender: the job folder excluded from real-time scanning, the exclusion recorded in the
  profile (scanning changes file timing).
- **Toolchain, pinned.** LLVM clang and llvm-rc at the version the runners' images carry, recorded by
  `clang --version` in every profile; Python and uv for probes with a `generate` step; zstd for the game's parts; git
  for `substrate.commit` checkouts.
- **Paths.** `C:\w2m\bin` (executor scripts), `C:\w2m\jobs` (per-job, deleted after), `C:\w2m\games\<game>`
  (unpacked tree), `C:\w2m\app` (the junction the game runs from, as on the runners).
- **The game.** Unpacked once from its private release with the parts' hashes checked, as `windows-game.yml` does;
  verified against `files.json` before each run.
- **Remote access.** §4.

## 10. Work plan

The work is one sprint of setup and calibration, then the comparison change, then the new capabilities as plan rows
reach them. Sprint numbers follow the plan's order at the time each opens.

| Step | Work | Done when |
|---|---|---|
| A. Arrival | §9 setup; §4 channel; `profile.json` | the Mac runs `ssh w2m-laptop pwsh -c "$PSVersionTable"`; a password login is refused |
| B. Executor | `tools/windows_machine/run-job.ps1`, `scripts/windows_machine.py`; probe jobs | one family's probe runs on the laptop and its bundle passes `store_probe_runs.py` |
| C. Calibration | §6 steps 1-4 | every family's rows sorted into equal, machine-decided and real differences; ten game executions stored |
| D. Reference sets | §7 in `call_compare.py` and `family_run.py` | the frontier's comparison runs against fourteen executions; HeapReAlloc 3268 reported as a value Windows varies |
| E. Initial state | §5 for the game | `state.json` digests equal the declared state on ten consecutive runs |
| F. Play mode | Q2 settled; play jobs | the game reaches its main menu on the laptop with a frame captured |
| G. Routing | new probe runs and game runs default to the laptop; the workflows stay for the runners | `BLACKBOARD.md` Decision; the card's closing CI green |

Steps B and D do not depend on each other; D can start on the runners' four game runs before the laptop answers.

## 11. Open questions

- **Q1. The hardware.** CPU vendor and model, GPU, RAM, Windows edition: *on arrival*. An Intel CPU removes
  `probe_on_intel.py`'s batching; an AMD CPU keeps the runners for Intel rows.
- **Q2. Play jobs in the desktop session.** Which mechanism starts a process in the logged-in user's session from an
  SSH command, and whether the logged-in session must stay unlocked for `SendKeys` and the GPU device.
- **Q3. The user profile path** w2m reports: `runneradmin` (runner reference) or `w2m` (laptop reference).
- **Q4. Bare metal against virtualized.** Calibration step 2 is the first test of Decision 2026-10-05's revisit
  condition; a difference narrows that Decision's claim.
- **Q5. Frame capture** for milestone B: a screenshot of the window, or a GPU capture tool's frame.
- **Q6. TTD's place in the record.** A TTD trace is evidence of one execution's path; whether it enters the record as
  an execution with its own `profile_kind` or as a sidecar of a laptop run.
