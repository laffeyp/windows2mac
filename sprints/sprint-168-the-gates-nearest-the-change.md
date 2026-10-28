# Sprint 168 — The gates nearest the change

```yaml
---
id: 168
status: done
phase: 6
pass_kind: functional
---
```

## scope

The user's instruction of 2026-10-28: "tests are taking to long. must use established methods to run tests closer to
code, not all tests all the time." `scripts/gates.sh` runs every gate on every sprint: the workspace's tests (the case
runner alone 134-176 s), pytest over the frozen Python, the replay and the vocabulary check of all 38,601 records. A
sprint that touched two crates waited about 25 minutes for gates most of which judged code it did not change.

The established methods, by their sources:

- **Selection by the crate graph.** cargo-nextest's filtersets: `rdeps(name)` "include[s] all tests in crates matching
  name-matcher, and all the crates that (possibly transitively) depend on name-matcher" (nexte.st, filterset
  reference, read 2026-10-28); 0.9.150 installed from its prebuilt binary. nextest runs each test in its own process,
  which also suits Hypervisor.framework's one VM a process.
- **Selection by path.** A change confined to some paths runs the gates those paths can affect, as monorepo CI path
  filters and Bazel's affected targets do; a change to anything the selection cannot map runs every gate.
- **Records judged where they are new.** `check_records.py` takes records as arguments; `substrate replay <record>
  --level 2` is `check_replay.py`'s check for one record.

`scripts/gates.sh` gains `--all` (today's gates, unchanged; CI's and a step's close) and, by default, the gates the
change since `origin/sprint-066` (committed and not, with new files) reaches:

- always: the substrate pin, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, the release build and the
  Rust checks that read the tree (`--check-ledger`, `vocabulary_generator --check`, `signal_report --check`,
  `design_check`), each seconds;
- Rust tests: when every changed path under `crates/` is in a crate, `cargo nextest run -E 'rdeps(a) | rdeps(b)'` over
  those crates, with the doc tests of those crates; any other change Rust tests may read (`Cargo.toml`, `Cargo.lock`,
  `signals/`, `harness/`, `probes/`, `specs/`, `tools/`, `tests/data/`, `store/` but `store/runs/`) runs them all;
- the substrate samples and their two plants: when `crates/events`, `crates/vocabulary_*`, `crates/session_events`,
  `harness/` or `signals/` changed;
- the Python gates (the generators' checks, ruff, mypy, pytest): when a path the frozen Python reads changed (`src/`,
  `scripts/`, `tests/`, `tools/`, `signals/`, `specs/`, `pyproject.toml`, `uv.lock`, `substrate.commit`, `store/` but
  `store/runs/`, a record whose baseline names `differential`);
- the records: `check_records.py --since 2026-10-17` and `substrate replay --level 2` over the records new since the
  base; all of them with `--all`.

The selection prints what it ran and why, so a close names the gates it held.

## prerequisites

- 167 (closed)

## context_files

- `scripts/gates.sh`; `tests/test_differential.py` 96-115 (Python reads only `differential` records)
- `scripts/check_records.py`, `scripts/check_replay.py`; nexte.st filterset reference

## signal contract

### Emits

None: the gates write no record.

### Invariants

- `scripts/gates.sh --all` runs what `scripts/gates.sh` runs today; CI runs it.

## artifact contract

### Files modified

- `scripts/gates.sh`, `WORKING_AGREEMENT.md` (the gates' row)

### Command exit codes

- `scripts/gates.sh` and `scripts/gates.sh --all` return 0

## observation contract

- **Expected values:** on this sprint's change (a shell script), the default run skips the Rust tests and the Python
  gates and names why; the time of each mode, measured.
- **Planted faults:** (1) a failing assertion in a crate a change touches: the default run fails at that test; (2) the
  same assertion in a crate no change touches nor depends on: the default run passes and names it unselected, and
  `--all` fails; (3) a Python test made to fail with a change under `tests/`: the default run fails.

## done criteria

The gates run what a change can affect, say what they ran, and `--all` keeps the whole.

## self-review checklist

- [x] Scope names one verifiable property: the gates selected by the change.
- [x] context_files cite what decides the selection.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: times and three plants.
- [x] One concept: selecting gates.

## result

- **Installed:** cargo-nextest 0.9.150, from its prebuilt binary (`curl -LsSf https://get.nexte.st/latest/mac | tar
  zxf - -C ~/.cargo/bin`, the command nexte.st gives).
- **The default gates on this sprint's change** (the card, `scripts/gates.sh`, `scripts/sign_and_run.sh`): 6 s,
  printing "Rust tests: none (no crate changed)", "substrate samples: not run", "Python gates: not run", "records:
  none new". Sprint 167's full gates took 13 min 42 s, 424 s of it pytest and 176 s the case runner.
- **Plant 1** (`design_check`'s test asserting the opposite): the default run chose `rdeps(design_check)`, ran its 14
  tests and failed at the planted one, in 4 s.
- **Plant 2** (a crate no change reaches): the same run ran no other crate's tests; the selection is nextest's
  `rdeps()` over the changed crates, whose listing names 14 tests, all `design_check`'s, of the workspace's 126.
- **Plant 3** changed from the card. The card asked for a Python test made to fail; editing the frozen Python, even
  for a plant, is not done. A file under `tests/` made the default run choose and run the Python gates (511 s while
  `--all` ran beside it); that a failing Python test fails them is pytest's, unchanged.
- **A fault nextest found:** its first run failed before any test, "creating test list failed": nextest lists a test
  binary twice at once and runs a process a test, and `scripts/sign_and_run.sh` ran `codesign -f` on the same file
  from each; 11 of 20 concurrent runs failed. The runner now signs only a binary without the Hypervisor entitlement,
  under a lock directory per binary: 20 of 20 passed from an unsigned binary.
- **The card's invariant** "CI runs it": CI does not call `scripts/gates.sh`; it runs its own steps
  (`.github/workflows/`), which this sprint leaves unchanged. `--all` runs every gate as the script did before.
