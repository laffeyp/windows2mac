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

## result

- **The gates' passes, tracked and read:** `gates.sh` stages `crates`, `scripts`, `tests`, `tools`, `src` and `store`
  into a temporary index at its start, drops `store/gates`, and writes each directory's tree id into the pass line's
  seventh column; the line goes to `store/gates/passes.tsv`, which holds the lines of Sprints 179-186 moved from
  `build/gates-passes.txt`. `signal_report --check` reads the tracked file, fails without it on CI as anywhere, and
  from Sprint 187 finds the work commit ("Sprint 187: ...") and compares its trees (`trees_moved`), naming the first
  that differs. `ci.yml` checks out with `fetch-depth: 0`. The check's tests moved to the sprints whose rules they hold
  (170-171, 180-181, 185-186), and `a_gates_run_must_have_tested_the_work_commits_trees` builds a two-commit repository.
- **A stale `case_runs` refused:** `crates/w2m_vm/build.rs` stamps the `crates/` tree id (`W2M_CRATES_TREE`);
  `w2m_vm::built_from::check` takes the work tree's at start, and `case_runs` refuses to write a record when they
  differ. The harness runs `cargo run --release --quiet --bin case_runs -- --rust-translator` and names the 435
  classes the Rust compiles.
- **Record ids** and **#72's rule:** the harness comment states the convention; WORKING_AGREEMENT gains "A coverage
  claim quotes its count", the multi-line note for 183-186, and the multi-vCPU stage of Sprint 185.
- **Records:** four `case-runner-rust` records through `cargo run` (1793213020000000000, -030000000, -060000000,
  -090000000), 268, 177, 107 and 26 classes as Sprint 183's, `check_records` 0 violations each; the ledger holds 16.
- **Plant 1** (the tree comparison dropped, `if false && n >= FIRST_TREE_BOUND_SPRINT`):
  `a_gates_run_must_have_tested_the_work_commits_trees` failed, "assertion `left == right` failed left: Some(0) right:
  Some(1)". Restored byte for byte.
- **Plant 2** (a line appended to `crates/w2m_vm/src/ledger.rs` after the build): `case_runs` refused, "case_runs was
  built from crates/ 78e35f17829e34c4d936092ead326742004aa883 and the work tree's is
  aac8c1a24e4834f4b894e544ca15bf94192b176e". Restored byte for byte; the tree id returned to 78e35f17.
- **Departures:** the digest is a tree id from a temporary index, not the sha256 of `git diff HEAD`, so an untracked
  file counts; every multi-line edit used the Edit tool.
