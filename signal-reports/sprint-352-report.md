# SIGNAL_REPORT — Sprint 352 — A handle's lookup holds a reference on its object, and every blocked wait keeps its objects

Made by `signal_report 352` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 352.

## 1. Observed

### scope_confirmation

> Card 2 of `docs/design-handles-and-sync-objects-2026-11-26.md` ("The cards that follow", lines 344-353; the rule at
> 166-231). Sprint 350 built the two counts, `handles` and `refs`, in `struct object`. It gave a reference to the waiter
> entries of a semaphore, an auto-reset event, a mutex and a wait for all.

### work_performed

- 191ce59fb Sprint 352 card amendment: the probe wait_closed_kinds, for waits on closed handles of every kind
- 1b17f5ef1 Sprint 352 card: a handle's lookup holds a reference on its object, and every blocked wait keeps its objects

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; when an object's entry is freed, and what a wait reads.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 352: its Rubber Duck Pass and close.
