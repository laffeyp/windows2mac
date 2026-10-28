# SIGNAL_REPORT — Sprint 195 — The bindings that fail open

Made by `signal_report 195` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 195.

## 1. Observed

### scope_confirmation

> The Decision "Sequence after Sprint 193", reviewer-2's Medium 4 and Lows 2 and 4 of its review of Sprints 190-192, and
> Sprint 194's Observation 3. `signal_report --check` finds a sprint's work commit by its subject and the close commit by
> "Sprint N closed:", so a commit under any other subject between the card and the close is checked by nothing; its
> multi-vCPU rule matches the phrase "multi-vCPU tests passed" and passes a close that says nothing; its test of the gated
> tree copies `gates.sh`'s steps rather than running them; `docs/`, which `design_check` reads, is a bookkeeping path; and
> `vex_store_reach` lifts nine encodings picked by hand. Sprint 194 made the third multi-line edit outside the Edit tool
> within three sprints (189, 191, 194), so the rule becomes a check.

### work_performed

- a64b1b435 Sprint 195: the bindings that fail open
- 30249c7c0 Sprint 195 card: the bindings that fail open

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None: no producer runs; the judges are `signal_report`'s tests, `vex_store_reach` and `edit_guard`'s tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 195: its Rubber Duck Pass and close.
