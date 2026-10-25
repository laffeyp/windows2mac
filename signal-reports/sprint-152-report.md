# SIGNAL_REPORT — Sprint 152 — The judges' records made true: no false trace, a ledger, one profile a session

Made by `signal_report 152` from 8 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 152.

## 1. Observed

### scope_confirmation

> reviewer-2's review of Sprints 149-150 (2026-10-25) found the record form Sprint 150 built false in one event and cut
> off from the ledger; Sprint 151 withheld its record for that reason. This sprint fixes the form and makes both judges'
> records again.

### work_performed

- 9db2f695c Sprint 152: the judges' records made true
- 28c7fca35 Sprint 152: its card, before its code (the judges' records made true; reviewer-2's review of 149-150)

### signal_trace

```
case-runner-1792920132816348000: SESSION_INIT 1, CANDIDATE_GENERATED 205, COMPARISON_COMPLETED 205, CANDIDATE_ACCEPTED 205, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.766  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.10)
  t=   39.093  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner:ADD_LOCK@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   39.094  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner:ADD_LOCK@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+3804
  t=   39.094  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner:ADD_LOCK@a20dc6efe84b, comparison_ids=['cmp-case-runner:ADD_LOCK@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   39.094  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner:ANDN@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_version=0.
  t=   39.094  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner:ANDN@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+38048998
  t=   39.094  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner:ANDN@a20dc6efe84b, comparison_ids=['cmp-case-runner:ANDN@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   39.094  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner:ANDNPS@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_version=
  t=   39.094  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner:ANDNPS@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+380489
  t=   39.094  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner:ANDNPS@a20dc6efe84b, comparison_ids=['cmp-case-runner:ANDNPS@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   39.110  case_runs -> SESSION_COMPLETED (accepted_count=205, discrepancy_count=0, elapsed_seconds=38.506335208, outcome=completed)
  ... CANDIDATE_GENERATED 202 more
  ... COMPARISON_COMPLETED 202 more
  ... CANDIDATE_ACCEPTED 202 more
case-runner-1792920172239058000: SESSION_INIT 1, CANDIDATE_GENERATED 174, COMPARISON_COMPLETED 174, CANDIDATE_ACCEPTED 174, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.450  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.10)
  t=   21.366  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner:ADC@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_version=0.2
  t=   21.366  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner:ADC@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+380397264
  t=   21.366  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner:ADC@a20dc6efe84b, comparison_ids=['cmp-case-runner:ADC@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   21.366  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner:ADD@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_version=0.2
  t=   21.367  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner:ADD@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+380397264
  t=   21.367  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner:ADD@a20dc6efe84b, comparison_ids=['cmp-case-runner:ADD@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   21.367  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner:ADDPD@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_version=0
  t=   21.367  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner:ADDPD@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803972
  t=   21.367  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner:ADDPD@a20dc6efe84b, comparison_ids=['cmp-case-runner:ADDPD@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   21.380  case_runs -> SESSION_COMPLETED (accepted_count=174, discrepancy_count=0, elapsed_seconds=21.137824125, outcome=completed)
  ... CANDIDATE_GENERATED 171 more
  ... COMPARISON_COMPLETED 171 more
  ... CANDIDATE_ACCEPTED 171 more
case-runner-1792920193927446000: SESSION_INIT 1, CANDIDATE_GENERATED 107, COMPARISON_COMPLETED 107, CANDIDATE_ACCEPTED 107, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.537  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz; ..., purpose=conformance, vocab_version=0.10)
  t=   12.401  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner:ADD@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_version=0.2
  t=   12.401  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner:ADD@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, relation
  t=   12.401  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner:ADD@a20dc6efe84b, comparison_ids=['cmp-case-runner:ADD@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.401  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner:ADDPD@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_version=0
  t=   12.401  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner:ADDPD@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, relati
  t=   12.401  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner:ADDPD@a20dc6efe84b, comparison_ids=['cmp-case-runner:ADDPD@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.401  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner:ADDPS@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_version=0
  t=   12.401  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner:ADDPS@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, relati
  t=   12.401  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner:ADDPS@a20dc6efe84b, comparison_ids=['cmp-case-runner:ADDPS@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.410  case_runs -> SESSION_COMPLETED (accepted_count=107, discrepancy_count=0, elapsed_seconds=12.177121833, outcome=completed)
  ... CANDIDATE_GENERATED 104 more
  ... COMPARISON_COMPLETED 104 more
  ... CANDIDATE_ACCEPTED 104 more
case-runner-1792920206640154000: SESSION_INIT 1, CANDIDATE_GENERATED 26, COMPARISON_COMPLETED 26, CANDIDATE_ACCEPTED 26, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.521  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) 6973P-C; gha-windows-2025-x64-..., purpose=conformance, vocab_version=0.10)
  t=    5.148  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner:MOVHPS@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_version=
  t=    5.148  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner:MOVHPS@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exact, r
  t=    5.148  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner:MOVHPS@a20dc6efe84b, comparison_ids=['cmp-case-runner:MOVHPS@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.148  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner:PACKUSWB@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=    5.148  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner:PACKUSWB@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exact,
  t=    5.148  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner:PACKUSWB@a20dc6efe84b, comparison_ids=['cmp-case-runner:PACKUSWB@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.148  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner:PADDQ@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_version=0
  t=    5.148  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner:PADDQ@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exact, ru
  t=    5.148  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner:PADDQ@a20dc6efe84b, comparison_ids=['cmp-case-runner:PADDQ@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.150  case_runs -> SESSION_COMPLETED (accepted_count=26, discrepancy_count=0, elapsed_seconds=4.924146167, outcome=completed)
  ... CANDIDATE_GENERATED 23 more
  ... COMPARISON_COMPLETED 23 more
  ... CANDIDATE_ACCEPTED 23 more
case-runner-rust-1792920212079636000: SESSION_INIT 1, CANDIDATE_GENERATED 205, COMPARISON_COMPLETED 205, CANDIDATE_ACCEPTED 205, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.523  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.10)
  t=   39.523  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD_LOCK@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_v
  t=   39.523  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD_LOCK@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955
  t=   39.524  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD_LOCK@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:ADD_LOCK@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   39.524  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDN@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_versi
  t=   39.524  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDN@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+380
  t=   39.524  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDN@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:ANDN@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   39.524  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDNPS@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=   39.524  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDNPS@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+3
  t=   39.524  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDNPS@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:ANDNPS@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   39.540  case_runs -> SESSION_COMPLETED (accepted_count=205, discrepancy_count=0, elapsed_seconds=39.2920925, outcome=completed)
  ... CANDIDATE_GENERATED 202 more
  ... COMPARISON_COMPLETED 202 more
  ... CANDIDATE_ACCEPTED 202 more
case-runner-rust-1792920251935199000: SESSION_INIT 1, CANDIDATE_GENERATED 174, COMPARISON_COMPLETED 174, CANDIDATE_ACCEPTED 174, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.538  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.10)
  t=   21.653  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADC@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   21.654  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADC@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   21.654  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADC@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:ADC@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   21.654  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@f9277c8b34d0, candidate_sha256=sha256:f9277c8b34d05dcf0b1c83f5fd29bd6749315150..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   21.654  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@f9277c8b34d0, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   21.654  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@f9277c8b34d0, comparison_ids=['cmp-case-runner-rust:ADD@f9277c8b34d0'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   21.654  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   21.654  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+38
  t=   21.654  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:ADDPD@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   21.668  case_runs -> SESSION_COMPLETED (accepted_count=174, discrepancy_count=0, elapsed_seconds=21.421738375, outcome=completed)
  ... CANDIDATE_GENERATED 171 more
  ... COMPARISON_COMPLETED 171 more
  ... CANDIDATE_ACCEPTED 171 more
case-runner-rust-1792920273914508000: SESSION_INIT 1, CANDIDATE_GENERATED 107, COMPARISON_COMPLETED 107, CANDIDATE_ACCEPTED 107, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.516  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz; ..., purpose=conformance, vocab_version=0.10)
  t=   12.461  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@f9277c8b34d0, candidate_sha256=sha256:f9277c8b34d05dcf0b1c83f5fd29bd6749315150..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   12.462  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@f9277c8b34d0, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, rel
  t=   12.462  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@f9277c8b34d0, comparison_ids=['cmp-case-runner-rust:ADD@f9277c8b34d0'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.462  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   12.462  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   12.462  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:ADDPD@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.462  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPS@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   12.462  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPS@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   12.462  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPS@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:ADDPS@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.470  case_runs -> SESSION_COMPLETED (accepted_count=107, discrepancy_count=0, elapsed_seconds=12.22890775, outcome=completed)
  ... CANDIDATE_GENERATED 104 more
  ... COMPARISON_COMPLETED 104 more
  ... CANDIDATE_ACCEPTED 104 more
case-runner-rust-1792920286686585000: SESSION_INIT 1, CANDIDATE_GENERATED 26, COMPARISON_COMPLETED 26, CANDIDATE_ACCEPTED 26, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.513  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) 6973P-C; gha-windows-2025-x64-..., purpose=conformance, vocab_version=0.10)
  t=    5.150  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:MOVHPS@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=    5.151  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:MOVHPS@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exa
  t=    5.151  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:MOVHPS@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:MOVHPS@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.151  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PACKUSWB@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_v
  t=    5.151  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PACKUSWB@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=e
  t=    5.151  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PACKUSWB@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:PACKUSWB@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.151  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PADDQ@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=    5.151  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PADDQ@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exac
  t=    5.151  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PADDQ@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:PADDQ@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.153  case_runs -> SESSION_COMPLETED (accepted_count=26, discrepancy_count=0, elapsed_seconds=4.9208645, outcome=completed)
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

> - Per judge (`case-runner`, `case-runner-rust`) and per reference CPU (three): SESSION_INIT {profile the CPU and
>   profile}, CANDIDATE_GENERATED, COMPARISON_COMPLETED {verdict conforms} and CANDIDATE_ACCEPTED per class judged in
>   that CPU's runs, SESSION_COMPLETED {completed}. No EXECUTION_* or TRACE_* event.
> - The planted CF flip (not committed): CANDIDATE_REJECTED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED; the plain run
>   after it: DISCREPANCY_RESOLVED for each.

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 1,024 times
- `CANDIDATE_GENERATED`: fired 1,024 times
- `CANDIDATE_REJECTED`: NOT FIRED
- `COMPARISON_COMPLETED`: fired 1,024 times
- `DISCREPANCY_REPORTED`: NOT FIRED
- `DISCREPANCY_RESOLVED`: NOT FIRED
- `DISCREPANCY_ROUTED`: NOT FIRED
- `SESSION_COMPLETED`: fired 8 times
- `SESSION_INIT`: fired 8 times

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 152: its Rubber Duck Pass and close.
