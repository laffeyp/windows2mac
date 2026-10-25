# SIGNAL_REPORT — Sprint 150 — The case runner on a record: its verdicts as the differential's events

Made by `signal_report 150` from 1 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 150.

## 1. Observed

### scope_confirmation

> The link-3 review's Medium 5: four functional sprints whose code runs 290,548 cases put nothing on a record, and each
> Signal Report reads "(no record)"; WORKING_AGREEMENT's "Signals that bind" says "None new" is not an Emits for code
> that runs. The judge the case runner replaces, `scripts/vm_oracle.py --record`, put its verdicts on a substrate record
> through `src/w2m/differential.py` (285 lines, read in full): one unit per instruction class, the Intel CPU under
> Windows the reference.

### work_performed

- 33f464a20 Sprint 150: the case runner's verdicts on a substrate record
- ffdb0dc01 Sprint 150: its card, before its code (the case runner on a record; review Medium 5)

### signal_trace

```
case-runner-1792916257296645000: SESSION_INIT 1, EXECUTION_STARTED 22, EXECUTION_COMPLETED 22, TRACE_INGESTED 22, CANDIDATE_GENERATED 387, COMPARISON_COMPLETED 387, CANDIDATE_ACCEPTED 387, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    1.546  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz; ..., purpose=conformance, vocab_version=0.10)
  t=    1.695  case_runs -> EXECUTION_STARTED (execution_id=case-run-38011858047, initial_state_sha256=sha256:bacab947c0f952f1fc1c3a39b0a1fe807a3e9be5..., injected_inputs_sha256=sha256:6a9da703a67c379854cbdbefe9b643aa1189928a..., kill_
  t=   10.259  case_runs -> EXECUTION_COMPLETED (elapsed_seconds=8.674148333, execution_id=case-run-38011858047, exit_code=None, exit_status=exited, guest_line_count=107, trace_sha256=sha256:4b57f77fcb5fd71935b4166328566ce3d6587ad7...,
  t=   10.259  case_runs -> TRACE_INGESTED (execution_id=case-run-38011858047, line_count=107, thread_count=1, trace_sha256=sha256:4b57f77fcb5fd71935b4166328566ce3d6587ad7..., vocab_version=0.10)
  t=   10.377  case_runs -> EXECUTION_STARTED (execution_id=case-run-38011862284, initial_state_sha256=sha256:bacab947c0f952f1fc1c3a39b0a1fe807a3e9be5..., injected_inputs_sha256=sha256:6a9da703a67c379854cbdbefe9b643aa1189928a..., kill_
  t=   19.007  case_runs -> EXECUTION_COMPLETED (elapsed_seconds=8.736701833, execution_id=case-run-38011862284, exit_code=None, exit_status=exited, guest_line_count=107, trace_sha256=sha256:e6d5acb73ceb721e83a49b9a6e2cdfbb8591f8ef...,
  t=   19.007  case_runs -> TRACE_INGESTED (execution_id=case-run-38011862284, line_count=107, thread_count=1, trace_sha256=sha256:e6d5acb73ceb721e83a49b9a6e2cdfbb8591f8ef..., vocab_version=0.10)
  t=   19.057  case_runs -> EXECUTION_STARTED (execution_id=case-run-38029980267, initial_state_sha256=sha256:bacab947c0f952f1fc1c3a39b0a1fe807a3e9be5..., injected_inputs_sha256=sha256:c46496354439cd9f1633db5174ddb9b3ac777625..., kill_
  t=   21.953  case_runs -> EXECUTION_COMPLETED (elapsed_seconds=2.931998583, execution_id=case-run-38029980267, exit_code=None, exit_status=exited, guest_line_count=32, trace_sha256=sha256:d04e72c5938ad49685b5fd756eb66d1d06455550..., 
  t=   21.953  case_runs -> TRACE_INGESTED (execution_id=case-run-38029980267, line_count=32, thread_count=1, trace_sha256=sha256:d04e72c5938ad49685b5fd756eb66d1d06455550..., vocab_version=0.10)
  t=   66.149  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner:ADC@bacab947c0f9, candidate_sha256=sha256:bacab947c0f952f1fc1c3a39b0a1fe807a3e9be5..., contract_id=w2m.guest_state.fault_context, contract_version=0.2
  t=   66.149  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner:ADC@bacab947c0f9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38011862284+380299802
  t=   66.149  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner:ADC@bacab947c0f9, comparison_ids=['cmp-case-runner:ADC@bacab947c0f9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   66.149  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner:ADD@bacab947c0f9, candidate_sha256=sha256:bacab947c0f952f1fc1c3a39b0a1fe807a3e9be5..., contract_id=w2m.guest_state.fault_context, contract_version=0.2
  t=   66.149  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner:ADD@bacab947c0f9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38011862284+380299802
  t=   66.149  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner:ADD@bacab947c0f9, comparison_ids=['cmp-case-runner:ADD@bacab947c0f9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   66.150  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner:ADDPD@bacab947c0f9, candidate_sha256=sha256:bacab947c0f952f1fc1c3a39b0a1fe807a3e9be5..., contract_id=w2m.guest_state.fault_context, contract_version=0
  t=   66.150  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner:ADDPD@bacab947c0f9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38011862284+3802998
  t=   66.150  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner:ADDPD@bacab947c0f9, comparison_ids=['cmp-case-runner:ADDPD@bacab947c0f9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   66.181  case_runs -> SESSION_COMPLETED (accepted_count=387, discrepancy_count=0, elapsed_seconds=65.578376333, outcome=completed)
  ... EXECUTION_STARTED 19 more
  ... EXECUTION_COMPLETED 19 more
  ... TRACE_INGESTED 19 more
  ... CANDIDATE_GENERATED 384 more
  ... COMPARISON_COMPLETED 384 more
  ... CANDIDATE_ACCEPTED 384 more
```

### signal_trace_tags

- `CANDIDATE_ACCEPTED`
- `CANDIDATE_GENERATED`
- `COMPARISON_COMPLETED`
- `EXECUTION_COMPLETED`
- `EXECUTION_STARTED`
- `SESSION_COMPLETED`
- `SESSION_INIT`
- `TRACE_INGESTED`

## 2. Expected

> - `SESSION_INIT` {vocab_version 0.10, profile the CPU of the Intel runs, purpose conformance}
> - `EXECUTION_STARTED` / `EXECUTION_COMPLETED` {execution_id `case-run-<run id>`, exit_status exited} per run, 22
> - `CANDIDATE_GENERATED`, `COMPARISON_COMPLETED` {verdict conforms}, `CANDIDATE_ACCEPTED` per judged class, 387 each
>   on the current tree
> - `CANDIDATE_REJECTED`, `DISCREPANCY_REPORTED`, `DISCREPANCY_ROUTED` per class with a difference: 0 on the current tree
> - `SESSION_COMPLETED` {outcome completed, accepted_count 387, discrepancy_count 0}

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 387 times
- `CANDIDATE_GENERATED`: fired 387 times
- `CANDIDATE_REJECTED`: NOT FIRED
- `COMPARISON_COMPLETED`: fired 387 times
- `DISCREPANCY_REPORTED`: NOT FIRED
- `DISCREPANCY_ROUTED`: NOT FIRED
- `EXECUTION_COMPLETED`: fired 22 times
- `EXECUTION_STARTED`: fired 22 times
- `SESSION_COMPLETED`: fired 1 time
- `SESSION_INIT`: fired 1 time
- fired, not named in Emits: `TRACE_INGESTED` (22)

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 150: its Rubber Duck Pass and close.
