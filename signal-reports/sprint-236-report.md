# SIGNAL_REPORT — Sprint 236 — The MMX lowering

Made by `signal_report 236` from 1 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 236.

## 1. Observed

### scope_confirmation

> Plan step 4's "456 of 458 classes" and the remainder of Sprint 235: the three MMX-only classes (EMMS, MOVDQ2Q,
> PSHUFW) the class judge counts as conforming only on the frozen Python's words, and every MMX-register form the batch
> run's calls hold.

### work_performed

- 58b40ae42 Sprint 236: the MMX lowering
- e0236e44a Sprint 236 card: the MMX lowering

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

> - The class judge's session again, as in Sprint 235.

## 3. Delta

### emits_check

- fired, not named in Emits: `CANDIDATE_ACCEPTED` (454), `CANDIDATE_GENERATED` (458), `COMPARISON_COMPLETED` (458), `DISCREPANCY_REPORTED` (4), `DISCREPANCY_ROUTED` (4), `REDUCTION_COMPLETED` (4), `SESSION_COMPLETED` (1), `SESSION_INIT` (1)
- the card's Emits names no tag, and the sprint's records fired the ones above

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 236: its Rubber Duck Pass and close.
