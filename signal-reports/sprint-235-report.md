# SIGNAL_REPORT — Sprint 235 — The 17 classes and the 6

Made by `signal_report 235` from 1 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 235.

## 1. Observed

### scope_confirmation

> The Decision "Sequence after Sprint 214" as annotated at Sprint 234, plan step 4's "456 of 458 classes", and
> BLACKBOARD Deferred "(Sprint 178, partial)".

### work_performed

- c2616c789 Sprint 235: the 17 classes and the 6
- 6c387f234 Sprint 235 card: the 17 classes and the 6

### signal_trace

```
class-* 1 records: SESSION_INIT 1, CANDIDATE_GENERATED 458, COMPARISON_COMPLETED 458, CANDIDATE_ACCEPTED 454, REDUCTION_COMPLETED 4, DISCREPANCY_REPORTED 4, DISCREPANCY_ROUTED 4, SESSION_COMPLETED 1; producers failed 0; not finalised 0
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

> - The class judge's session, as in Sprint 222: per class CANDIDATE_GENERATED and COMPARISON_COMPLETED, then
>   CANDIDATE_ACCEPTED, or REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED per difference; one
>   SESSION_INIT and one SESSION_COMPLETED.

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 454 times
- `CANDIDATE_GENERATED`: fired 458 times
- `COMPARISON_COMPLETED`: fired 458 times
- `DISCREPANCY_REPORTED`: fired 4 times
- `DISCREPANCY_ROUTED`: fired 4 times
- `REDUCTION_COMPLETED`: fired 4 times
- `SESSION_COMPLETED`: fired 1 time
- `SESSION_INIT`: fired 1 time

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 235: its Rubber Duck Pass and close.
