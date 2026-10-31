# SIGNAL_REPORT — Sprint 293 — The run loop on several vCPUs, designed

Made by `signal_report 293` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 293.

## 1. Observed

### scope_confirmation

> Step 6's next move after census row D7: the run loop on a second vCPU, under today's runtime word (the census's "What
> the counts say": "a second vCPU under it is the frozen host's shape"), judged by gate items G2 to G4. The host design
> (`docs/design-rust-host-2026-10-24.md`) fixes the target: a thread per vCPU that runs it and serves its exits, its
> host locks (`modules`, `vm_map`, `code_page`, `files`, `file`, `output`, `trace`, `sched`, `vcpu_handle`) and no lock
> across `hv_vcpu_run`; the guest-memory design's choice 6 fixes the order: the runtime word outermost, no lock over
> `Guest` whole. Neither says how today's `Guest`, 45 fields the run loop holds as `&mut Guest`, divides.

### work_performed

- 735844f18 Sprint 293: the run loop on several vCPUs, designed
- 28baf1bb5 Sprint 293 card: the run loop on several vCPUs, designed

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a design.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 293: its Rubber Duck Pass and close.
