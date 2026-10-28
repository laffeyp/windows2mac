# SIGNAL_REPORT — Sprint 168 — The gates nearest the change

Made by `signal_report 168` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 168.

## 1. Observed

### scope_confirmation

> The user's instruction of 2026-10-28: "tests are taking to long. must use established methods to run tests closer to
> code, not all tests all the time." `scripts/gates.sh` runs every gate on every sprint: the workspace's tests (the case
> runner alone 134-176 s), pytest over the frozen Python, the replay and the vocabulary check of all 38,601 records. A
> sprint that touched two crates waited about 25 minutes for gates most of which judged code it did not change.

### work_performed

- d7260e37d Sprint 168: the gates nearest the change
- 529c29c39 Sprint 168: its card, before its code (the gates nearest the change)

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> None: the gates write no record.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 168: its Rubber Duck Pass and close.
