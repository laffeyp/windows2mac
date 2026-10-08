# SIGNAL_REPORT — Sprint 025

## 1. Observed

### scope_confirmation

> Put the generated decoder differential on a substrate record as one comparison with routed discrepancies, and gave the judge run the session open and close it lacked.

### work_performed

- `generated_differential.py --record`: SESSION_INIT, COMPARISON_COMPLETED, DISCREPANCY_REPORTED/ROUTED per class, SESSION_COMPLETED; class examples in `store/programs/`.
- `judge_traces.py`: SESSION_INIT first; SESSION_COMPLETED after the last COMPARISON_COMPLETED, with the discrepancy count from the verdicts.
- Run roots named by `time.time_ns()` in five scripts after a same-second collision corrupted one scratch record.
- Judged the new runner run (evt24, 2 executions) with observed exit codes and elapsed times.

### signal_trace

Read back from the records with `substrate.api.read_record`, application events only:

```
runs/gendiff-1791496598044137000:   SESSION_INIT 1, COMPARISON_COMPLETED 1 (discrepancy), DISCREPANCY_REPORTED 300, DISCREPANCY_ROUTED 300 (contract_revision 300), SESSION_COMPLETED 1 (completed_with_discrepancies, 300)
runs/judge-1791496574903423000:     SESSION_INIT 1, EXECUTION_STARTED 2, EXECUTION_COMPLETED 2, TRACE_INGESTED 2, COMPARISON_COMPLETED 2 (conforms), SESSION_COMPLETED 1 (completed, 0)
runs/judge-1791496574995151000:     SESSION_INIT 1, EXECUTION_STARTED 10, EXECUTION_COMPLETED 10, TRACE_INGESTED 10, COMPARISON_COMPLETED 10 (conforms), SESSION_COMPLETED 1 (completed, 0)
```

### signal_trace_tags

- `SESSION_INIT`, `COMPARISON_COMPLETED`, `DISCREPANCY_REPORTED`, `DISCREPANCY_ROUTED`, `SESSION_COMPLETED`, `EXECUTION_STARTED`, `EXECUTION_COMPLETED`, `TRACE_INGESTED`

## 2. Expected

> All declared tags; no `implementation_repair` because length_differs is 0; T1 and T2 on both runs. All held.

## 3. Delta

### dual_contract_self_grade

**signal contract:** declared tags fired; counts match the worklist (300); T1/T2 hold on all three records. First firing of `DISCREPANCY_REPORTED` and `DISCREPANCY_ROUTED` in the project. *pass*

**artifact contract:** every `minimized_program_sha256` resolves under `store/programs/` (300 of 300); pytest, ruff, mypy 0. *pass*

**overall:** pass

## 4. Hypothesis

> 300 classes route to contract_revision and none can route to implementation_repair while length agrees. The routing is only as good as the outcome classes; an iclass difference caused by a decoder routine (not a spec row) would be misrouted. The first such case will show whether the rule needs a second input (which row matched).
