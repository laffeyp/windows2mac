# SIGNAL_REPORT — Sprint 343 — The process heap takes its own lock, not the runtime word

Made by `signal_report 343` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 343.

## 1. Observed

### scope_confirmation

> The second lock class of Change 1 in the host design is **a lock per heap**, "as Windows has one"
> (`docs/design-rust-host-2026-10-24.md` 95-140, citing Microsoft Learn, HeapCreate: "the heap serializes access within
> the calling process"). HeapAlloc is one of the game's four hot imports, the mix G6 measures. Sprint 342 built the None
> class. This card moves the process heap's services off the runtime word and onto the heap's own lock:

### work_performed

- 0d085b17f Sprint 343 card: the process heap takes its own lock, not the runtime word; Sprint 342's late Decision and Deferred

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; which lock the heap's services take, no new event.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 343: its Rubber Duck Pass and close.
