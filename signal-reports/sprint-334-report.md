# SIGNAL_REPORT — Sprint 334 — Each gates step timed

Made by `signal_report 334` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 334.

## 1. Observed

### scope_confirmation

> Every sprint waits on one `scripts/gates.sh` run, and the run has grown: 4,928 s at Sprint 331, 5,845 s at 332, 6,243 s
> at 333. Sprint 332 placed part of it and left three figures unexplained (Deferred (Sprint 332), "The gates' time not
> spent in `codesign -d`"): nextest's listing at about 22.5 s a test binary with one 9.85 s signature check each; about
> 50 minutes between nextest's end and the timing stage's output that the log does not place; and a test binary's first
> launch in a series at 15 to 30 s with its signature stamp matching and no `codesign -d` run. The Decision "The gates'
> stall, a card after 333" (2026-11-24) orders this card: put a time on each gates step's start and end, and measure each
> of the three figures before any change. This card measures; it changes no step's work. The change the figures call for
> is the next card's, written from this card's numbers.

### work_performed

- d5a265bd4 Sprint 334 card: each gates step timed; Sprint 333's four Decisions

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a build script's timing rows, no runtime behavior.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 334: its Rubber Duck Pass and close.
