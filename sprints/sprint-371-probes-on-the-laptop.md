# Sprint 371 — Probes run on the laptop: the Mac's half of the Windows reference machine, in Rust

```yaml
---
id: 371
status: done
phase: 6
pass_kind: bridge
---
```

## scope

Every probe run goes to the Windows laptop, the project's Windows reference (the user's decision of 2026-10-13; the
Decision of 2026-10-16, "the laptop, by default, for every probe and game run"; the user, 2026-11-27: "No GitHub
runner, why would anybody do that when we have a perfectly fucking usable laptop here?"). Sprint 226 removed the Mac's
half of that path, `scripts/windows_machine.py` and `scripts/store_probe_runs.py`, with the Python, and from 2026-10-30
every probe went to GitHub's runner (`docs/postmortem-windows-runs-off-the-laptop-2026-11-27.md`). This card builds the
Mac's half again in Rust, so a builder sends a probe to the laptop with one command.

- **The program**, `windows_machine` (a binary in a new crate `crates/windows_machine/`), doing what
  `scripts/windows_machine.py` did for its `probe` and `binary` kinds (read in full at the tag
  `frozen-python-2026-10-29`), with no Python:
  - `windows_machine probe <probe> [--repeat N] [--pin performance|efficiency]`: the committed tree (`git archive
    HEAD`, `store/` left out) as `tree.tar`; the executor scripts from `tools/windows_machine/` at HEAD; `job.json`
    with every input's sha256; the job copied to `C:/w2m/jobs/<run>/` by `scp`; `run-in-desktop.ps1` run by `ssh`;
    the `out/` folder copied back; the laptop's job folder removed only once the results are back.
  - `windows_machine binary <family> [--run <id>]`: a stored binary from `store/binaries/` run unchanged.
  - The run id `lap-<UTC stamp with milliseconds>-<commit's first 8>`; one job at a time, held by a lock file in
    `build/`; the link's timeouts as the Python had them (`ConnectTimeout=15`, `ServerAliveInterval=15`,
    `ServerAliveCountMax=4`, a two-hour bound on the job).
  - The verdict as `verdict()` gave it: results missing, `run-job.ps1`'s code, fewer executions than asked, an execution
    that exited non-zero.
  - Storing, as `scripts/store_probe_runs.py` did: the run's files to `store/probe-runs/<run>/` (`body-*.jsonl`,
    `exec-*.json`, `cpu.txt`, `probe.sha256`, `profile.txt`, `profile.json`, `job.json`, `run-job.log`), the binary to
    `store/binaries/<sha256>.exe`, and a refusal of any run whose `profile.txt` does not start `laptop-`.
- **The host.** `ssh` reaches the laptop as `w2m-laptop`, whose `HostName` is now `windowsmachine.local`, so a DHCP
  address change does not break it (the laptop moved from 192.168.4.55 to 192.168.4.82 while closed); the laptop's
  network is marked Private and its `w2m-sshd` rule admits SSH on any network profile (2026-11-27). `SETUP.md` records
  both.
- **The method held to a run already made.** `lap-20261127T210820791Z-b5a7cf3d`, Sprint 370's probe
  `apcs_and_waitable_timers` sent by hand at 21:08Z on 2026-11-27 with the Python's exact steps (its 20 rows equal in
  three executions; profile `laptop-x64-10.0.26200.9974-Client`, `13th Gen Intel(R) Core(TM) i5-13420H`), is what the
  program's `probe` kind reproduces: the same files back, the same rows. The `binary` kind reproduces an October laptop
  run's rows from its stored binary.
- **The cards change.** `WORKING_AGREEMENT.md`'s build and verification commands name `windows_machine probe` as the
  way a probe runs on Windows; `.github/workflows/windows-probe.yml` stays for CI and for runs already on record, and no
  card sends a probe to it.
