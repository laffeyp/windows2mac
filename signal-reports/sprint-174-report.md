# SIGNAL_REPORT — Sprint 174 — The model's MMX slice

Made by `signal_report 174` from 4 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 174.

## 1. Observed

### scope_confirmation

> Step 4's model (census row 4), its second slice. After Sprint 173 the case runner counts 21,556 model cases in 61
> classes it does not judge; about 17,000 are MMX-register forms (MOVQ 2,600, PSRAW 1,416, PSRAD 1,362, PSRLQ 1,159, MOVD
> 1,039, the PUNPCK and PADD/PSUB/PMUL classes, PXOR 650, PACKUSWB 628, PSADBW 602, PMADDUBSW 513), the rest scalar SSE
> with exceptions unmasked (112 to 180 a class).

### work_performed

- bc4d84a35 Sprint 174: the model's MMX slice
- 79321b4f3 Sprint 174: its card, before its code (the model's MMX slice)

### signal_trace

```
case-runner-rust-1793184966000000000: SESSION_INIT 1, CANDIDATE_GENERATED 223, COMPARISON_COMPLETED 223, CANDIDATE_ACCEPTED 223, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.740  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.12)
  t=   40.194  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD_LOCK@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_v
  t=   40.195  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD_LOCK@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955
  t=   40.195  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD_LOCK@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADD_LOCK@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   40.195  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDN@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_versi
  t=   40.195  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDN@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+380
  t=   40.195  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDN@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ANDN@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   40.195  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDNPS@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=   40.195  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDNPS@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+3
  t=   40.195  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDNPS@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ANDNPS@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   40.214  case_runs -> SESSION_COMPLETED (accepted_count=223, discrepancy_count=0, elapsed_seconds=39.804552083, outcome=completed)
  ... CANDIDATE_GENERATED 220 more
  ... COMPARISON_COMPLETED 220 more
  ... CANDIDATE_ACCEPTED 220 more
case-runner-rust-1793185007000000000: SESSION_INIT 1, CANDIDATE_GENERATED 175, COMPARISON_COMPLETED 175, CANDIDATE_ACCEPTED 175, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.351  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.12)
  t=   21.675  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADC@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   21.676  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADC@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   21.676  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADC@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADC@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   21.676  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   21.676  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   21.676  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADD@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   21.676  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   21.676  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+38
  t=   21.676  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADDPD@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   21.690  case_runs -> SESSION_COMPLETED (accepted_count=175, discrepancy_count=0, elapsed_seconds=21.666202625, outcome=completed)
  ... CANDIDATE_GENERATED 172 more
  ... COMPARISON_COMPLETED 172 more
  ... CANDIDATE_ACCEPTED 172 more
case-runner-rust-1793185030000000000: SESSION_INIT 1, CANDIDATE_GENERATED 107, COMPARISON_COMPLETED 107, CANDIDATE_ACCEPTED 107, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.351  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz; ..., purpose=conformance, vocab_version=0.12)
  t=   12.393  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   12.393  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, rel
  t=   12.394  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADD@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.394  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   12.394  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   12.394  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADDPD@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.394  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPS@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   12.394  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPS@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   12.394  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPS@3e42e716918b, comparison_ids=['cmp-case-runner-rust:ADDPS@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.403  case_runs -> SESSION_COMPLETED (accepted_count=107, discrepancy_count=0, elapsed_seconds=12.383047292, outcome=completed)
  ... CANDIDATE_GENERATED 104 more
  ... COMPARISON_COMPLETED 104 more
  ... CANDIDATE_ACCEPTED 104 more
case-runner-rust-1793185043000000000: SESSION_INIT 1, CANDIDATE_GENERATED 26, COMPARISON_COMPLETED 26, CANDIDATE_ACCEPTED 26, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.375  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) 6973P-C; gha-windows-2025-x64-..., purpose=conformance, vocab_version=0.12)
  t=    5.039  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:MOVHPS@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=    5.039  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:MOVHPS@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exa
  t=    5.039  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:MOVHPS@3e42e716918b, comparison_ids=['cmp-case-runner-rust:MOVHPS@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.039  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PACKUSWB@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_v
  t=    5.039  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PACKUSWB@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=e
  t=    5.039  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PACKUSWB@3e42e716918b, comparison_ids=['cmp-case-runner-rust:PACKUSWB@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.039  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PADDQ@3e42e716918b, candidate_sha256=sha256:3e42e716918b265e4735d2557215599f547c9ecc..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=    5.039  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PADDQ@3e42e716918b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exac
  t=    5.039  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PADDQ@3e42e716918b, comparison_ids=['cmp-case-runner-rust:PADDQ@3e42e716918b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.041  case_runs -> SESSION_COMPLETED (accepted_count=26, discrepancy_count=0, elapsed_seconds=5.027947417, outcome=completed)
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
>   SESSION_COMPLETED, now judging the MMX classes' model cases.

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 531 times
- `CANDIDATE_GENERATED`: fired 531 times
- `COMPARISON_COMPLETED`: fired 531 times
- `SESSION_COMPLETED`: fired 4 times
- `SESSION_INIT`: fired 4 times

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 174: its Rubber Duck Pass and close.
