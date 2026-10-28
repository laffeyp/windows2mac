# SIGNAL_REPORT — Sprint 173 — The model's core and its integer classes, in Rust

Made by `signal_report 173` from 4 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 173.

## 1. Observed

### scope_confirmation

> Step 4's first component, the model (S4; Decision 2026-10-27; census row 4, `docs/step-4-census-2026-10-28.md`). The
> model is the frozen Python's interpreter of lifted operations: `src/w2m/runtime.py` `_exec` and `_exec_ops` (995-1161),
> with `integer_model.py` (157 lines) for the `x86` steps, `x86fp.py` for SSE, `x87.py` and `_mmx` for the rest. The VM
> reaches it through hvc 10 (slow SSE) and hvc 11 (slow step); the Rust run loop ends at both (`run.rs` 553-554).

### work_performed

- 3dde2c3ed Sprint 173: the model's core and its integer classes, in Rust
- 75e891c44 Sprint 173: its card, before its code (the model's core and its integer classes)

### signal_trace

```
case-runner-rust-1793182653000000000: SESSION_INIT 1, CANDIDATE_GENERATED 223, COMPARISON_COMPLETED 223, CANDIDATE_ACCEPTED 223, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    4.128  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.12)
  t=   40.723  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD_LOCK@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_v
  t=   40.723  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD_LOCK@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955
  t=   40.723  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD_LOCK@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADD_LOCK@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   40.723  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDN@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_versi
  t=   40.723  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDN@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+380
  t=   40.723  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDN@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ANDN@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   40.723  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDNPS@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=   40.723  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDNPS@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+3
  t=   40.723  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDNPS@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ANDNPS@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   40.741  case_runs -> SESSION_COMPLETED (accepted_count=223, discrepancy_count=0, elapsed_seconds=40.359969958, outcome=completed)
  ... CANDIDATE_GENERATED 220 more
  ... COMPARISON_COMPLETED 220 more
  ... CANDIDATE_ACCEPTED 220 more
case-runner-rust-1793182695000000000: SESSION_INIT 1, CANDIDATE_GENERATED 174, COMPARISON_COMPLETED 174, CANDIDATE_ACCEPTED 174, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.348  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.12)
  t=   21.619  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADC@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   21.620  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADC@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   21.620  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADC@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADC@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   21.620  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   21.620  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   21.620  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADD@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   21.620  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   21.620  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+38
  t=   21.620  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADDPD@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   21.634  case_runs -> SESSION_COMPLETED (accepted_count=174, discrepancy_count=0, elapsed_seconds=21.606491, outcome=completed)
  ... CANDIDATE_GENERATED 171 more
  ... COMPARISON_COMPLETED 171 more
  ... CANDIDATE_ACCEPTED 171 more
case-runner-rust-1793182718000000000: SESSION_INIT 1, CANDIDATE_GENERATED 107, COMPARISON_COMPLETED 107, CANDIDATE_ACCEPTED 107, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.361  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz; ..., purpose=conformance, vocab_version=0.12)
  t=   12.405  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   12.405  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, rel
  t=   12.405  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADD@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.405  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   12.405  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   12.405  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADDPD@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.405  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPS@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   12.405  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPS@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   12.406  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPS@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADDPS@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.415  case_runs -> SESSION_COMPLETED (accepted_count=107, discrepancy_count=0, elapsed_seconds=12.38906675, outcome=completed)
  ... CANDIDATE_GENERATED 104 more
  ... COMPARISON_COMPLETED 104 more
  ... CANDIDATE_ACCEPTED 104 more
case-runner-rust-1793182732000000000: SESSION_INIT 1, CANDIDATE_GENERATED 26, COMPARISON_COMPLETED 26, CANDIDATE_ACCEPTED 26, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.385  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) 6973P-C; gha-windows-2025-x64-..., purpose=conformance, vocab_version=0.12)
  t=    5.046  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:MOVHPS@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=    5.046  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:MOVHPS@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exa
  t=    5.046  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:MOVHPS@3e42e716918b, comparison_ids=['cmp-case-runner-rust:MOVHPS@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.046  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PACKUSWB@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_v
  t=    5.046  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PACKUSWB@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=e
  t=    5.046  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PACKUSWB@3e42e716918b, comparison_ids=['cmp-case-runner-rust:PACKUSWB@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.046  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PADDQ@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=    5.046  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PADDQ@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exac
  t=    5.046  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PADDQ@3e42e716918b, comparison_ids=['cmp-case-runner-rust:PADDQ@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.048  case_runs -> SESSION_COMPLETED (accepted_count=26, discrepancy_count=0, elapsed_seconds=5.031591458, outcome=completed)
  ... CANDIDATE_GENERATED 23 more
  ... COMPARISON_COMPLETED 23 more
  ... CANDIDATE_ACCEPTED 23 more
```

### signal_trace_tags

- `CANDIDATE_ACCEPTED`
- `CANDIDATE_GENERATED`
- `COMPARISON_COMPLETED`
- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - On four `case-runner-rust` records: SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED,
>   SESSION_COMPLETED, as before, now judging the integer classes' model cases.

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 530 times
- `CANDIDATE_GENERATED`: fired 530 times
- `COMPARISON_COMPLETED`: fired 530 times
- `SESSION_COMPLETED`: fired 4 times
- `SESSION_INIT`: fired 4 times

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 173: its Rubber Duck Pass and close.
