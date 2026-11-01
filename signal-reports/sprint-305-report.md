# SIGNAL_REPORT — Sprint 305 — A split in the Arm order

Made by `signal_report 305` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 305.

## 1. Observed

### scope_confirmation

> Reviewer-3's Medium 1 at 304: Sprint 304 writes the new table, then the descriptor, then flushes through `FLUSH_STUB`,
> so the requesting vCPU may hold the old block and the new table's entry at once, which break-before-make forbids at
> BBM 0 (Sprint 303); Arm leaves the outcome a TLB conflict abort or the silent use of either entry, and Deferred
> (Sprint 304)'s revisit could see only the first. With every other vCPU stopped, the strict order costs one exit a split.
> A split happens inside a request (`map_pages` writes page entries into the new table in the same host call), so the
> host cannot return to the guest between the steps; it runs the requester's vCPU itself, from inside the split.

### work_performed

- 80970c399 Sprint 305: a split in the Arm order, its flush run from inside it
- 41491cec9 Sprint 305 card: a split in the Arm order

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

See the BLACKBOARD tail entry for Sprint 305: its Rubber Duck Pass and close.
