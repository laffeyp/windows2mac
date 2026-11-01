# SIGNAL_REPORT — Sprint 310 — The guest behind one lock

Made by `signal_report 310` from 0 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 310.

## 1. Observed

### scope_confirmation

> The run-loop design's card 4b as Sprint 309 decided it (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`, "How
> the threads share `Guest`"; BLACKBOARD Decision "Architecture: how the vCPU threads share `Guest`"): the whole `Guest`
> behind one `Mutex`, taken once per served exit inside the runtime word, its acquire asserting the word in debug builds
> (card 2's guard and plant). On one vCPU the run loop owns the lock, so outside a served exit it reaches `Guest` by
> `Mutex::get_mut`, no lock taken; card 4c turns those reaches into each thread's copies and view.

### work_performed

- d67426f18 Sprint 310: the lock held Sync, a view of the mapping in place of its pointer
- afc8727bc Sprint 310: the guest behind one lock
- 70b07cf37 Sprint 310 card: the guest behind one lock

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a lock and its guard.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 310: its Rubber Duck Pass and close.
