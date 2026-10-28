# SIGNAL_REPORT — Sprint 178 — The model's legacy SSE slice

Made by `signal_report 178` from 4 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 178.

## 1. Observed

### scope_confirmation

> Step 4's model (census row 4), its fourth slice. After Sprint 175 the case runner counts, and does not judge, 35,663
> model cases in 142 classes, every one an `sse`, `vex`, `vexs` or `fma` operation, which `model_step` refuses. This
> slice takes the legacy-encoded ones, 6,845 cases: set `semantics`' 40 classes (2,085 a run, two runs), set 049's
> CMPSD_XMM, CVTPD2DQ and CVTTPD2DQ (425), set 050's 23 classes (3,513) and set 065c's MPSADBW (822). The VEX-encoded
> 26,733 (sets 049 and 049b) come after.

### work_performed

- 9ea51a1f5 Sprint 178: the model's legacy SSE slice
- ecb354725 Sprint 178's card: the model's legacy SSE slice

### signal_trace

```
case-runner-rust-1793193981524123000: SESSION_INIT 1, CANDIDATE_GENERATED 235, COMPARISON_COMPLETED 235, CANDIDATE_ACCEPTED 233, CANDIDATE_REJECTED 2, REDUCTION_COMPLETED 2, DISCREPANCY_REPORTED 2, DISCREPANCY_ROUTED 2, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    4.782  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.12)
  t=   45.980  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD_LOCK@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_v
  t=   45.981  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD_LOCK@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955
  t=   45.981  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD_LOCK@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:ADD_LOCK@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   45.981  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDN@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_versi
  t=   45.981  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDN@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+380
  t=   45.981  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDN@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:ANDN@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   45.981  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDNPS@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=   45.981  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDNPS@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+3
  t=   45.981  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDNPS@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:ANDNPS@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   45.985  case_runs -> CANDIDATE_REJECTED (candidate_id=case-runner-rust:FCOS@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, discrepancy_id=case-runner-rust-1793193986446468000-FCOS, reason=2 diff
  t=   45.985  case_runs -> REDUCTION_COMPLETED (comparison_id=cmp-case-runner-rust:FCOS@4698e7819ca9, minimized_program_sha256=sha256:88781aa2b72dfb74d1d27b7e8be6135f9ee5ecf7..., preserved_mismatch=True, reduction_id=case-runner-rust-
  t=   45.985  case_runs -> DISCREPANCY_REPORTED (comparison_id=cmp-case-runner-rust:FCOS@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, destination_profile_id=w2m the Rust translator (crates/translato
  t=   45.985  case_runs -> DISCREPANCY_ROUTED (discrepancy_id=case-runner-rust-1793193986446468000-FCOS, grounds=the Intel CPU under Windows is the reference; t..., reference_conforms=True, repair_path=implementation_repair)
  t=   45.985  case_runs -> CANDIDATE_REJECTED (candidate_id=case-runner-rust:FSIN@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, discrepancy_id=case-runner-rust-1793193986446468000-FSIN, reason=2 diff
  t=   45.985  case_runs -> REDUCTION_COMPLETED (comparison_id=cmp-case-runner-rust:FSIN@4698e7819ca9, minimized_program_sha256=sha256:04fdb6d6870cc53479d0a9c26b94fb42368aa9d7..., preserved_mismatch=True, reduction_id=case-runner-rust-
  t=   45.986  case_runs -> DISCREPANCY_REPORTED (comparison_id=cmp-case-runner-rust:FSIN@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, destination_profile_id=w2m the Rust translator (crates/translato
  t=   45.986  case_runs -> DISCREPANCY_ROUTED (discrepancy_id=case-runner-rust-1793193986446468000-FSIN, grounds=the Intel CPU under Windows is the reference; t..., reference_conforms=True, repair_path=implementation_repair)
  t=   46.000  case_runs -> SESSION_COMPLETED (accepted_count=233, discrepancy_count=2, elapsed_seconds=45.758209709, outcome=completed_with_discrepancies)
  ... CANDIDATE_GENERATED 232 more
  ... COMPARISON_COMPLETED 232 more
  ... CANDIDATE_ACCEPTED 230 more
case-runner-rust-1793193981557203000: SESSION_INIT 1, CANDIDATE_GENERATED 177, COMPARISON_COMPLETED 177, CANDIDATE_ACCEPTED 177, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    4.783  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.12)
  t=   25.715  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADC@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   25.716  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADC@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   25.716  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADC@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:ADC@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   25.716  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   25.716  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   25.716  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:ADD@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   25.716  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   25.716  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+38
  t=   25.716  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:ADDPD@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   25.731  case_runs -> SESSION_COMPLETED (accepted_count=177, discrepancy_count=0, elapsed_seconds=25.487896208, outcome=completed)
  ... CANDIDATE_GENERATED 174 more
  ... COMPARISON_COMPLETED 174 more
  ... CANDIDATE_ACCEPTED 174 more
case-runner-rust-1793193981584542000: SESSION_INIT 1, CANDIDATE_GENERATED 107, COMPARISON_COMPLETED 107, CANDIDATE_ACCEPTED 107, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    4.782  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz; ..., purpose=conformance, vocab_version=0.12)
  t=   16.274  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   16.275  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, rel
  t=   16.275  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:ADD@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   16.275  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   16.275  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   16.275  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:ADDPD@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   16.275  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPS@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   16.275  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPS@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   16.275  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPS@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:ADDPS@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   16.284  case_runs -> SESSION_COMPLETED (accepted_count=107, discrepancy_count=0, elapsed_seconds=16.046672959, outcome=completed)
  ... CANDIDATE_GENERATED 104 more
  ... COMPARISON_COMPLETED 104 more
  ... CANDIDATE_ACCEPTED 104 more
case-runner-rust-1793193981614746000: SESSION_INIT 1, CANDIDATE_GENERATED 26, COMPARISON_COMPLETED 26, CANDIDATE_ACCEPTED 26, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    4.755  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) 6973P-C; gha-windows-2025-x64-..., purpose=conformance, vocab_version=0.12)
  t=    6.881  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:MOVHPS@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=    6.882  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:MOVHPS@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exa
  t=    6.882  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:MOVHPS@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:MOVHPS@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    6.882  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PACKUSWB@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_v
  t=    6.882  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PACKUSWB@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=e
  t=    6.882  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PACKUSWB@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:PACKUSWB@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    6.882  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PADDQ@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=    6.882  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PADDQ@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exac
  t=    6.882  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PADDQ@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:PADDQ@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    6.884  case_runs -> SESSION_COMPLETED (accepted_count=26, discrepancy_count=0, elapsed_seconds=6.676672459, outcome=completed)
  ... CANDIDATE_GENERATED 23 more
  ... COMPARISON_COMPLETED 23 more
  ... CANDIDATE_ACCEPTED 23 more
```

### signal_trace_tags

- `CANDIDATE_ACCEPTED`
- `CANDIDATE_GENERATED`
- `CANDIDATE_REJECTED`
- `COMPARISON_COMPLETED`
- `DISCREPANCY_REPORTED`
- `DISCREPANCY_ROUTED`
- `REDUCTION_COMPLETED`
- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - On four `case-runner-rust` records: SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED,
>   SESSION_COMPLETED, now judging the legacy SSE classes' model cases; FCOS's and FSIN's CANDIDATE_REJECTED,
>   REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED as Sprint 175 left them.

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 543 times
- `CANDIDATE_GENERATED`: fired 545 times
- `CANDIDATE_REJECTED`: fired 2 times
- `COMPARISON_COMPLETED`: fired 545 times
- `DISCREPANCY_REPORTED`: fired 2 times
- `DISCREPANCY_ROUTED`: fired 2 times
- `REDUCTION_COMPLETED`: fired 2 times
- `SESSION_COMPLETED`: fired 4 times
- `SESSION_INIT`: fired 4 times

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 178: its Rubber Duck Pass and close.
