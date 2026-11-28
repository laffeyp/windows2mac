# Sprint 377 — The reference machine named exactly, and the runner's Windows workflows retired

```yaml
---
id: 377
status: active
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

- `crates/windows_machine/{src/lib.rs, tests/*.rs}`, `tools/windows_machine/profile.ps1`, `.github/workflows/`
  (two moved to `tools/retired-workflows/`), with others only where the class requires, reported to main when found. One concept: the reference
  machine named exactly, and no other machine able to stand in for it.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree
- one laptop probe job through `windows_machine` returns 0, its `profile.json` carrying the code-integrity state

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

GATES-LINE
