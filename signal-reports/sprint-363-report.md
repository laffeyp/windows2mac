# SIGNAL_REPORT — Sprint 363 — The request server: what crosses processes, answered one request at a time under `kernel`

Made by `signal_report 363` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 363.

## 1. Observed

### scope_confirmation

> Card 4f2 of `docs/design-a-process-that-starts-another-2026-11-26.md` ("The cards that follow", item 2; "The request
> server", "Which lock or request each crossing takes"). The design is the spec. On several vCPUs no Machine runs, and a
> thread in the first `w2m-vm` answers what crosses processes. This card builds that server and its client, judged by a
> test of two processes, before any run reaches it (card 4f3 connects it).

### work_performed

- 278b42a76 Sprint 363 card: the request server, what crosses processes answered one request at a time under kernel (card 4f2)

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None: a server no run reaches yet; no emit site changes. Every run's records are unchanged.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 363: its Rubber Duck Pass and close.
