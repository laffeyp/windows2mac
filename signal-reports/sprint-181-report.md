# SIGNAL_REPORT — Sprint 181 — The run loop serves the model

Made by `signal_report 181` from 4 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 181.

## 1. Observed

### scope_confirmation

> Decision "Sequence after Sprint 178" (reviewer-2's Medium 2 of its review of 175-177): `model_step` (`crates/w2m_vm/
> src/cases.rs`) is the only server of hvc 10 and 11, and the run loop stops there (`run.rs` 554, `Stop::Hvc`). Each model
> slice since Sprint 173 has added state it carries in and out (x87 in 175, XMM back in 178). The move makes the case
> runner, with its 86,129 stepped cases, judge the function the run loop runs.

### work_performed

- e20d96612 Sprint 181: the run loop serves the model
- c942d2601 Sprint 181's card: the run loop serves the model

### signal_trace

```
case-runner-rust-1793199191973152000: SESSION_INIT 1, CANDIDATE_GENERATED 236, COMPARISON_COMPLETED 236, CANDIDATE_ACCEPTED 234, CANDIDATE_REJECTED 2, REDUCTION_COMPLETED 2, DISCREPANCY_REPORTED 2, DISCREPANCY_ROUTED 2, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    4.620  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.12)
  t=   45.646  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD_LOCK@15c41dd9661d, candidate_sha256=sha256:15c41dd9661d9b619e872cd39ffee72a367ae342..., contract_id=w2m.guest_state.fault_context, contract_v
  t=   45.646  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD_LOCK@15c41dd9661d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955
  t=   45.647  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD_LOCK@15c41dd9661d, comparison_ids=['cmp-case-runner-rust:ADD_LOCK@15c41dd9661d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   45.647  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDN@15c41dd9661d, candidate_sha256=sha256:15c41dd9661d9b619e872cd39ffee72a367ae342..., contract_id=w2m.guest_state.fault_context, contract_versi
  t=   45.647  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDN@15c41dd9661d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+380
  t=   45.647  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDN@15c41dd9661d, comparison_ids=['cmp-case-runner-rust:ANDN@15c41dd9661d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   45.647  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDNPS@15c41dd9661d, candidate_sha256=sha256:15c41dd9661d9b619e872cd39ffee72a367ae342..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=   45.647  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDNPS@15c41dd9661d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+3
  t=   45.647  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDNPS@15c41dd9661d, comparison_ids=['cmp-case-runner-rust:ANDNPS@15c41dd9661d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   45.662  case_runs -> CANDIDATE_REJECTED (candidate_id=case-runner-rust:FCOS@15c41dd9661d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, discrepancy_id=case-runner-rust-1793199196678244000-FCOS, reason=2 diff
  t=   45.662  case_runs -> REDUCTION_COMPLETED (comparison_id=cmp-case-runner-rust:FCOS@15c41dd9661d, minimized_program_sha256=sha256:88781aa2b72dfb74d1d27b7e8be6135f9ee5ecf7..., preserved_mismatch=True, reduction_id=case-runner-rust-
  t=   45.662  case_runs -> DISCREPANCY_REPORTED (comparison_id=cmp-case-runner-rust:FCOS@15c41dd9661d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, destination_profile_id=w2m the Rust translator (crates/translato
  t=   45.662  case_runs -> DISCREPANCY_ROUTED (discrepancy_id=case-runner-rust-1793199196678244000-FCOS, grounds=the Intel CPU under Windows is the reference; t..., reference_conforms=True, repair_path=implementation_repair)
  t=   45.662  case_runs -> CANDIDATE_REJECTED (candidate_id=case-runner-rust:FSIN@15c41dd9661d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, discrepancy_id=case-runner-rust-1793199196678244000-FSIN, reason=2 diff
  t=   45.663  case_runs -> REDUCTION_COMPLETED (comparison_id=cmp-case-runner-rust:FSIN@15c41dd9661d, minimized_program_sha256=sha256:04fdb6d6870cc53479d0a9c26b94fb42368aa9d7..., preserved_mismatch=True, reduction_id=case-runner-rust-
  t=   45.663  case_runs -> DISCREPANCY_REPORTED (comparison_id=cmp-case-runner-rust:FSIN@15c41dd9661d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, destination_profile_id=w2m the Rust translator (crates/translato
  t=   45.663  case_runs -> DISCREPANCY_ROUTED (discrepancy_id=case-runner-rust-1793199196678244000-FSIN, grounds=the Intel CPU under Windows is the reference; t..., reference_conforms=True, repair_path=implementation_repair)
  t=   45.677  case_runs -> SESSION_COMPLETED (accepted_count=234, discrepancy_count=2, elapsed_seconds=45.43152925, outcome=completed_with_discrepancies)
  ... CANDIDATE_GENERATED 233 more
  ... COMPARISON_COMPLETED 233 more
  ... CANDIDATE_ACCEPTED 231 more
case-runner-rust-1793199191999713000: SESSION_INIT 1, CANDIDATE_GENERATED 177, COMPARISON_COMPLETED 177, CANDIDATE_ACCEPTED 177, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    4.596  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.12)
  t=   26.165  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADC@15c41dd9661d, candidate_sha256=sha256:15c41dd9661d9b619e872cd39ffee72a367ae342..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   26.165  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADC@15c41dd9661d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   26.165  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADC@15c41dd9661d, comparison_ids=['cmp-case-runner-rust:ADC@15c41dd9661d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   26.165  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@15c41dd9661d, candidate_sha256=sha256:15c41dd9661d9b619e872cd39ffee72a367ae342..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   26.165  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@15c41dd9661d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   26.165  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@15c41dd9661d, comparison_ids=['cmp-case-runner-rust:ADD@15c41dd9661d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   26.165  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@15c41dd9661d, candidate_sha256=sha256:15c41dd9661d9b619e872cd39ffee72a367ae342..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   26.165  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@15c41dd9661d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+38
  t=   26.165  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@15c41dd9661d, comparison_ids=['cmp-case-runner-rust:ADDPD@15c41dd9661d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   26.180  case_runs -> SESSION_COMPLETED (accepted_count=177, discrepancy_count=0, elapsed_seconds=25.954508416, outcome=completed)
  ... CANDIDATE_GENERATED 174 more
  ... COMPARISON_COMPLETED 174 more
  ... CANDIDATE_ACCEPTED 174 more
case-runner-rust-1793199192030299000: SESSION_INIT 1, CANDIDATE_GENERATED 107, COMPARISON_COMPLETED 107, CANDIDATE_ACCEPTED 107, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    4.564  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz; ..., purpose=conformance, vocab_version=0.12)
  t=   15.720  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@15c41dd9661d, candidate_sha256=sha256:15c41dd9661d9b619e872cd39ffee72a367ae342..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   15.721  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@15c41dd9661d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, rel
  t=   15.721  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@15c41dd9661d, comparison_ids=['cmp-case-runner-rust:ADD@15c41dd9661d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   15.721  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@15c41dd9661d, candidate_sha256=sha256:15c41dd9661d9b619e872cd39ffee72a367ae342..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   15.721  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@15c41dd9661d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   15.721  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@15c41dd9661d, comparison_ids=['cmp-case-runner-rust:ADDPD@15c41dd9661d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   15.721  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPS@15c41dd9661d, candidate_sha256=sha256:15c41dd9661d9b619e872cd39ffee72a367ae342..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   15.721  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPS@15c41dd9661d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   15.721  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPS@15c41dd9661d, comparison_ids=['cmp-case-runner-rust:ADDPS@15c41dd9661d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   15.730  case_runs -> SESSION_COMPLETED (accepted_count=107, discrepancy_count=0, elapsed_seconds=15.542455, outcome=completed)
  ... CANDIDATE_GENERATED 104 more
  ... COMPARISON_COMPLETED 104 more
  ... CANDIDATE_ACCEPTED 104 more
case-runner-rust-1793199192058947000: SESSION_INIT 1, CANDIDATE_GENERATED 26, COMPARISON_COMPLETED 26, CANDIDATE_ACCEPTED 26, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    4.536  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) 6973P-C; gha-windows-2025-x64-..., purpose=conformance, vocab_version=0.12)
  t=    6.648  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:MOVHPS@15c41dd9661d, candidate_sha256=sha256:15c41dd9661d9b619e872cd39ffee72a367ae342..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=    6.648  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:MOVHPS@15c41dd9661d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exa
  t=    6.649  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:MOVHPS@15c41dd9661d, comparison_ids=['cmp-case-runner-rust:MOVHPS@15c41dd9661d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    6.649  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PACKUSWB@15c41dd9661d, candidate_sha256=sha256:15c41dd9661d9b619e872cd39ffee72a367ae342..., contract_id=w2m.guest_state.fault_context, contract_v
  t=    6.649  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PACKUSWB@15c41dd9661d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=e
  t=    6.649  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PACKUSWB@15c41dd9661d, comparison_ids=['cmp-case-runner-rust:PACKUSWB@15c41dd9661d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    6.649  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PADDQ@15c41dd9661d, candidate_sha256=sha256:15c41dd9661d9b619e872cd39ffee72a367ae342..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=    6.649  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PADDQ@15c41dd9661d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exac
  t=    6.649  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PADDQ@15c41dd9661d, comparison_ids=['cmp-case-runner-rust:PADDQ@15c41dd9661d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    6.653  case_runs -> SESSION_COMPLETED (accepted_count=26, discrepancy_count=0, elapsed_seconds=6.4967123749999995, outcome=completed)
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

> - On four `case-runner-rust` records: the verdicts Sprint 180's records hold.

## 3. Delta

### emits_check

- fired, not named in Emits: `CANDIDATE_ACCEPTED` (544), `CANDIDATE_GENERATED` (546), `CANDIDATE_REJECTED` (2), `COMPARISON_COMPLETED` (546), `DISCREPANCY_REPORTED` (2), `DISCREPANCY_ROUTED` (2), `REDUCTION_COMPLETED` (2), `SESSION_COMPLETED` (4), `SESSION_INIT` (4)
- the card's Emits names no tag, and the sprint's records fired the ones above

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 181: its Rubber Duck Pass and close.
