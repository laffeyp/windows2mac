# SIGNAL_REPORT — Sprint 238 — The case runner on Rust alone, the Python words fixture retired

Made by `signal_report 238` from 2 records under store/runs/ whose baseline names one of the sprint's 4 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 238.

## 1. Observed

### scope_confirmation

> Reviewer-3's direction at 235 and the split of card 237: since Sprint 237 the Rust translator compiles every case code
> and every conforming class is judged on its words, so no judge needs the frozen Python lowering's words.

### work_performed

- 5d773cd6d Sprint 238: the class judge's session on c57eb5095; its index and the ledger
- c57eb5095 Sprint 238: rustfmt's layout for the new tests and the candidate tuples
- 44fa7fed8 Sprint 238: the case runner on Rust alone
- 39fc5b1a6 Sprint 238 card: the case runner on Rust alone, the Python words fixture retired

### signal_trace

```
class-* 2 records: SESSION_INIT 2, CANDIDATE_GENERATED 916, COMPARISON_COMPLETED 916, CANDIDATE_ACCEPTED 912, REDUCTION_COMPLETED 8, DISCREPANCY_REPORTED 8, DISCREPANCY_ROUTED 8, SESSION_COMPLETED 2; producers failed 0; not finalised 0
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

> - The class judge's session, its baseline naming the Rust translator only.

## 3. Delta

### emits_check

- fired, not named in Emits: `CANDIDATE_ACCEPTED` (912), `CANDIDATE_GENERATED` (916), `COMPARISON_COMPLETED` (916), `DISCREPANCY_REPORTED` (8), `DISCREPANCY_ROUTED` (8), `REDUCTION_COMPLETED` (8), `SESSION_COMPLETED` (2), `SESSION_INIT` (2)
- the card's Emits names no tag, and the sprint's records fired the ones above

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 238: its Rubber Duck Pass and close.
