# SIGNAL_REPORT — Sprint 304 — A block split breaks before it makes

Made by `signal_report 304` from 0 records under store/runs/ whose baseline names one of the sprint's 5 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 304.

## 1. Observed

### scope_confirmation

> The second vCPU's precondition 4 (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`), sized by Sprint 303: a vCPU
> reads `ID_AA64MMFR2_EL1.BBM` 0, so a valid block made a table while another vCPU may hold it in its TLB needs the
> block invalid, a broadcast invalidation, then the table (the Arm ARM's break-before-make; without it the result is
> UNPREDICTABLE, a TLB conflict among the outcomes). `Guest::page_descriptor` (`guest.rs` 1064-1096) splits a block in
> one store today. Its callers, the two requests that can split:

### work_performed

- 36f751b13 Sprint 304: the flush exit has no exit-ledger row
- bdea165fa Sprint 304: the batch run holds its 35 splits
- 5e86ccbf1 Sprint 304: the split's order on the requester stated; card 4 stops for a split
- 07e802849 Sprint 304: a block split, then every TLB flushed before the vCPU runs on
- 219e99412 Sprint 304 card: a block split breaks before it makes

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

See the BLACKBOARD tail entry for Sprint 304: its Rubber Duck Pass and close.
