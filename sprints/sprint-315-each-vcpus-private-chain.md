# Sprint 315 — Each vCPU's private chain

```yaml
---
id: 315
status: active
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

