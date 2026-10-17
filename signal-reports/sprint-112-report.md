# SIGNAL_REPORT — Sprint 112 — discrepancy reports that name their program and state

Made by `scripts/signal_report.py 112` from 526 records under store/runs/ whose baseline names one of the sprint's 2 commits. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 112.

## 1. Observed

### scope_confirmation

> In H4's place in the SDD hardening order (Decision 2026-10-17; Deferred 2026-10-17, Sprint 109). One concept: a
> DISCREPANCY_REPORTED says which program failed from which state, as v0.2 11.3 asks ("everything the generation system
> needs to reproduce the failure"). Today the class run reports `minimized_program_sha256` and `initial_state_sha256` as
> `sha256:` and 64 zeros, the family run the state so, and file_run.py both (789 such fields on the record); and no
> reduction precedes a class or family report (S5, 676 times).

### work_performed

- fdba4c17b Sprint 112: the sweep on b1ad0cfb; check_records.py knows a fixed defect only before its fix
- b1ad0cfb4 Sprint 112: discrepancy reports name their program and state

### signal_trace

```
class-* 458 records: SESSION_INIT 458, CANDIDATE_GENERATED 458, COMPARISON_COMPLETED 916, CANDIDATE_ACCEPTED 456, SESSION_COMPLETED 458, REDUCTION_COMPLETED 8, DISCREPANCY_REPORTED 8, DISCREPANCY_ROUTED 8; producers failed 0; not finalised 0
family-* 68 records: SESSION_INIT 68, CANDIDATE_GENERATED 68, COMPARISON_COMPLETED 273, CANDIDATE_ACCEPTED 68, SESSION_COMPLETED 68; producers failed 0; not finalised 0
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

> - From a class run with a discrepancy (FCOS, FSIN at each sweep): per difference class and judge,
>   REDUCTION_COMPLETED (`steps` 0, `preserved_mismatch` true, source equal to minimized, a case's instruction-byte
>   sha256) then DISCREPANCY_REPORTED with that program and a state hash other than the zero placeholder, then
>   DISCREPANCY_ROUTED; SESSION_INIT, CANDIDATE_GENERATED, two COMPARISON_COMPLETED and SESSION_COMPLETED as before.
> - From a family or file run with a discrepancy: DISCREPANCY_REPORTED with the probe binary's sha256 and the run's
>   start's state hash.

## 3. Delta

### emits_check

- `CANDIDATE_GENERATED`: fired 526 times
- `COMPARISON_COMPLETED`: fired 1,189 times
- `DISCREPANCY_REPORTED`: fired 8 times
- `DISCREPANCY_ROUTED`: fired 8 times
- `REDUCTION_COMPLETED`: fired 8 times
- `SESSION_COMPLETED`: fired 526 times
- `SESSION_INIT`: fired 526 times
- fired, not named in Emits: `CANDIDATE_ACCEPTED` (524)

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 112: its Rubber Duck Pass and close.
