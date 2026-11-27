# SIGNAL_REPORT — Sprint 365 — The child on the run's vCPUs; the run loop's own refusal of the instruction clock above one vCPU

Made by `signal_report 365` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 365.

## 1. Observed

### scope_confirmation

> Card 4f4 of `docs/design-a-process-that-starts-another-2026-11-26.md` ("The cards that follow", item 4). The design is
> the spec. Since Sprint 364 a process that starts another runs at once with its child on several vCPUs, but the child
> runs on one: `child_start` gives it no vCPU count and `vcpus_allowed` refuses a child above one ("a child process runs
> on one", `run.rs` 658-690). This card runs the child on the run's vCPUs. G3's batch run needs it, since the game starts
> UnityCrashHandler64.exe as a child.

### work_performed

- 4ba3d0117 Sprint 365 card: the child on the run's vCPUs; the run loop's own refusal of the instruction clock above one vCPU (card 4f4)

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None: how many vCPUs a child runs on, and where the clock's refusal lives; no emit site changes. The family runs'
>   records are unchanged.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 365: its Rubber Duck Pass and close.
