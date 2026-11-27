# Sprint 371 — Probes run on the laptop: the Mac's half of the Windows reference machine, in Rust

```yaml
---
id: 371
status: active
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

GATES-LINE
