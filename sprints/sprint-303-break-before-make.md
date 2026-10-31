# Sprint 303 — Break-before-make for a block split

```yaml
---
id: 303
status: active
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's card 3 and the second vCPU's precondition 4 (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`,
"The second vCPU's preconditions" 4, "The cards" 3): `Guest::page_descriptor` (`crates/w2m_vm/src/guest.rs` 1064-1096)
turns a valid block into a table by one Release store of the table's descriptor (1090-1091), after writing the
table's 512 entries; while another vCPU may hold the block in its TLB, the Arm architecture requires
break-before-make for a change of block size (Arm ARM, "Translation table entry changes", the BBM rules), unless the
CPU implements FEAT_BBM at level 2, under which the old and new entries may both be in a TLB with no conflict abort,
a TLB invalidation following. The guest runtime already invalidates every vCPU's TLB after a request that maps or
unmaps (`tools/guest_runtime/runtime.h` 331-335).

- **First, the lookup that sizes the card:** macOS's `sysctl` reports `hw.optional.arm.FEAT_LSE2: 1` and names no
  FEAT_BBM. `ID_AA64MMFR2_EL1.BBM` (bits 55:52) is read from a vCPU by `hv_vcpu_get_sys_reg` and recorded with its
  source. At level 2: the split stays one store, its invalidation the runtime's broadcast that follows the request,
  the reasoning and the register's value written into the design's precondition 4. Below level 2: the design's two
  steps, the block made invalid and the request marked to retry, the runtime's broadcast, the table written on the
  retry, the C runtime changed where the request returns.
- **Either way:** a count of valid blocks split, on the batch run's record, as the design asks; a test recording a
  split's writes in order.
- **Not here:** the second vCPU; a split's cost.

## prerequisites

- 302 (closed)

## context_files

- the design's precondition 4 and card 3; `guest.rs` 1064-1130 (`page_descriptor`, `map_pages`)
- `tools/guest_runtime/runtime.h` 331-335; the Hypervisor.framework headers for `hv_vcpu_get_sys_reg` and
  `HV_SYS_REG_ID_AA64MMFR2_EL1`
- the Arm ARM's rules for translation table entry changes and FEAT_BBM (DDI 0487), cited by section in the result;
  looked up while drafting: `ID_AA64MMFR2_EL1.BBM` 0b0010 is level 2 (the register's reference,
  df.lth.se/~getz/ARM/SysReg/AArch64-id_aa64mmfr2_el1.html), and level 2 guarantees the TLB "for the specific case
  where the only thing that changed is the size of the block" (QEMU commit 75d08a4, quoting the Arm ARM); without it
  the result is UNPREDICTABLE, a TLB conflict fault among the outcomes

## signal contract

### Emits

- None; an order of writes, held by a test and the runs.

### Invariants

- Every run as held; one vCPU.

## artifact contract

### Files modified

- `crates/w2m_vm/src/guest.rs`, a test; at BBM below level 2 also the C runtime's request return and the record's
  count; the design's precondition 4

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `ID_AA64MMFR2_EL1` and its BBM field; the count of valid blocks split on the batch run; the
  split's writes in the order the chosen form requires.
- **Planted faults:** (1) the split's writes reordered against the chosen form (the table's descriptor before its
  entries, or, below level 2, the table written without the invalid step): the recording test must name the write out
  of order.

## done criteria

A block split while another vCPU could hold the block in its TLB follows the Arm rule this CPU needs, shown by the
register and a test of the writes' order.

## self-review checklist

- [x] Scope names one verifiable property: the split's order under the CPU's BBM level.
- [x] context_files cite the design, the code, the runtime and the sources to look up.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and a plant changing what its judge reads.
- [x] One concept: a split's order; the register read first sizes it.

## result

