# SIGNAL_REPORT — Sprint 364 — Processes at once on several vCPUs: the child on one, the request server answering it, the deadlock verdict across processes

Made by `signal_report 364` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 364.

## 1. Observed

### scope_confirmation

> Card 4f3 of `docs/design-a-process-that-starts-another-2026-11-26.md` ("The cards that follow", item 3; "The form on
> several vCPUs", "Which lock or request each crossing takes", "The deadlock rule", "The stop"). The design is the spec.
> On several vCPUs a process that starts another is refused today at `MACHINE_ON_ONE` (`host_request.rs`). This card lifts
> that refusal: the parent runs on the run's vCPUs, its child on one, the two at once, with no Machine and no turns, and
> what crosses between them goes through Sprint 363's request server (`cross_process.rs`).

### work_performed

- 958177d36 Sprint 364 card: processes at once on several vCPUs, the child on one, the request server answering it, the deadlock verdict across processes (card 4f3)

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None: how processes run on several vCPUs; no emit site changes. The six runs' records, which stopped at the refusal,
>   become runs held to Windows' rows; every other run's records are unchanged.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 364: its Rubber Duck Pass and close.
