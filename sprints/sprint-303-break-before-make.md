# Sprint 303 — Break-before-make for a block split

```yaml
---
id: 303
status: done
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

- **The lookup, and the card narrowed by it:** a vCPU reads `ID_AA64MMFR2_EL1` = 0x1001001102001011, BBM 0: the
  guest is promised no FEAT_BBM, whatever the chip implements, and the host writes its tables, so a change of block
  size needs the break. The two steps (the block invalid and the request marked to retry, the runtime's broadcast,
  the table on the retry) change the C runtime's request return, a second concept past rule 6; they are card 304,
  with the count on the batch run and the recording test of the writes' order. This card keeps the read, its test and
  the design's precondition 4 stating the value.
- **The change** (dcf9d9ae6, and cd8e7ebd5 for the undo below): `hypervisor::sys_reg::ID_AA64MMFR2_EL1` (0xc03a, `hv_vcpu_types.h` 336);
  `tests/id_registers.rs`, `the_vcpu_reports_its_bbm_level`, printing the register and its BBM, failing on a level the
  Arm ARM does not define or one the design does not state, its own binary once the merge below is undone; the design's precondition 4 and
  card 3 amended; and a consequence of Sprint 300 named: Hypervisor.framework allows one VM a process, nextest giving
  each test its own, `cargo test` on macOS would not, said in `hypervisor`'s root and `first_exit.rs`' header, whose
  "each scenario is its own test file" the merge had made untrue.
- **Sprint 300's merge undone for `hypervisor`, by measurement:** `WORKING_AGREEMENT.md` 142 expects `cargo test` to
  exit 0; on this Mac `cargo test -p hypervisor --test all` exited 101, three tests panicking at `Vm::new`, the merged
  binary running its VM tests as threads of one process where Hypervisor.framework allows one VM. `first_exit.rs`'s
  header, "One VM per process, so each scenario is its own test file", had said why; Sprint 300 did not read it, and
  its judge, nextest's count, gives each test a process and could not see it. `hypervisor` is a binary a file again
  (`autotests` and `[[test]]` gone, `tests/all.rs` removed, the manifest saying why, the gates' timing step and
  `first_exit.rs`' header as before 300, by `diff`), and `cargo test -p hypervisor` exits 0. The work commit after
  dcf9d9ae6 carries it, the gates on dcf9d9ae6 stopped. `w2m_vm`'s one binary under `cargo test` on this Mac:
  72 passed and 8 failed at `Vm::new`, seven `run_loop.rs`'s (one file, one process before 300 too), one `unmapped_image_faults`', which the merge made share; `w2m_vm` keeps its one binary, its VM tests run under nextest, and `WORKING_AGREEMENT.md` 142 says so.
- **Expected values:** BBM 0; the register test passing; `cargo test -p hypervisor` exit 0.
- **The gates:** `scripts/gates.sh` exit 0 in 1167 s, kept in `store/gates/passes.tsv` as sprint 303 at cd8e7ebd5 with root=f79c0c87a and "rust=all python=0 samples=1", `--all` (the gates' script changed), 331 tests, base=606de58f8, Sprint 302's close, the run on dcf9d9ae6 stopped for the undo; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: the card's plant reordered the split's writes, which this card
  no longer changes; the plant set the design's stated level to "BBM 2"; the test failed, "this vCPU reports BBM 0,
  which the design's precondition 4 does not state".

