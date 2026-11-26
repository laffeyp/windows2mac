# SIGNAL_REPORT — Sprint 360 — The instruction clock on two vCPUs

Made by `signal_report 360` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 360.

## 1. Observed

### scope_confirmation

> The Decision "Step 6's order weighed again at Sprint 360, G2 unmet" puts this card first. G2, the 68 families on two
> vCPUs, stands at 267 of 273 runs conforming. Two of the six held runs, `processes` 38189162864 and 38189164581
> (`store/family_runs/two-vcpus-refused-2026-11-26.tsv`), take the instruction clock. `vcpus_allowed` (`run.rs` 653-689)
> refuses them: "the instruction clock reads the running vCPU's own count". `process.c` 41-46 computes the clock in clock
> mode 2 as the instructions this process ran, at a set rate, plus the idle warp, from `monitor_word(0)`, the running
> vCPU's own count. The per-vCPU pages design left it open: the runs that take it "stay on one vCPU until a card decides
> how the counts combine" (`docs/design-each-vcpus-own-pages-2026-10-31.md` 106-109). This card decides that and builds
> it.

### work_performed

- c08297a0a Sprint 360 card: the instruction clock on two vCPUs; step 6's order weighed again, G2's two waiting cards first

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None: the clock's readings are the guest's own results, judged in the family runs' rows; no emit site changes.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 360: its Rubber Duck Pass and close.
