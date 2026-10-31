# Sprint 288 — A test refuses outputs made from other sources

```yaml
---
id: 288
status: done
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Low 1 at 287, the class: six tests judge files the gates' batch-run step writes, and a run of the tests
after a change and before that step judges the old files. Sprint 286's pre-commit run did, naming `create_step+0x4`
from Sprint 285's record against the frozen `+0x0`; Sprint 284's did the same. WORKING_AGREEMENT's rule since 286 runs
the step first; nothing checks it. `built_from`'s tree would not have caught it: its inputs leave out `tools/`, whose C
`guest_runtime` compiles when a run starts (`crates/guest_runtime/src/lib.rs` 37-39, 155).

- **The readers:** `crates/w2m_vm/tests/` `batch_run.rs`, `batch_record.rs`, `block_judge.rs` (`build/batch_run/`),
  `exit_ledger.rs` (`build/exit_ledger/`), `family_runs.rs` (`build/family_runs/`), and
  `crates/call_compare/tests/call_compare.rs` (`build/batch_run/`).
- **The work:**
  - `built_from`: `RUN_INPUTS`, the paths a run reads from the work tree beyond its build, `tools`; a stamp, the build's
    tree (`BUILT_FROM`) and `RUN_INPUTS`' tree as the run found them, one line; `stamp`, which writes it as
    `inputs.tree` in a directory; `stale`, None when a directory's stamp is the work tree's trees, else why not: no
    stamp, or both stamps named.
  - `w2m-vm` computes the stamp once at its start and writes it in the directory of each file it is told to write
    (`--output`, `--stop`, `--exit-ledger`, `--judge-blocks`); a child process writes none.
  - Each reader refuses a stale directory as it refuses a missing file: a panic naming why under
    `W2M_REQUIRE_CASE_RUNS=1`, which the gates set, a reason on stderr and no judgement otherwise.
  - WORKING_AGREEMENT's amendment at 286 names the check.
- **Not here:** the case runner's inputs (`build/probe-runs`, `build/case_runner`), downloaded and pinned, not written
  by the gates; the module census.
- **With this card's commit,** reviewer-3's Medium 1 at 287's disposition: the design's choice 6, the lock order
  (`docs/design-guest-memory-on-several-vcpus-2026-10-31.md`), held out of 287's close commit as a work file.

## prerequisites

- 287 (closed)

## context_files

- `crates/w2m_vm/src/built_from.rs`; `crates/w2m_vm/build.rs`; `crates/guest_runtime/src/lib.rs` 37-39, 155
- `crates/w2m_vm/src/bin/w2m-vm.rs` (the flags, 229-289; the dispatch, 298-335); `harness/w2m-vm-game.toml` 22
- the six readers named above; `scripts/gates.sh` 137-187
- WORKING_AGREEMENT, "Before a work commit" and "A plant's judge"

## signal contract

### Emits

- None; a refusal, held by tests.

### Invariants

- The batch run, the guests and the family runs conform as held: each reader's assertions unchanged.

## artifact contract

### Files modified

- `crates/w2m_vm/src/built_from.rs`, `crates/w2m_vm/src/bin/w2m-vm.rs`, the six readers, WORKING_AGREEMENT.md,
  `docs/design-guest-memory-on-several-vcpus-2026-10-31.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `inputs.tree` in `build/batch_run/`, `build/exit_ledger/` and `build/family_runs/` after the
  batch-run step, each the work tree's trees; the six readers pass on them.
- **The case the card exists for,** run after the gates' batch-run step: a comment added to
  `tools/guest_runtime/block_compiler.c`, restored by `cp` and checked by `cmp`, then the six readers under
  `W2M_REQUIRE_CASE_RUNS=1`: each fails naming its directory and the two stamps.
- **Planted faults**, each judge found by lookup to reach its line and to assert what the plant breaks:
  - (1) `stale` comparing the build's tree alone, `RUN_INPUTS` dropped. Judge: a unit test in `built_from` that stamps
    a directory, changes a file under a temporary repository's `tools/`, and asserts `stale` names it; and the case
    above, where the six readers would pass.
  - (2) `w2m-vm` writing no stamp beside `--exit-ledger`'s file. Judge: `exit_ledger.rs`' reader after the batch-run
    step, which fails "no stamp".

## done criteria

A run of the six readers against outputs made before a change to `crates/` or `tools/` fails naming both trees, and
passes on outputs the batch-run step made from the work tree.

## self-review checklist

- [x] Scope names one verifiable property: a reader refuses outputs made from other sources.
- [x] context_files cite the stamp's module, the binary's flags, the readers, the gates' step and the rules.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values, the motivating case and two plants, each judge's reach named.
- [x] One concept: outputs tied to the sources they stand for.

## result

- **The stamp** (f7b1a7a72): `built_from::RUN_INPUTS` (`tools`), `run_stamp`, `stamp`, `stale` and `fresh`;
  `w2m-vm` writes `inputs.tree` once a run in the directory of each `--output`, `--stop`, `--exit-ledger` and
  `--judge-blocks` file; the six readers call `fresh` before they read; `call_compare` takes `w2m_vm` as a
  dev-dependency for it.
- **Before any stamp:** the six readers run under `W2M_REQUIRE_CASE_RUNS=1` on the outputs of Sprint 287's gates: 9
  tests failed, each "no stamp" with its directory, the 18 that read no gates output passed.
