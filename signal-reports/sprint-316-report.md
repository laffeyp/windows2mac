# SIGNAL_REPORT — Sprint 316 — The host's switch on any vCPU

Made by `signal_report 316` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 316.

## 1. Observed

### scope_confirmation

> The run-loop design's card 4c3, first part (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`, card 4; split at this
> card into 4c3a, this, and 4c3b, the threads). The host's thread switch (`threads.rs` `switch`, "for vCPU 0 of one
> process") marks the thread it loads held by vCPU 0, writing 1 into the run entry's `vcpu` word (`E_VCPU`, 878), and
> refuses to load one another vCPU holds (864-869); but it picks the next ready thread without skipping one held
> elsewhere, so on two vCPUs it would stop the run where the C runtime's own switch picks another
> (`tools/guest_runtime/threads.c` 68, `held_elsewhere`; 293-294, the claim with `this_vcpu()`, the vCPU's `LOCK_OWNER`).

### work_performed

- e17e0cee0 Sprint 316: the host's switch on any vCPU
- 69c3352f8 Sprint 316 card: the host's switch on any vCPU

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; the switch's choice.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 316: its Rubber Duck Pass and close.
