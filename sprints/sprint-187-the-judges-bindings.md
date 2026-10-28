# Sprint 187 — The judges' bindings

```yaml
---
id: 187
status: active
phase: 6
pass_kind: functional
---
```

## scope

reviewer-2's Mediums 2 and 3 and Lows 3 and 5 of its review of Sprints 182-184 (the Decision "Sequence after Sprint
185"; Medium 4's gates stage landed in Sprint 185). Each is a judge whose input is not bound to the tree it judges.

- **The gates' passes, tracked and read** (Medium 2): `scripts/gates.sh` appends its pass line to
  `store/gates/passes.tsv`, committed with the sprint's work, with the tree ids of `crates/`, `scripts/`, `tests/`,
  `tools/`, `src/` and `store/` (less `store/gates/`) taken at the run's start from a temporary index of the work tree
  (`git add -A`, `git write-tree`). From this sprint `signal_report --check` finds a closed entry's work commit (subject
  "Sprint N: ...") and compares each tree id with that commit's, naming the first that differs; the lines of
  `build/gates-passes.txt` for Sprints 179-186 move into the tracked file as they are, so a fresh clone's gates pass.
- **A stale `case_runs` refused** (Medium 3): `harness/case-runs-rust.toml` runs `cargo run --release --quiet --bin
  case_runs -- --rust-translator`, which builds from the current sources and signs through the runner; `case_runs`
  embeds at build time HEAD and the sha256 of `git diff HEAD -- crates` (a build script) and refuses at start, naming
  both digests, when the tree's differs. The harness's `implementation` text names the 435 Rust-compiled classes.
- **Record ids** (Low 3): the harness comment states the convention: `<ns>` is the run's start in nanoseconds, the four
  profiles started together taking 30 ms steps from one start.
- **#72's rule** (Low 5): WORKING_AGREEMENT gains KIT_DIARY #72's rule: a card claiming the translator lifts N classes
  quotes the lift's per-class output before its commit.

## prerequisites

- 186 (closed)

## context_files

- `scripts/gates.sh`, `crates/signal_report/src/main.rs` (`closed_too_early`), `crates/signal_report/tests/check.rs`,
  `harness/case-runs-rust.toml`, `crates/w2m_vm/src/bin/case_runs.rs`, `.cargo/config.toml`, `scripts/sign_and_run.sh`

## signal contract

### Emits

- On four `case-runner-rust` records written through the new harness command: Sprint 183's verdicts, unchanged.

### Invariants

- No Python file added or changed; every record's verdicts as Sprint 183's.

## artifact contract

### Files modified

- `scripts/gates.sh`, `store/gates/passes.tsv` (new), `crates/signal_report/src/main.rs`, `tests/check.rs`,
  `harness/case-runs-rust.toml`, `crates/w2m_vm/build.rs` (new), `crates/w2m_vm/src/bin/case_runs.rs`,
  `WORKING_AGREEMENT.md`; four records

### Command exit codes

- `signal_report --check`, `case_runs`, the case runner and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** `--check` passes over Sprints 177-187 from the tracked file; the four records hold Sprint 183's
  counts (268, 177, 107, 26 classes); `case_runs` started from a tree it was not built from refuses.
- **Planted faults:** (1) a source line changed after the gates' pass and before the work commit: `--check` names the
  tree that differs; (2) `case_runs` run as built over a changed `crates/` file: it refuses, naming its digest.

## done criteria

A closed sprint's gates pass is bound to its work commit's trees, and no record is written by a binary built from
other sources.

## self-review checklist

- [x] Scope names one verifiable property: each judge's input bound to its tree.
- [x] context_files cite the code each binding lives in.
- [x] Emits: the records, unchanged.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: the judges' bindings.