- **Expected values:** after the batch-run step, `build/batch_run/`, `build/exit_ledger/` and `build/family_runs/`
  each stamped "513340fa613def9a0f83b2c14d5b8375af1dc82d 45fed359274fbfdb34c143080b6457e73093fcba", the readers
  passing: 170 tests in the pre-commit run, rdeps(w2m_vm) with `call_compare` among them.
- **The case the card exists for,** restored by `cp` and checked by `cmp`: a comment on `block_compiler.c`'s first
  line after the batch-run step; the six readers' 9 tests failed, each naming its directory, "made from the sources
  513340fa6… 45fed3592… and the work tree's are 513340fa6… 25db8f5af…", the `tools/` tree alone differing.
- **A race the readers exposed** (92ea752da): `tree_of`'s temporary index was named by the process alone, and the
  readers' tests, threads of one process under `cargo test`, now call it at once; each call's index is its own, by the
  process and a count, held by `threads_taking_a_tree_at_once_agree`, 64 calls on 16 threads. Planted back, restored
  by `cp` and checked by `cmp`: the test failed 3 runs of 3, git's "Unable to create
  '…/w2m-built-from-<pid>.lock': File exists". The gates on f7b1a7a72 were stopped before their end for it, no pass
  row written.
- **A stamp a file, not a directory, a departure from the card** (ce1541d55; reviewer-3's Low 1 at 288): a
  timed record leaves out `--judge-blocks` (`scripts/batch-run-record.sh` 55-56) and still writes `--output` and
  `--stop` into `build/batch_run/`, so the directory's stamp stood beside an earlier run's `whole-blocks.tsv`. Each
  file a run is told to write is removed with its stamp at the run's start, an exit ledger's children's `<path>.<pid>`
  with it, and stamped `<file>.tree` after the run; the `--stop` file, written last (`w2m-vm.rs`' `game`), stands for
  the files its run writes beside it; each reader checks the file it judges, `fresh` taking the work tree's stamp once
  a process. The gates on 92ea752da were stopped before their end for it, no pass row written. Removing the file at
  the start closes the same hole in time: stamping each listed file the run left would have stamped an earlier run's
  file when a run failed before writing it. The plants below were run on the directory's form; they are run again on
  this one.
- **The gates:** `scripts/gates.sh` exit 0 in 792 s, kept in `store/gates/passes.tsv` as sprint 288 at 6d13df494 with root=897528694 and "rust=all python=0 samples=0", every Rust test, an input outside one crate changed, 320 run, 320 passed, 1 skipped, base=b7a96648b, Sprint 287's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: `stale` comparing the build's tree alone;
  `a_stamp_goes_stale_when_tools_changes` failed at `built_from.rs` 167, "a changed tools/ left the stamp fresh".
- **Plant 2,** restored by `cp` and checked by `cmp`: `w2m-vm` writing no stamp beside `--exit-ledger`'s file, the
  batch-run step run on it; `build/batch_run/` stamped by the planted build (2f161926a…), `build/exit_ledger/` keeping
  the earlier build's stamp (513340fa6…), which the gates' step does not remove; `exit_ledger.rs`' reader failed, "was
  made from the sources 513340fa6… 45fed3592… and the work tree's are 2f161926a… 45fed3592…", not the card's "no
  stamp": a stamp left by an earlier run names that run's sources and is refused as theirs.
- **A file checked by its own stamp** (6d13df494): `batch_run.rs`' `written` checked a guest's output,
  `guest-<name>.txt`, by the game's `first-stop.tsv` stamp, the file another run writes; a game run that left no stop
  file would have failed the guests' comparison naming the game's stamp, or, without `W2M_REQUIRE_CASE_RUNS=1`, at an
  `unwrap` with no reason. The game's stop stands for the files under its name alone, the guests' stops for
  `guests.tsv`, any other file for itself. Found in the check of ce1541d55, its gates stopped before their end, no
  pass row.
- **On the file's form** (ce1541d55), each restored by `cp` and checked by `cmp`:
  - *A run that fails before writing:* `build/stamp-check/` with an earlier run's `first-stop.tsv`, its stamp and
    `out.txt`; `w2m-vm --game /nonexistent-depot x.exe` into them exited 1, "x.exe: No such file or directory", and
    left the directory empty, no file and no stamp.
  - *Reviewer-3's case:* after the batch-run step, a comment on `block_compiler.c`, then a timed record
    (`scripts/batch-run-record.sh … --time-compile`): `first-stop.tsv`'s stamp became 363148b97… 25db8f5af…,
    `whole-blocks.tsv`'s stayed 363148b97… 45fed3592…; `block_judge.rs` failed, "whole-blocks.tsv was made from the
    sources 363148b97… 45fed3592… and the work tree's are 363148b97… 25db8f5af…".
  - *Plant 1 again:* `a_stamp_goes_stale_when_tools_changes` failed at `built_from.rs` 221, "a changed tools/ left
    the stamp fresh".
  - *Plant 2 again,* `--exit-ledger` dropped from the files `w2m-vm` stamps, the batch-run step run on it:
    `first-stop.tsv` stamped b8a64b25c… 45fed3592… by the planted build, no `rust.tsv.tree`, the gates' step removing
    `rust.tsv.*` with it; `exit_ledger.rs`' reader failed, "build/exit_ledger/rust.tsv.tree: no stamp", the card's
    expectation.
