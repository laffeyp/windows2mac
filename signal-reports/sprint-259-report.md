# SIGNAL_REPORT — Sprint 259 — Where the VM's time goes with the cache

Made by `signal_report 259` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 259.

## 1. Observed

### scope_confirmation

> Deferred (Sprint 258), reviewer-3's Medium 1 at 258: on Sprint 257's like records the VM's compile time falls by about
> 700 ms with the cache (1,363 to 1,385 ms to 667 to 674) while the VM's run seconds do not (24.18 to 24.33 against
> 24.28 to 24.36). Card 5 installs inside the VM and can recover only what the host's path costs. This card measures,
> by run variants that differ in one thing each, how much of the VM's time goes to the cached code's words, to the
> `hvc 1` exits, and to the global cache and TLB invalidation each exit's return makes.

### work_performed

- 796590103 Sprint 259: card 5's design takes the measured VM time
- fd840f005 Sprint 259 card: where the VM's time goes with the cache

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None of its own: twelve measured runs, their figures in the card's result.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 259: its Rubber Duck Pass and close.
