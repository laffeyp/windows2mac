# Sprint 315 — Each vCPU's private chain

```yaml
---
id: 315
status: done
phase: 6
pass_kind: functional
---
```

## scope

The per-vCPU pages design's card 4c2b (`docs/design-each-vcpus-own-pages-2026-10-31.md`, "The cards" 2, and "The
copied tables after load", Sprint 314: no run-time writer writes a table a private chain copies, so none is made ahead).

- **The work:** `page_descriptor`'s refusal, by name and before any write, of a split at level 0 or 1 under 0xFE or
  0xFF and of a split or leaf inside the monitor area's or the process area's first gigabyte; at a vCPU's creation
  past the first, its level-0 table and the
  tables to its 18 pages in its `VCPU_STRIDE` slot, each copying the shared table with the vCPU's entries put in, the
  set-once monitor words copied to its page, its `VcpuPages` and its `TTBR0_EL1`; a run on the instruction clock
  refused on more than one vCPU, by name, beside Sprint 308's reasons. The run loop still runs one vCPU.
- **Not here:** the threads (4c3).

## prerequisites

- 314 (closed)

## context_files

- the per-vCPU pages design as Sprint 314 amends it; `tables.rs`, `guest.rs`, `monitor.rs` (`set_system_registers`),
  `run.rs` (`vcpus_allowed`); `guest_layout` (`VCPU_AREA`, `VCPU_STRIDE`); `tests/litmus.rs`

## signal contract

### Emits

- None; page tables.

### Invariants

- Every run as held; one vCPU in the run loop.

## artifact contract

### Files modified

- `crates/w2m_vm/src/` (`tables.rs`, `guest.rs`, `run.rs`); a VM test of two vCPUs

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** every run as held; on two vCPUs of one VM, each with its own chain, raw AArch64 reading the
  monitor data page's first word at one address, each its own value (Sprint 104's judge); `TTBR0_EL1.CnP` 0 on each;
  a page the guest maps after vCPU 1's chain is made, read on both; `vcpus_allowed` naming the instruction clock.
- **Planted faults:** (1) vCPU 1 given vCPU 0's level-0 table: the first judge reads one value on both; (2) a split
  asked inside the process area's gigabyte: the refusal names it.

## done criteria

A second vCPU can be given its own monitor data page, state block and stack at the shared addresses, and no mapping
made after it misses it.

## self-review checklist

- [x] Scope names one verifiable property: each vCPU's own pages at one address, no copy left stale.
- [x] context_files cite the design, the table writers and the layout.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what its judge reads.
- [x] One concept: one vCPU's private chain, made once.

## result

- **The chain** (`Guest::private_chain(k)`, `guest.rs` 1168): vCPU `k`'s 18 pages in its `VCPU_STRIDE` slot, the
  monitor data page and the state block copied from vCPU 0's, the set-once words with them, then its tables after the
  pages: a copy of the level-0 table, and on each walk to a private page a copy of the shared table or a 2 MB block
  expanded into its pages with the block's attributes, the vCPU's leaves put in; 8 tables at most, refused past that or
  past the slot; the `TTBR0_EL1` value (the root's IPA, CnP 0) and the vCPU's `VcpuPages`. vCPU 0's tables are not
  touched. No caller in the run loop yet, which runs one vCPU.
- **The refusals** (`guest.rs`): `copied_range` (803), checked first in `page_descriptor`, refuses any descriptor in
  the monitor area's or the process area's first gigabyte, naming the range; a split of a level-0 or level-1 block
  under 0xFE or 0xFF is refused in the walk (1106). `map_pages` and `protect_code` reach their writes through
  `page_descriptor`. Every run as held shows no writer reaches them today.
- **The instruction clock:** `run::vcpus_allowed` takes `clock`, refusing more than one vCPU with "the instruction
  clock reads the running vCPU's own count"; `w2m-vm` passes `--instruction-clock` or `--game` at its first check
  (318) and a family start's clock once its options are read (389); the unit test holds the new reason.
- **The test** `private_chain::each_vcpu_reads_its_own_monitor_data_at_one_address` (`tests/private_chain.rs`, a module
  of `tests/all.rs`): the qpc guest, vCPU 1's chain made, 0xA0A0 written at vCPU 0's monitor data and 0xB1B1 at vCPU
  1's; an arena page mapped after the chain (a split under level-0 entry 0, which both chains share) holding 0x1A7E; a
  split asked at `PROCESS_VA + 0x20_0000` refused by name; then two vCPUs, each on its own thread, loading through
  `ldr x0, [x1]; hvc #0x23` at the monitor area's free words: (0xA0A0, 0xB1B1) at `MONITOR_VA + MONITOR_DATA`,
  (0x1A7E, 0x1A7E) at the late page, `TTBR0_EL1` bit 0 clear on both.
- **Plant 1,** restored by `cp` and checked by `cmp`: `private_chain` returning vCPU 0's `ttbr`; the test failed,
  "each vCPU's monitor data at one address", left (41120, 41120), right (41120, 45489).
- **Plant 2,** restored likewise: the copied-range refusal switched off (`.filter(|_| false)`); `page_descriptor`
  split the process area's block and the test failed at private_chain.rs:89, "a split in the process area's first
  gigabyte: 98304". The card named the plant as the split asked; the split is the test's own request, and the plant is
  the refusal taken away.
- **Two work commits:** 44b4730bc went in after `scripts/pre-commit.sh` failed its format check on the new test, the
  two joined by `;`; 3a8f36028 formats the test, its commit gated by `&&` on a passing run, and the gates judge it.
- **The gates:** `scripts/gates.sh` exit 0 in 979 s, kept in `store/gates/passes.tsv` as sprint 315 at 3a8f36028 with root=988d51705 and "rust=crates python=0 samples=0", 187 tests, 822 ns an exit at load 5.85 (Sprint 313: 651 ns at load 4.22; one run each, the rise not measured to a cause), base=eafec35d2, Sprint 314's close; multi-vCPU: not run (Python gates not run)

