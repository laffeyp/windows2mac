# Sprint 351 — The 273 family runs on two vCPUs, held to Windows' rows

```yaml
---
id: 351
status: active
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-4's twentieth review, High 1, recorded as the Decision "The family runs on two vCPUs before the lock cards".
G2 in `docs/step-6-census-2026-10-30.md` asks for the 68 Windows function families on two vCPUs, 273 runs, each held to
Windows' rows. Today `scripts/gates.sh`' `family_runs()` runs each run of `store/family_runs/known-2026-10-29.tsv`
through `w2m-vm` on one vCPU. `crates/w2m_vm/tests/family_runs.rs` then holds its printed rows to its contract's rows,
or to `store/family_runs/sweep-held-2026-10-29.tsv`. Neither takes a vCPU count.

The design's next four cards (`docs/design-handles-and-sync-objects-2026-11-26.md`) change the handle table, the waits,
the releases and CloseHandle. The threads, sync-object and handle families judge exactly those, so they must run on two
vCPUs before those cards do.

- **The change:**
  - `family_runs()` runs every run on one vCPU, as today, and on two, by `w2m-vm --vcpus 2`, writing each run's
    printed rows and stop beside the one-vCPU ones under a name that says two.
  - `family_runs.rs` holds each two-vCPU run to the same contract rows, or the same held entry, as its one-vCPU run.
  - A run that starts a child process stops on two vCPUs at the host's refusal, `MACHINE_ON_ONE`
    (`crates/w2m_vm/src/host_request.rs` 79, 531), until 4f. Hold those runs by name to that stop in a list the test
    reads, each with its family and run, and count them.
- **Every other two-vCPU failure is a finding.** Quote the run, its stop and the rows that differ. A run that stops,
  hangs past its budget or answers differently from Windows on two vCPUs is a defect. Under the user's Decision of
  2026-10-18 it is fixed in this card: report it to the Supervisor at once with its evidence, so the card can be
  amended for its fix. Do not hold a failing run to its failure.
- **The judge's reach.** Before the change, run the families on two vCPUs once by hand and quote the count that pass,
  the count refused for a child process, and each other failure.
- **Time.** The family runs take about 37 s on one vCPU. Quote the two-vCPU step's time from `store/gates/steps.tsv`.
- **Not here:**
  - the child-process runs on two vCPUs (4f);
  - the game's batch run on two vCPUs;
  - the lock cards.

## prerequisites

- 350 (closed)

## context_files

- `scripts/gates.sh` (`family_runs()`, lines 205-260) and `crates/w2m_vm/tests/family_runs.rs` whole.
- `store/family_runs/known-2026-10-29.tsv` and `store/family_runs/sweep-held-2026-10-29.tsv`.
- `crates/w2m_vm/src/bin/w2m-vm.rs` (`--vcpus`) and `crates/w2m_vm/src/host_request.rs` 70-90 and 520-540
  (`MACHINE_ON_ONE`).
- `docs/step-6-census-2026-10-30.md` (G2) and `docs/design-handles-and-sync-objects-2026-11-26.md` (the cards that
  follow).
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None; the family runs' judge on two vCPUs.

### Invariants

- The one-vCPU family runs pass unchanged, every run held as before.

## artifact contract

### Files modified

- `scripts/gates.sh`, `crates/w2m_vm/tests/family_runs.rs`, and the list of runs refused for a child process (a new
  file under `store/family_runs/`, named with its date).

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The count of the 273 runs held to Windows' rows on two vCPUs, the count refused for a child process, by family.
  - Each other failure found and its fix, or the card's amendment.
  - The two-vCPU step's seconds.
- **Planted faults:**
  - (1) The two-vCPU runs skipped by `family_runs()`. The test fails, naming a missing two-vCPU output.
  - (2) One two-vCPU run's printed rows changed in its output file, on a scratch copy of `build/family_runs/`. The
    test fails, naming that run and its field.

  Before the change, read each judge's reach: the test failing with no two-vCPU outputs.

## done criteria

Every one of the 273 family runs runs on two vCPUs in the gates and is held to Windows' rows as on one vCPU, except the
runs that start a child process, which are held by name to the host's refusal until 4f.

## self-review checklist

- [x] Scope names one verifiable property: the family runs on two vCPUs held to Windows' rows.
- [x] context_files cite the gates' step, the test, its inputs, the vCPU flag and the refusal.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the counts, the findings, the time; two numbered plants, their reach read.
- [x] Sweet spot: `gates.sh` and `family_runs.rs`; one concept, G2's runnable part.

## result

GATES-LINE
