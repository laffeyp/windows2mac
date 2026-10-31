# SIGNAL_REPORT — Sprint 303 — Break-before-make for a block split

Made by `signal_report 303` from 0 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 303.

## 1. Observed

### scope_confirmation

> The run-loop design's card 3 and the second vCPU's precondition 4 (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`,
> "The second vCPU's preconditions" 4, "The cards" 3): `Guest::page_descriptor` (`crates/w2m_vm/src/guest.rs` 1064-1096)
> turns a valid block into a table by one Release store of the table's descriptor (1090-1091), after writing the
> table's 512 entries; while another vCPU may hold the block in its TLB, the Arm architecture requires
> break-before-make for a change of block size (Arm ARM, "Translation table entry changes", the BBM rules), unless the
> CPU implements FEAT_BBM at level 2, under which the old and new entries may both be in a TLB with no conflict abort,
> a TLB invalidation following. The guest runtime already invalidates every vCPU's TLB after a request that maps or
> unmaps (`tools/guest_runtime/runtime.h` 331-335).

### work_performed

- cd8e7ebd5 Sprint 303: hypervisor a test binary a file again; cargo test on macOS measured
- dcf9d9ae6 Sprint 303: the CPU's BBM level read; break-before-make needed
- 65e1451fb Sprint 303 card: break-before-make for a block split; the run-loop design's cards 2b and 4

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; an order of writes, held by a test and the runs.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 303: its Rubber Duck Pass and close.
