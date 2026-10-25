# SIGNAL_REPORT — Sprint 153 — The integer rest: link 5's second group, lifted and lowered in Rust

Made by `signal_report 153` from 4 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 153.

## 1. Observed

### scope_confirmation

> Link 5's second group (`docs/design-host-compile-path-2026-10-24.md`): the rest of `src/w2m/rules/_int.py`'s rules
> whose lowering is A64 (not the model), and `rules/bmi.py`, carried into `crates/translator` with the lowering their ops
> need. By the record's calls outside group 1 (Sprint 151's census), INC 8,403, IMUL 4,744, SHR 4,668, CMOVcc 3,976, SHL
> 3,871, SETcc 2,189, SAR 1,483, DEC 1,219, BT 1,201, MOVSX 904, NOT 750 lead.

### work_performed

- df2c20b5a Sprint 153: the integer rest in Rust (link 5, group 2)
- 11d4b5363 Sprint 153: its card, before its code (the integer rest; link 5's second group)

### signal_trace

```
case-runner-rust-1792922090254600000: SESSION_INIT 1, CANDIDATE_GENERATED 205, COMPARISON_COMPLETED 205, CANDIDATE_ACCEPTED 205, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.863  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.10)
  t=   40.149  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD_LOCK@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_v
  t=   40.149  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD_LOCK@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955
  t=   40.149  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD_LOCK@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:ADD_LOCK@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   40.149  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDN@f6403159e7c9, candidate_sha256=sha256:f6403159e7c9453f886da8c65f3374b3bef7cfb5..., contract_id=w2m.guest_state.fault_context, contract_versi
  t=   40.149  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDN@f6403159e7c9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+380
  t=   40.149  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDN@f6403159e7c9, comparison_ids=['cmp-case-runner-rust:ANDN@f6403159e7c9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   40.149  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDNPS@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=   40.149  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDNPS@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+3
  t=   40.149  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDNPS@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:ANDNPS@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   40.166  case_runs -> SESSION_COMPLETED (accepted_count=205, discrepancy_count=0, elapsed_seconds=39.636652125, outcome=completed)
  ... CANDIDATE_GENERATED 202 more
  ... COMPARISON_COMPLETED 202 more
  ... CANDIDATE_ACCEPTED 202 more
case-runner-rust-1792922130742172000: SESSION_INIT 1, CANDIDATE_GENERATED 174, COMPARISON_COMPLETED 174, CANDIDATE_ACCEPTED 174, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.599  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.10)
  t=   21.814  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADC@f6403159e7c9, candidate_sha256=sha256:f6403159e7c9453f886da8c65f3374b3bef7cfb5..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   21.814  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADC@f6403159e7c9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   21.814  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADC@f6403159e7c9, comparison_ids=['cmp-case-runner-rust:ADC@f6403159e7c9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   21.814  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@f6403159e7c9, candidate_sha256=sha256:f6403159e7c9453f886da8c65f3374b3bef7cfb5..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   21.814  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@f6403159e7c9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   21.814  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@f6403159e7c9, comparison_ids=['cmp-case-runner-rust:ADD@f6403159e7c9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   21.814  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   21.814  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+38
  t=   21.814  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:ADDPD@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   21.829  case_runs -> SESSION_COMPLETED (accepted_count=174, discrepancy_count=0, elapsed_seconds=21.587633084, outcome=completed)
  ... CANDIDATE_GENERATED 171 more
  ... COMPARISON_COMPLETED 171 more
  ... CANDIDATE_ACCEPTED 171 more
case-runner-rust-1792922152885554000: SESSION_INIT 1, CANDIDATE_GENERATED 107, COMPARISON_COMPLETED 107, CANDIDATE_ACCEPTED 107, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.553  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz; ..., purpose=conformance, vocab_version=0.10)
  t=   12.556  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@f6403159e7c9, candidate_sha256=sha256:f6403159e7c9453f886da8c65f3374b3bef7cfb5..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   12.556  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@f6403159e7c9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, rel
  t=   12.556  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@f6403159e7c9, comparison_ids=['cmp-case-runner-rust:ADD@f6403159e7c9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.556  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   12.556  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   12.556  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:ADDPD@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.556  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPS@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   12.556  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPS@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   12.556  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPS@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:ADDPS@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.565  case_runs -> SESSION_COMPLETED (accepted_count=107, discrepancy_count=0, elapsed_seconds=12.318188375, outcome=completed)
  ... CANDIDATE_GENERATED 104 more
  ... COMPARISON_COMPLETED 104 more
  ... CANDIDATE_ACCEPTED 104 more
case-runner-rust-1792922165755985000: SESSION_INIT 1, CANDIDATE_GENERATED 26, COMPARISON_COMPLETED 26, CANDIDATE_ACCEPTED 26, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.557  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) 6973P-C; gha-windows-2025-x64-..., purpose=conformance, vocab_version=0.10)
  t=    5.203  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:MOVHPS@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=    5.203  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:MOVHPS@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exa
  t=    5.203  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:MOVHPS@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:MOVHPS@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.203  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PACKUSWB@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_v
  t=    5.203  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PACKUSWB@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=e
  t=    5.203  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PACKUSWB@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:PACKUSWB@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.203  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PADDQ@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=    5.203  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PADDQ@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exac
  t=    5.203  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PADDQ@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:PADDQ@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.206  case_runs -> SESSION_COMPLETED (accepted_count=26, discrepancy_count=0, elapsed_seconds=4.971360916, outcome=completed)
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

> - The `case-runner-rust` sessions, four reference profiles (`harness/case-runs-rust.toml`): CANDIDATE_GENERATED,
>   COMPARISON_COMPLETED and CANDIDATE_ACCEPTED per class, with group 2's classes now the Rust translator's candidates;
>   SESSION_COMPLETED completed. The judges' ledger stays empty.

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 512 times
- `CANDIDATE_GENERATED`: fired 512 times
- `COMPARISON_COMPLETED`: fired 512 times
- `SESSION_COMPLETED`: fired 4 times
- fired, not named in Emits: `SESSION_INIT` (4)

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 153: its Rubber Duck Pass and close.
