# SIGNAL_REPORT — Sprint 032

## 1. Observed

### scope_confirmation

> Contract `windows.thread.last_error.set_get@0.2.0` judges three SetLastError/GetLastError pairs whose values the probe picks at run time, with the `exact` relation checked through each SetLastError line's args_sha256. The Windows reference, the model runtime and the managed VM conform 3 of 3 each; a constant, a thread-id echo and a register echo no longer pass; the candidate loop runs against 0.2.0 by default.

### signal_trace

Read from the records by command:

```
runs/judge-1791529699873420000: {'SESSION_INIT': 1, 'EXECUTION_STARTED': 3, 'EXECUTION_COMPLETED': 3, 'TRACE_INGESTED': 3, 'COMPARISON_COMPLETED': 3, 'SESSION_COMPLETED': 1} | verdicts {'conforms': 3} | relation {'exact': 3} | profile_kind {'windows_reference': 3} | model calls [] | outcome completed
runs/judge-1791529701004655000: {'SESSION_INIT': 1, 'EXECUTION_STARTED': 3, 'EXECUTION_COMPLETED': 3, 'TRACE_INGESTED': 3, 'COMPARISON_COMPLETED': 3, 'SESSION_COMPLETED': 1} | verdicts {'conforms': 3} | relation {'exact': 3} | profile_kind {'model': 3} | model calls [] | outcome completed
runs/judge-1791529701091251000: {'SESSION_INIT': 1, 'EXECUTION_STARTED': 3, 'EXECUTION_COMPLETED': 3, 'TRACE_INGESTED': 3, 'COMPARISON_COMPLETED': 3, 'SESSION_COMPLETED': 1} | verdicts {'conforms': 3} | relation {'exact': 3} | profile_kind {'managed_vm': 3} | model calls [] | outcome completed
runs/candidates-1791529836408254000: {'SESSION_INIT': 1, 'ModelUsage': 1, 'CANDIDATE_GENERATED': 1, 'EXECUTION_STARTED': 3, 'EXECUTION_COMPLETED': 3, 'TRACE_INGESTED': 3, 'COMPARISON_COMPLETED': 3, 'CANDIDATE_ACCEPTED': 1, 'SESSION_COMPLETED': 1} | verdicts {'conforms': 3} | relation {'exact': 3} | profile_kind {'managed_vm': 3} | model calls [('qwen2.5-coder:7b', 461, 36, 518)] | outcome completed
runs/candidates-1791529841605209000: {'SESSION_INIT': 1, 'ModelUsage': 2, 'CANDIDATE_GENERATED': 2, 'EXECUTION_STARTED': 6, 'EXECUTION_COMPLETED': 6, 'TRACE_INGESTED': 6, 'COMPARISON_COMPLETED': 6, 'CANDIDATE_REJECTED': 1, 'REDUCTION_COMPLETED': 1, 'DISCREPANCY_REPORTED': 1, 'DISCREPANCY_ROUTED': 1, 'CANDIDATE_ACCEPTED': 1, 'DISCREPANCY_RESOLVED': 1, 'SESSION_COMPLETED': 1} | verdicts {'discrepancy': 3, 'conforms': 3} | relation {'exact': 6} | profile_kind {'managed_vm': 6} | model calls [('planted', 0, 0, 0), ('qwen2.5-coder:7b', 565, 36, 431)] | outcome completed_with_discrepancies | grounds ['3 of 3 committed Windows reference executions of sha256:d210df33adce2f08 conform']
```

## 2. Expected

> Windows 3 of 3 conform with different values per run; model and VM 3 of 3 each; lockstep and state differential 0 differences; the constant, the thread id and `args[0]` rejected; the correct candidate accepted. Held, after two harness fixes (lockstep re-dispatch; the loop's guard trigger).

## 3. Delta

### dual_contract_self_grade

**signal contract:** pass (COMPARISON_COMPLETED relation `exact`; DISCREPANCY_REPORTED mismatch_class `exact`; S3, S6, S7 as Sprint 031; 0.5 tags only). **artifact contract:** gates.sh 0 (202 passed). **overall:** pass. The Sprint 031 defect is closed: the test that pinned it now asserts the constant is accepted under 0.1.0 and rejected under 0.2.0.

## 4. Hypothesis

> Both bugs this sprint lived in the verification harness, and both were reached only because the new probe exercised a path the old one never did: a return to an address already compiled, and a candidate whose every run fails. The harness needs programs chosen for its own control paths, not only for the guest behavior under test.
