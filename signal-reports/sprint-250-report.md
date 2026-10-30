# SIGNAL_REPORT — Sprint 250 — A written block's old code

Made by `signal_report 250` from 0 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 250.

## 1. Observed

### scope_confirmation

> A departure from the frozen host, found by the lookup for the translation cache's design (2026-10-30): after a guest
> write to a page of translated code, the Rust host can hand back the in-VM compiler's old code for a block on that page.
> The cache will put its blocks in the same maps, so the maps are made right first.

### work_performed

- a1f6305a0 Sprint 250: the held first stop moves by the fix, three compiles more
- d3dd37eb2 Sprint 250: a block a guest write dropped is not handed back by hvc 1
- 3a6edf954 Sprint 250 card: a written block's old code

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None of its own; the gates' batch run, which must still conform.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 250: its Rubber Duck Pass and close.