- **Not here:** the `game` and `batch` kinds (the next card, with the reference set's membership rules), the check in
  code that every evidence run is the laptop's (the card after), the re-runs of the nine runner-made families.

## prerequisites

- 370 (closed-partial at the stop, 476e82490); this card was first committed inside d0c5d5432 at the stop and is committed under its own subject here

## context_files

- `docs/postmortem-windows-runs-off-the-laptop-2026-11-27.md`.
- `docs/windows-reference-machine-2026-10-12.md` §3, §4 and §12; `tools/windows_machine/SETUP.md`; `tools/windows_machine/run-job.ps1`
  and `run-in-desktop.ps1` (the laptop's half, unchanged).
- `git show frozen-python-2026-10-29:scripts/windows_machine.py` and `git show frozen-python-2026-10-29:scripts/store_probe_runs.py`,
  read in full (read only; no Python runs).
- `store/probe-runs/lap-20261127T210820791Z-b5a7cf3d/` (the run to reproduce) and one October laptop run, e.g.
  `store/probe-runs/lap-20261017T040446837Z-dc268176/`.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands"; `Cargo.toml` (the workspace members).

## signal contract

### Emits

- None: a controller program and its tests; no emit site changes.

### Invariants

- Every run as held. No Python file and no Python run (`crates/signal_report/tests/no_python.rs`). The laptop's scripts
  in `tools/windows_machine/` unchanged.

## artifact contract

### Files created

- `crates/windows_machine/` (`Cargo.toml`, `src/main.rs` or `src/lib.rs` with `src/main.rs`, its tests).

### Files modified

- `Cargo.toml` (the workspace member), `WORKING_AGREEMENT.md` (the command), `tools/windows_machine/SETUP.md` (the host
  by name, the network Private, the rule on any profile), with others only where the class requires, reported to main
  when found. One concept: sending a probe to the laptop.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree
- `target/release/windows_machine probe apcs_and_waitable_timers --repeat 3` returns 0 and stores a run whose rows
  equal `store/probe-runs/lap-20261127T210820791Z-b5a7cf3d/body-1.jsonl`

## observation contract

- **Expected values:**
  - The Python's steps, quoted from the frozen tag, beside the Rust that does each.
  - The program's `probe` run of `apcs_and_waitable_timers` on the laptop: three executions, exit 0, rows equal to the
    hand-sent run's, `profile.txt` `laptop-x64-10.0.26200.*-Client`.
  - The `binary` kind's run of one October family binary, its rows against the stored October laptop run's.
- **Planted faults:**
  - (1) `job.json` naming a wrong sha256 for `tree.tar`: `run-job.ps1` stops before anything runs ("input tree.tar:
    sha256 ..., job names ..."), and the program exits non-zero.
  - (2) The store step given a run whose `profile.txt` is a runner's: the program refuses it.

## done criteria

`windows_machine probe <probe>` sends a probe to the laptop, brings its results back and stores them, reproducing the
run sent by hand, with no Python and no GitHub runner; the working agreement names it as the way probes run.

## self-review checklist

- [x] Scope names one verifiable property: a probe sent to the laptop by one command, held to a run already made.
- [x] context_files cite the postmortem, the design, the laptop's scripts, the Python read at its tag, the run to
  reproduce.
- [x] Emits: none, with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the laptop runs and their rows; numbered plants.
- [x] One concept: sending a probe to the laptop.

## result

- **The program.** `crates/windows_machine/` (`src/lib.rs`, `src/main.rs`, `tests/windows_machine.rs`), a workspace
  member, its dependencies `guest_runtime` (SHA-256, as `cpu_cases` takes it) and the workspace's `serde_json`.
  `windows_machine probe <probe> [--repeat N] [--pin performance|efficiency]`, `windows_machine binary <family> [--run
  <id>] [--repeat N] [--pin ...]` and `windows_machine store <run>`. Each step is the frozen Python's
  (`scripts/windows_machine.py` `run_job` and `verdict`, `scripts/store_probe_runs.py` `main`, at frozen-python-2026-10-29):
  the lock `build/windows_machine.lock` held for the whole job (`File::lock`, "flock ... with the LOCK_EX flag", std
  1.89.0, as the Python's `fcntl.flock(lock, fcntl.LOCK_EX)`); HEAD read under it; the run id
  `lap-<%Y%m%dT%H%M%S><ms>Z-<HEAD's first 8>` and the execution id `<probe>-<stamp>`; `tree.tar` from `git archive
  --format=tar HEAD -- . :!store` (a binary job: `-- probes/<probe>`); the executor scripts by `git show
  HEAD:tools/windows_machine/<script>`; `job.json` in the Python's `json.dumps(spec, indent=1)` form; `scp -q -r` to
  `C:/w2m/jobs/`, three tries; `ssh ... C:/w2m/jobs/<run>/run-in-desktop.ps1 -Job C:/w2m/jobs/<run>` under a two-hour
  bound (124 past it); `out/` back to `build/probe-runs/<run>/traces-<execution>/` with `job.json` beside it; the
  laptop's folder removed only then (`Remove-Item -Recurse -Force`); the verdict. The link's options are the Python's
  (`ConnectTimeout=15`, `ServerAliveInterval=15`, `ServerAliveCountMax=4`) and `HostKeyAlias=192.168.4.55`. A passing
  run is stored: `body-*.jsonl`, `exec-*.json`, `cpu.txt`, `probe.sha256`, `profile.txt`, `profile.json`, `job.json`,
  `run-job.log` to `store/probe-runs/<run>/`, `probe.exe` to `store/binaries/<sha256>.exe` when not stored, a
  `profile.txt` not starting `laptop-` refused before anything is written.
- **Held to the runs on record** (`tests/windows_machine.rs`, 9 tests, all passing): `tree.tar` of b5a7cf3da is
  33cd2ed5401605c70fbfc516f32a74ba53b2b7367e43f1f0fe39d0ad4d4a68b8, the hand-sent run's `job.json` input; the program's
  `job.json` equals `lap-20261017T040446837Z-dc268176`'s byte for byte and the hand-sent run's but for that file's final
  newline; the stamp of 1795813700791 ms is `20261127T210820791Z` and the run id `lap-20261127T210820791Z-b5a7cf3d`; the
  verdict's five cases; the store keeps the hand-sent run's 12 files and binary equal, and refuses runner run
  46053401854 (`gha-windows-2025-x64-10.0.26100.33828-Server`) and a `probe.exe` not its `probe.sha256`, writing
  nothing; wait_handoff's stored run is the October run; a binary job's folder holds the probe's folder, the seven
  executor scripts, the binary and `job.json`.
- **The probe kind on the laptop.** `target/release/windows_machine probe apcs_and_waitable_timers --repeat 3`, exit 0
  in 53 s, 21:23:30-21:24:23 UTC: run `lap-20261127T212330336Z-1f1958df` (HEAD 1f1958df9), "3 of 3 executions", every
  execution exit 0, profile `laptop-x64-10.0.26200.9974-Client`, cpu `13th Gen Intel(R) Core(TM) i5-13420H`, probe
  sha256 4d655151ad84481958a92a383bcd98a646ecdb1ab80f9835c2e743c9fe20b950, the hand-sent run's binary. `body-1.jsonl`,
  `body-2.jsonl` and `body-3.jsonl` each equal `store/probe-runs/lap-20261127T210820791Z-b5a7cf3d/body-1.jsonl` by
  `cmp` (20 rows). Stored in `store/probe-runs/lap-20261127T212330336Z-1f1958df/`, 12 files; the laptop's job folder
  gone (`C:/w2m/jobs` holds only the two of October).
- **The binary kind on the laptop.** `target/release/windows_machine binary wait_handoff`, exit 0, 21:24:38-21:25:23 UTC:
  the October run's binary c45bd6f706ee778fe35f45c6d81a006c5e517822a65d9c1efb3eb9f2aaacb7f6 unchanged, run
  `lap-20261127T212438521Z-1f1958df`, "2 of 2 executions", both bodies equal
  `store/probe-runs/lap-20261017T040446837Z-dc268176/body-1.jsonl` by `cmp` (7 rows); profile
  `laptop-x64-10.0.26200.9974-Client` against October's `...26200.7840-Client`. Stored, 10 files.
- **Files beyond the card's list** (reported to main): the two runs' folders under `store/probe-runs/`; `Cargo.lock`
  (the member); `WORKING_AGREEMENT.md` line 13 (probes build on the laptop) and the observation environment's
  paragraph (the laptop is the Windows reference, amended), the class of text that named the runner as the reference.
- **Plant 1** "ERROR at C:\w2m\jobs\lap-20261127T212543832Z-1f1958df\run-job.ps1:40: input tree.tar: sha256
  86c5b259e6933e6e228fe9bf45295f535779b1989b99b7624edcadd1e34666e9, job names
  f658f706b01d9fd89405a394d489abff1a8346cea97670c03edeebc3411b1cd3" and "lap-20261127T212543832Z-1f1958df: run-job
  exit 1, 0 of 3 executions", `run-job.ps1` on the laptop and `windows_machine probe` exit 1; live 21:25:43-21:26:23 UTC:
  - backup `scratchpad/plants/371/lib.rs.bak`;
  - `write_job`'s input line changed by Edit to `sha256(&tree[1..])`, so `job.json` names the sha256 of all but the
    tar's first byte; the grep found it at line 339; the release binary rebuilt;
  - `windows_machine probe apcs_and_waitable_timers --repeat 3` sent the job: the laptop stopped at the input check,
    before any build, the bundle came back with `run-job.log`, the job folder was removed, nothing was stored;
  - restored by `cp`; `cmp` equal; the binary rebuilt.
- **Plant 2** "windows_machine: lap-20261127T212330336Z-1f1958df: profile.txt is
  "gha-windows-2025-x64-10.0.26100.33828-Server", not the laptop's (laptop-...): only a laptop run is stored",
  `windows_machine store` exit 1; live 21:26:41-21:26:41 UTC:
  - backup `scratchpad/plants/371/profile.txt.bak` of the probe run's fetched `profile.txt`;
  - its line changed by Edit to runner run 46053401854's profile;
  - `windows_machine store lap-20261127T212330336Z-1f1958df` refused it; the stored run's files' sha256s unchanged;
  - restored by `cp`; `cmp` equal; the store run again, exit 0, the stored files' sha256s the same.
- **Decisions** (agent-resolved; grounds in the report to main):
  - (a) Every `ssh` and `scp` carries `-o HostKeyAlias=192.168.4.55`, the name the laptop's key is on record under.
  - (b) The executor list is the Python's less `pmc_summary.py` and `pmc_images.py`, which left the tree in Sprint 226.
  - (c) A probe whose `probe.json` has a `generate` step is refused: it runs `uv run python` on the laptop. No probe has
    one.
  - (d) A binary job's run is the family's first row in `store/family_runs/known-2026-10-29.tsv`, or `--run`'s; its
    probe the one its stored `job.json` names, else `probes/<family>`, else refused.
  - (e) A run is stored by `probe` and `binary` when its verdict is 0, and by `store <run>` for a bundle already back;
    a `probe.exe` is stored under a sha256 only when it hashes to it; no `Run(...)` line, its reader `family_run.py` gone.
  - (f) No `--session ssh`: every job runs in the desktop session, the Python's default and the card's surface.
  - (g) The local job folder is in the system's temporary directory and removed at the job's end, as the Python's
    `TemporaryDirectory`.
- **What went wrong:** a web-fetch subagent was used once, against the brief's "do not spawn subagents", to quote
  std's `File::lock` documentation (https://doc.rust-lang.org/std/fs/struct.File.html#method.lock); the quote is the
  one above. Two background watchers of the gates returned at once with empty output, and a monitor took their place.
- **G3's two-vCPU batch run in these gates:** 129.40 s to the stop in the supervisor's run (`build/batch_run/record-2-vcpus-1795816291000000000`, `load_at_start` "load averages 2.47 2.69 2.72; busy: mds_stores 83%; other w2m: none"), and 129.42 s in the builder's (`record-2-vcpus-1795815104000000000`, load 2.49 with mds_stores at 80%).
- **Gates:** `exit 0 in 623 s	default	2026-11-27T21:59:57Z	sprint 371	4fdf5ac255d4785385227287cbffeb221392657c	d3ee603b43f1edac7db0504f1f38498a250863329b76d8a22e39b168691eb71d	root=51963afbe19a25aad6334f1e47f6ce3224a2bf67	rust=all python=0 samples=0	base=476e82490eb66bc912fe32b52eab08ffe3750246` (the supervisor's run at the close, its base Sprint 370's close 476e82490; the builder's run, `exit 0 in 634 s` at 1f1958df9 with base 53449a78a, is kept, refused by the record check while Sprint 370 had no close commit).
