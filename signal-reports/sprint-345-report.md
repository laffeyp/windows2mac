# SIGNAL_REPORT — Sprint 345 — The TLS and FLS slots take their own lock, not the runtime word

Made by `signal_report 345` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 345.

## 1. Observed

### scope_confirmation

> The third lock class of Change 1 is **the TLS and FLS slots' lock** (`docs/design-rust-host-2026-10-24.md` 113-114:
> "for TlsAlloc, TlsFree, FlsAlloc and FlsFree, which change the process's slot bitmap"). In the guest's lock order it
> comes after the handle table and before a sync object, a heap and the run-table word. Sprint 343 built the heap's lock,
> and this card moves the slot services off the runtime word onto the slots' own lock.

### work_performed

- 3522feb7f Sprint 345 card: the TLS and FLS slots take their own lock, not the runtime word; the probe thread_slots_race

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; which lock the slot services take, no new event.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 345: its Rubber Duck Pass and close.
