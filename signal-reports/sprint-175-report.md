# SIGNAL_REPORT — Sprint 175 — The model's x87 slice

Made by `signal_report 175` from 4 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 175.

## 1. Observed

### scope_confirmation

> Step 4's model (census row 4), its third slice. After Sprint 174 the case runner's largest block of uncounted model
> cases is set 065's: 53,349 of its 54,679 cases go to the model and are counted, not judged. They are x87 classes,
> which the Rust translator does not compile, so the case runner's per-class line (`crates/w2m_vm/tests/case_runner.rs`
> 237, which lists only the classes the Rust compiled) never showed them; the SSE classes left (4,170 cases in 40) come
> after.

### work_performed

- 769787c86 Sprint 175: the model's x87 slice
- 5d4a82b85 Sprint 175's card: the model's x87 slice

### signal_trace

```
case-runner-rust-1793187102199422000: SESSION_INIT 1, CANDIDATE_GENERATED 235, COMPARISON_COMPLETED 235, CANDIDATE_ACCEPTED 233, CANDIDATE_REJECTED 2, REDUCTION_COMPLETED 2, DISCREPANCY_REPORTED 2, DISCREPANCY_ROUTED 2, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    4.833  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.12)
  t=   45.591  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD_LOCK@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_v
  t=   45.592  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD_LOCK@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955
  t=   45.592  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD_LOCK@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADD_LOCK@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   45.592  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDN@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_versi
  t=   45.592  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDN@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+380
  t=   45.592  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDN@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ANDN@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   45.592  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDNPS@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=   45.592  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDNPS@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+3
  t=   45.592  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDNPS@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ANDNPS@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   45.596  case_runs -> CANDIDATE_REJECTED (candidate_id=case-runner-rust:FCOS@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, discrepancy_id=case-runner-rust-1793187107144287000-FCOS, reason=2 diff
  t=   45.596  case_runs -> REDUCTION_COMPLETED (comparison_id=cmp-case-runner-rust:FCOS@a20dc6efe84b, minimized_program_sha256=sha256:88781aa2b72dfb74d1d27b7e8be6135f9ee5ecf7..., preserved_mismatch=True, reduction_id=case-runner-rust-
  t=   45.596  case_runs -> DISCREPANCY_REPORTED (comparison_id=cmp-case-runner-rust:FCOS@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, destination_profile_id=w2m the frozen Python lowering's words (b
  t=   45.596  case_runs -> DISCREPANCY_ROUTED (discrepancy_id=case-runner-rust-1793187107144287000-FCOS, grounds=the Intel CPU under Windows is the reference; t..., reference_conforms=True, repair_path=implementation_repair)
  t=   45.597  case_runs -> CANDIDATE_REJECTED (candidate_id=case-runner-rust:FSIN@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, discrepancy_id=case-runner-rust-1793187107144287000-FSIN, reason=2 diff
  t=   45.597  case_runs -> REDUCTION_COMPLETED (comparison_id=cmp-case-runner-rust:FSIN@a20dc6efe84b, minimized_program_sha256=sha256:04fdb6d6870cc53479d0a9c26b94fb42368aa9d7..., preserved_mismatch=True, reduction_id=case-runner-rust-
  t=   45.597  case_runs -> DISCREPANCY_REPORTED (comparison_id=cmp-case-runner-rust:FSIN@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, destination_profile_id=w2m the frozen Python lowering's words (b
  t=   45.597  case_runs -> DISCREPANCY_ROUTED (discrepancy_id=case-runner-rust-1793187107144287000-FSIN, grounds=the Intel CPU under Windows is the reference; t..., reference_conforms=True, repair_path=implementation_repair)
  t=   45.611  case_runs -> SESSION_COMPLETED (accepted_count=233, discrepancy_count=2, elapsed_seconds=45.269116416, outcome=completed_with_discrepancies)
  ... CANDIDATE_GENERATED 232 more
  ... COMPARISON_COMPLETED 232 more
  ... CANDIDATE_ACCEPTED 230 more
case-runner-rust-1793187102230029000: SESSION_INIT 1, CANDIDATE_GENERATED 177, COMPARISON_COMPLETED 177, CANDIDATE_ACCEPTED 177, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    4.833  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.12)
  t=   25.806  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADC@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   25.807  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADC@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   25.807  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADC@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADC@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   25.807  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   25.807  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   25.807  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADD@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   25.807  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   25.807  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+38
  t=   25.807  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADDPD@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   25.821  case_runs -> SESSION_COMPLETED (accepted_count=177, discrepancy_count=0, elapsed_seconds=25.478631875, outcome=completed)
  ... CANDIDATE_GENERATED 174 more
  ... COMPARISON_COMPLETED 174 more
  ... CANDIDATE_ACCEPTED 174 more
case-runner-rust-1793187102258242000: SESSION_INIT 1, CANDIDATE_GENERATED 107, COMPARISON_COMPLETED 107, CANDIDATE_ACCEPTED 107, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    4.809  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz; ..., purpose=conformance, vocab_version=0.12)
  t=   16.297  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   16.297  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, rel
  t=   16.297  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADD@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   16.297  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   16.298  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   16.298  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADDPD@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   16.298  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPS@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   16.298  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPS@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   16.298  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPS@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADDPS@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   16.307  case_runs -> SESSION_COMPLETED (accepted_count=107, discrepancy_count=0, elapsed_seconds=15.993107834, outcome=completed)
  ... CANDIDATE_GENERATED 104 more
  ... COMPARISON_COMPLETED 104 more
  ... CANDIDATE_ACCEPTED 104 more
case-runner-rust-1793187102289222000: SESSION_INIT 1, CANDIDATE_GENERATED 26, COMPARISON_COMPLETED 26, CANDIDATE_ACCEPTED 26, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    4.778  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) 6973P-C; gha-windows-2025-x64-..., purpose=conformance, vocab_version=0.12)
  t=    7.352  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:MOVHPS@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=    7.352  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:MOVHPS@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exa
  t=    7.352  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:MOVHPS@3e42e716918b, comparison_ids=['cmp-case-runner-rust:MOVHPS@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    7.352  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PACKUSWB@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_v
  t=    7.352  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PACKUSWB@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=e
  t=    7.352  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PACKUSWB@3e42e716918b, comparison_ids=['cmp-case-runner-rust:PACKUSWB@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    7.352  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PADDQ@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=    7.352  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PADDQ@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exac
  t=    7.352  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PADDQ@3e42e716918b, comparison_ids=['cmp-case-runner-rust:PADDQ@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    7.354  case_runs -> SESSION_COMPLETED (accepted_count=26, discrepancy_count=0, elapsed_seconds=7.074355375, outcome=completed)
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
>   SESSION_COMPLETED, now judging the x87 classes' model cases.

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 543 times
- `CANDIDATE_GENERATED`: fired 545 times
- `COMPARISON_COMPLETED`: fired 545 times
- `SESSION_COMPLETED`: fired 4 times
- `SESSION_INIT`: fired 4 times
- fired, not named in Emits: `CANDIDATE_REJECTED` (2), `DISCREPANCY_REPORTED` (2), `DISCREPANCY_ROUTED` (2), `REDUCTION_COMPLETED` (2)

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 175: its Rubber Duck Pass and close.
