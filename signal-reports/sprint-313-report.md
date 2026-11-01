# SIGNAL_REPORT — Sprint 313 — The host's per-vCPU base

Made by `signal_report 313` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 313.

## 1. Observed

### scope_confirmation

> The per-vCPU pages design's card 4c2a (`docs/design-each-vcpus-own-pages-2026-10-31.md`, "The cards" 1; BLACKBOARD
> Decision "Architecture: each vCPU's own pages"). The host reads a vCPU's monitor data page and state block by fixed
> offset, `MONITOR_DATA + X` and `PROCESS_VA + STATE_OFF` turned into memory offsets, in about 100 places; on two vCPUs
> each must read the exiting vCPU's own pages, which card 4c2b puts at that vCPU's `VCPU_STRIDE` slot.

### work_performed

- 96939301f Sprint 313: the host's per-vCPU base
- 04f27f1dd Sprint 313 card: the host's per-vCPU base

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; the host's reads re-addressed.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 313: its Rubber Duck Pass and close.
