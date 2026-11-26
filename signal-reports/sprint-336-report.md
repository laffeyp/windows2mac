# SIGNAL_REPORT — Sprint 336 — Vocabulary 0.13: the two-vCPU run's words

Made by `signal_report 336` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 336.

## 1. Observed

### scope_confirmation

> The order in the Decision "Step 6 restored to the user's fix (b), and the order after Sprint 332" (2026-11-24) puts
> vocabulary 0.13 next, after the gates' stall (Sprints 334 and 335). The card that follows it has `w2m-vm --vcpus N`
> write a substrate record its two-vCPU tests' judges read, and emits CAPABILITY_PROBED where `w2m-vm` makes its refusal
> checks. Today vocabulary 0.12 (`signals/rust-current.json`) has no word for any of the following, so that record
> cannot carry them:

### work_performed

- a420a5b48 Sprint 336 card: vocabulary 0.13; Sprint 335's three Decisions and its Deferred close

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - The samples record (`harness/vocabulary-samples.toml`) carries every live harness kind. It also carries
>   EXECUTION_STARTED with `vcpus` 2 once, MEASUREMENT_RECORDED with each of the three new `metric` values and one
>   `holder` once, and CAPABILITY_PROBED with each of the three new `capability` values once.
> - *(Annotated 2026-11-25, Sprint 340: the samples record is not kept under `store/runs/`, so this sprint's Signal
>   Report, which reads that directory alone, shows the three tags above NOT FIRED. A kept samples record breaks rules
>   the records hold: run on 2026-11-25 into the dated record `vocabulary-samples-1795660404000000000`, it replayed at
>   level 2 (128 frames), and `check_records --since 2026-10-17` held 18 violations in it, exit 1 (T2 1, T3 3, T4 2, S2
>   8, S4 2, S8 2), since the samples write each harness tag at sample values, not in the order or the counts a session
>   writes them; the gates' records check over a new record would fail on it. The samples run under a temporary root in
>   the gates' samples stage (`scripts/gates.sh` 336-341), where that run held EXECUTION_STARTED 3 times, one with
>   `vcpus` 2, MEASUREMENT_RECORDED 38 times and CAPABILITY_PROBED 20 times.)*

## 3. Delta

### emits_check

- `CAPABILITY_PROBED`: NOT FIRED
- `EXECUTION_STARTED`: NOT FIRED
- `MEASUREMENT_RECORDED`: NOT FIRED

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 336: its Rubber Duck Pass and close.
