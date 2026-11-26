# SIGNAL_REPORT — Sprint 341 — The call ring safe for concurrent callers

Made by `signal_report 341` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 341.

## 1. Observed

### scope_confirmation

> Sprint 339's proof stopped Change 1's None class. All nine of its services, and every other logged service, return
> through `w2m_logged` (`tools/guest_runtime/runtime.c` 60-70). That function writes the process-wide call ring
> (`struct w2m_ring`, `runtime.h` 84-95, at `W2M_RING_VA`, `RING_OFF` 0x5000, size 0x40000) by unlocked load, add and
> store. The built `_w2m_logged` reloads `count` before each field's store. On two vCPUs this has two faults: counts are
> lost, and a record can be written past the ring, into the exception area at `EXCEPTION_OFF`.

### work_performed

- c62a25911 Sprint 341 card: the call ring safe for concurrent callers; Sprint 340's Decision and the record check's note

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; how the runtime writes its call ring, no new event.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 341: its Rubber Duck Pass and close.
