# SIGNAL_REPORT — Sprint 031

## 1. Observed

### scope_confirmation

> Ran the candidate loop for real: a local model generated SetLastError and GetLastError from the contract, the managed VM ran the skeleton with the candidate in kernel32, the unchanged comparator judged it, and a separate guard accepted or rejected it; a planted wrong candidate was reported, routed, and repaired by the model.

### signal_trace

Read from the records by command:

```
runs/candidates-1791507278435079000: {'SESSION_INIT': 1, 'ModelUsage': 1, 'CANDIDATE_GENERATED': 1, 'EXECUTION_STARTED': 3, 'EXECUTION_COMPLETED': 3, 'TRACE_INGESTED': 3, 'COMPARISON_COMPLETED': 3, 'CANDIDATE_ACCEPTED': 1, 'SESSION_COMPLETED': 1} | verdicts {'conforms': 3} | model calls [('qwen2.5-coder:7b', 458, 36, 1044)] | outcome completed
runs/candidates-1791507396445439000: {'SESSION_INIT': 1, 'ModelUsage': 2, 'CANDIDATE_GENERATED': 2, 'EXECUTION_STARTED': 6, 'EXECUTION_COMPLETED': 6, 'TRACE_INGESTED': 6, 'COMPARISON_COMPLETED': 6, 'CANDIDATE_REJECTED': 1, 'REDUCTION_COMPLETED': 1, 'DISCREPANCY_REPORTED': 1, 'DISCREPANCY_ROUTED': 1, 'CANDIDATE_ACCEPTED': 1, 'DISCREPANCY_RESOLVED': 1, 'SESSION_COMPLETED': 1} | verdicts {'discrepancy': 3, 'conforms': 3} | model calls [('planted', 0, 0, 0), ('qwen2.5-coder:7b', 531, 36, 372)] | outcome completed_with_discrepancies
```

## 2. Expected

> Run 1: whatever the model's first candidate earns, on the record. Run 2: rejection, reduction, report, implementation_repair route, repair attempt with discrepancy_id, acceptance, resolution. Held.

## 3. Delta

### dual_contract_self_grade

**signal contract:** pass (S3: acceptance cites 3 earlier conforming comparisons; S6, S7: route after report, resolution after route on the same path; S8 exercised in tests). **artifact contract:** gates.sh 0 (183 passed). **overall:** pass, with a contract defect found: 0.1.0 accepts a constant.

## 4. Hypothesis

> The loop's weakest part is now the contract, not the code: an acceptance condition built from one fixed scenario is satisfied by a constant. The next contract work should give the candidate a scenario it cannot predict before it runs.
