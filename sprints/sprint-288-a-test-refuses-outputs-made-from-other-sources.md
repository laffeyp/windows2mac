# Sprint 288 — A test refuses outputs made from other sources

```yaml
---
id: 288
status: active
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

- **The gates:** GATES-LINE
