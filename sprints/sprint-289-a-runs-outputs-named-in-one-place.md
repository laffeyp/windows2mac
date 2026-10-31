# Sprint 289 — A run's outputs named in one place

```yaml
---
id: 289
status: done
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Low 1 at 288, settled as no defect today and a rule kept by nothing: the `--stop` file's stamp stands for
the four files the game run writes beside it under its name (`<stop>.unity.log`, `.calls.tsv`, `.calls.strings.tsv`,
`.in-flight.tsv`), each named by its own `format!` (`w2m-vm.rs` 908, 972, 976, 979), and a run's start removes the
files it is told to write and an exit ledger's children, not these. Every run that writes its stop writes all four
today; a change that made one conditional would leave an earlier run's beside a fresh stamp.

- **The work:** `crates/w2m_vm/src/run_outputs.rs`, the files a `w2m-vm` run writes in one place: the flags that name
  them, the stop's companions as one list and the name each takes, the files a run's arguments name, and `clear`,
  which removes at a run's start each such file with its stamp, the stop's companions and the exit ledger's children.
  `w2m-vm` takes its start, its stamps and its four companion names from it; `batch_run.rs` takes the companions'
  names from it.
- **Not here:** the stamp itself (`built_from`), the readers' checks.

## prerequisites

- 288 (closed)

## context_files

- `crates/w2m_vm/src/bin/w2m-vm.rs` 183-232 (the start and the stamps), 893-990 (the stop and its companions)
- `crates/w2m_vm/src/built_from.rs`; `crates/w2m_vm/tests/batch_run.rs` (`stamped_for`)
- BLACKBOARD Surfaced, reviewer-3's review at 288 on the file's form

## signal contract

### Emits

- None; a run's files, held by tests.

### Invariants

- The batch run, the guests and the family runs conform as held.

## artifact contract

### Files modified

- `crates/w2m_vm/src/run_outputs.rs` new, `crates/w2m_vm/src/lib.rs`, `crates/w2m_vm/src/bin/w2m-vm.rs`,
  `crates/w2m_vm/tests/batch_run.rs`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** a unit test's directory holding a stop, its four companions, every stamp, an exit ledger, two
  children `<ledger>.<pid>` and `<ledger>.blocks`: after `clear`, only `<ledger>.blocks` and a file no flag names are
  left. The batch-run step writes the four companions and their stop's stamp as before; the readers pass.
- **Planted faults**, each judge found by lookup to reach its line and to assert what the plant breaks:
  - (1) `clear` leaving the stop's companions. Judge: the unit test above, which asserts each companion gone.
  - (2) `clear` removing every `<ledger>.<anything>`, `.blocks` with them. Judge: the same test, which asserts
    `<ledger>.blocks` kept.
- **The review's case,** run by the binary: an earlier run's `first-stop.unity.log` and `.calls.tsv` left beside a
  stop, then `w2m-vm --game` into it failing before its run: no companion left.

## done criteria

A run's files are named once, in `run_outputs`, and a run's start leaves none of an earlier run's files its stop's
stamp would stand for.

## self-review checklist

- [x] Scope names one verifiable property: a run's files named in one place and cleared from it.
- [x] context_files cite the binary's start and companions, the stamp's module, the reader and the review.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values, two plants with their judge, the review's case.
- [x] One concept: the set of files a run writes.

## result

- **The module** (628e19224): `run_outputs`, `WRITTEN`, `STOP_COMPANIONS`, `companion`, `written` and `clear`;
  `w2m-vm`'s start is `run_outputs::clear`, 30 lines of its own gone, and its four companion names call `companion`,
  no `trim_end_matches(".tsv")` left in it; `batch_run.rs`' `stamped_for` takes the companions from the list.
- **Expected values:** `a_runs_start_clears_an_earlier_runs_files` leaves `["b/other.tsv", "l/rust.tsv.blocks"]`; the
  batch-run step writes the stop, its stamp and the four companions; 172 tests in the pre-commit run, the step first.
- **The gates:** `scripts/gates.sh` exit 0 in 896 s, kept in `store/gates/passes.tsv` as sprint 289 at 628e19224 with root=29268bead and "rust=crates python=0 samples=0", rdeps(w2m_vm), 172 tests, base=bf504ba2d, Sprint 288's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: `clear` leaving the stop's companions; the test failed, left
  `["b/other.tsv", "b/stop.calls.strings.tsv", "b/stop.calls.tsv", "b/stop.in-flight.tsv", "b/stop.unity.log",
  "l/rust.tsv.blocks"]`.
- **Plant 2,** restored by `cp` and checked by `cmp`: `clear` removing every `<ledger>.<anything>`; the test failed,
  left `["b/other.tsv"]`, `l/rust.tsv.blocks` gone.
- **The review's case:** `build/stamp-check/` with an earlier run's stop, its stamp, its four companions, `out.txt`
  and `keep.tsv`; `w2m-vm --game /nonexistent-depot x.exe` into them exited 1, "x.exe: No such file or directory",
  and left `keep.tsv` alone.
