# SIGNAL_REPORT — Sprint 222 — The class judge

Made by `signal_report 222` from 1 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 222.

## 1. Observed

### scope_confirmation

> The Decision "Sequence after Sprint 214" as annotated at Sprint 221 (reviewer-3's Medium 3 of its refreshed project
> review), and plan step 4's gate item "456 of 458 classes" (census row 3, "absent" on Rust).

### work_performed

- af30038c7 Sprint 222: the class judge
- e5795a7e3 Sprint 222 card: the class judge

### signal_trace

```
class-* 1 records: SESSION_INIT 1, CANDIDATE_GENERATED 458, COMPARISON_COMPLETED 458, CANDIDATE_ACCEPTED 437, REDUCTION_COMPLETED 4, DISCREPANCY_REPORTED 4, DISCREPANCY_ROUTED 4, SESSION_COMPLETED 1; producers failed 0; not finalised 0
```

### signal_trace_tags

- `CANDIDATE_ACCEPTED`
- `CANDIDATE_GENERATED`
- `COMPARISON_COMPLETED`
- `DISCREPANCY_REPORTED`
- `DISCREPANCY_ROUTED`
- `REDUCTION_COMPLETED`
- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - Per class: SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED (`managed_vm`), then CANDIDATE_ACCEPTED, or
>   REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED; SESSION_COMPLETED. All are in vocabulary 0.12.

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 437 times
- `CANDIDATE_GENERATED`: fired 458 times
- `COMPARISON_COMPLETED`: fired 458 times
- `DISCREPANCY_REPORTED`: fired 4 times
- `DISCREPANCY_ROUTED`: fired 4 times
- `REDUCTION_COMPLETED`: fired 4 times
- `SESSION_COMPLETED`: fired 1 time
- `SESSION_INIT`: fired 1 time

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 222: its Rubber Duck Pass and close.
