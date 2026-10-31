# Sprint 284 — The guest's dispatch writers store whole pairs

```yaml
---
id: 284
status: done
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

- **The C** (6f40f25fa): `block_compiler.c`'s `store_pair` stores a pair by one `stp`; `dispatch_insert` stores each
  pair whole, on eviction the first way's pair at the second way, then the new pair at the first.
- **The installer:** `stp(4, 5, 3, 16)` moves the first way's pair, `stp(0, 14, 3, 0)` or `stp(0, 14, 3, 16)` stores the
  new one, x0 the target and x14 the code as before. A new test, `the_installer_stores_each_dispatch_pair_whole`, holds
  the three `stp`s and no lone `str` of a pair's word after the installer's `add x3, x3, x9, lsl 4`; its first draft
  checked the whole installer and failed on the compile log's `str x0, [x3]`, x3 being the log's base before the slot's.
- **The image judge:** `DEPARTURES` names `tools/guest_runtime/block_compiler.c`; its 4 tests pass.
- **The design's table:** the dispatch row names the three writers, each storing whole pairs; the reconstruction row
  its three writers and EnTSO, with `litmus.rs`' plant, as the ground for the guest's two (Low 1). The step 6 Decision
  lists the fifth precondition met.
- **Expected values:** the installer's test; the 11 run-loop tests on the rebuilt image; before the commit, 168 `w2m_vm` and `guest_runtime` tests and 18 of `cpu_cases` and `translator`, which depend on `guest_runtime`.
- **The gates:** `scripts/gates.sh` exit 0 in 1014 s, kept in `store/gates/passes.tsv` as sprint 284 at 6f40f25fa with root=da93efc66 and "rust=all python=0 samples=0", every Rust test, 318 run, 318 passed, 1 skipped, base=1cb9ea741, Sprint 283's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`, `w2m-vm` rebuilt: `store_pair` storing the code word plus 4. The
  named judge reaches the line, a lookup showed, and passed in 2.27 s under it: a block entered one word late still ran
  `qpc` to code 0, and the test asserts no more. On the live game, as the gates run it, the run stopped "HostFailure op
  5: guest runtime fault: ELR 0x7f0000209fa0 ESR 0x96000046 (EC 0x25)" where the held stop is "ReferenceEnd".
- **Plant 2,** restored by `cp` and checked by `cmp`: the installer's new pair at the first way stored with its words
  swapped, `stp(14, 0, 3, 0)`, not the card's "code word plus 4", which needs a register the installer's sequence does
  not have free; `qpc_runs_with_its_blocks_installed_inside_the_vm` failed, "the installs left: 7 right: 6".
