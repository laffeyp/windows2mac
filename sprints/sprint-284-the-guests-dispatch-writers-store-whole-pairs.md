# Sprint 284 — The guest's dispatch writers store whole pairs

```yaml
---
id: 284
status: active
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Medium 1 and Low 1 at 283 (BLACKBOARD Surfaced). The dispatch table has three writers. Sprint 276 made the
host's (`run::dispatch_insert`) store each pair by one `STP` after a release fence. The other two still store a pair
word by word: the in-VM C compiler (`tools/guest_runtime/block_compiler.c` 145-161: on eviction `p[2] = p[0]; p[3] =
p[1]; p[1] = code; p[0] = target;`) and the installer (`crates/w2m_vm/src/monitor_code.rs` 696-700: `str` the code,
then `str` the target, and the way-1 move as two `str`s). The dispatcher reads a pair by one `LDP`, single-copy atomic
with FEAT_LSE2 (DDI 0487 B2.2.1.1; Sprint 276), so on two vCPUs a lookup between two of those stores can match a target
with the wrong block. On one vCPU the writer runs on the only vCPU, and nothing can read between its stores.

- **The work:**
  - `block_compiler.c`: a `store_pair` by one `stp` (inline assembly, as the file's `dsb`/`tlbi` and `dc cvau` are), and
    `dispatch_insert` storing each pair whole, the way-1 move first, then way 0.
  - The installer: `stp` for each pair, `ldp` of way 0 then `stp` of it at way 1, then `stp` of the new pair.
  - `python_image.rs`' `DEPARTURES` names `tools/guest_runtime/block_compiler.c` with its reason.
  - The design's table: the dispatch row names the three writers, each a whole-pair store; the reconstruction row names
    its three writers (the in-VM compiler, the installer, `HostCompiler::place`) and EnTSO on every vCPU as the fact
    that orders the guest's two, with `litmus.rs`' EnTSO plant as the evidence (Low 1).
  - The step 6 Decision lists it as the second vCPU's fifth precondition, met here, and names that card's race test: two
    vCPUs, one evicting a slot while the other looks up its old target, a two-store form planted.
- **Not here:** the race test (the second vCPU's card); the remaining modules' slice calls.

## prerequisites

- 283 (closed)

## context_files

- `tools/guest_runtime/block_compiler.c` 145-162, 281-316; `crates/w2m_vm/src/monitor_code.rs` 660-705
- `crates/w2m_vm/src/run.rs` (`dispatch_insert`); `crates/guest_runtime/tests/python_image.rs` 70-77
- `docs/design-guest-memory-on-several-vcpus-2026-10-31.md` (the table); `crates/w2m_vm/tests/run_loop.rs` 312-355, 547-580

## signal contract

### Emits

- None; a store's form, held by tests and the one-vCPU runs.

### Invariants

- One-vCPU runs unchanged: the batch run's stop and stream (runtime addresses by symbol plus offset, Sprint 272), the
  exit ledger, the cache's installs.

## artifact contract

### Files modified

- `tools/guest_runtime/block_compiler.c`, `crates/w2m_vm/src/monitor_code.rs`, `crates/guest_runtime/tests/python_image.rs`,
  `docs/design-guest-memory-on-several-vcpus-2026-10-31.md`, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the installer's words hold an `stp` at each pair and no lone `str` to the slot; the run-loop tests,
  the batch run and the image judge pass.
- **Planted faults**, each judge found by lookup to reach its line:
  - (1) the C's `store_pair` storing the code word plus 4: `run_loop.rs`'
    `each_compile_and_install_carries_the_instructions_retired_at_it` runs `qpc` uncached, asserts compiles inside the
    VM, so it reaches `dispatch_insert` at `block_compiler.c` 316, and asserts `ExitProcess { code: 0 }`.
  - (2) the installer's new pair storing the code word plus 4: `run_loop.rs`'
    `qpc_runs_with_its_blocks_installed_inside_the_vm` runs `qpc` with the installer inside the VM and asserts its stop.

## done criteria

Every writer of the dispatch table stores a pair by one whole-pair store, and the design's table names the three
writers of the dispatch and reconstruction tables and the fact each relies on.

## self-review checklist

- [x] Scope names one verifiable property: every dispatch writer stores whole pairs.
- [x] context_files cite both writers, the host's, the image judge, the table and the judges' lines.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each judge's reach looked up.
- [x] One concept: the dispatch table's writers.

## result
