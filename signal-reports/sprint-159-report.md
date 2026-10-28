# SIGNAL_REPORT — Sprint 159 — The locked and system forms: link 5's third group, lifted and lowered in Rust

Made by `signal_report 159` from 4 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 159.

## 1. Observed

### scope_confirmation

> Link 5's third group (`docs/design-host-compile-path-2026-10-24.md`), the forms Sprint 153's card left out by name:
> the `_LOCK` forms, CMPXCHG16B, the system ops, and the model's `x86` steps, carried into `crates/translator` with the
> lowering their ops need. With it, every class `src/w2m/rules/_int.py` lifts is lifted in Rust. 54 classes, counted
> from `store/rules/registry.tsv`:

### work_performed

- 47a2a5927 Sprint 159: the locked and system forms in Rust (link 5, group 3)
- 8aecb3c04 Sprint 159: its card, before its code (the locked and system forms)

### signal_trace

```
case-runner-rust-1793160290000000000: SESSION_INIT 1, CANDIDATE_GENERATED 205, COMPARISON_COMPLETED 205, CANDIDATE_ACCEPTED 205, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    8.052  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.10)
  t=   46.788  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD_LOCK@f88bf2bd7400, candidate_sha256=sha256:f88bf2bd7400eeb227d2af19bf10382c036a32cf..., contract_id=w2m.guest_state.fault_context, contract_v
  t=   46.788  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD_LOCK@f88bf2bd7400, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955
  t=   46.788  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD_LOCK@f88bf2bd7400, comparison_ids=['cmp-case-runner-rust:ADD_LOCK@f88bf2bd7400'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   46.788  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDN@f88bf2bd7400, candidate_sha256=sha256:f88bf2bd7400eeb227d2af19bf10382c036a32cf..., contract_id=w2m.guest_state.fault_context, contract_versi
  t=   46.788  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDN@f88bf2bd7400, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+380
  t=   46.788  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDN@f88bf2bd7400, comparison_ids=['cmp-case-runner-rust:ANDN@f88bf2bd7400'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   46.788  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDNPS@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=   46.788  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDNPS@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+3
  t=   46.788  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDNPS@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:ANDNPS@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   46.806  case_runs -> SESSION_COMPLETED (accepted_count=205, discrepancy_count=0, elapsed_seconds=46.4320675, outcome=completed)
  ... CANDIDATE_GENERATED 202 more
  ... COMPARISON_COMPLETED 202 more
  ... CANDIDATE_ACCEPTED 202 more
case-runner-rust-1793160338000000000: SESSION_INIT 1, CANDIDATE_GENERATED 174, COMPARISON_COMPLETED 174, CANDIDATE_ACCEPTED 174, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.774  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.10)
  t=   22.689  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADC@f88bf2bd7400, candidate_sha256=sha256:f88bf2bd7400eeb227d2af19bf10382c036a32cf..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   22.689  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADC@f88bf2bd7400, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   22.689  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADC@f88bf2bd7400, comparison_ids=['cmp-case-runner-rust:ADC@f88bf2bd7400'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   22.689  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@f88bf2bd7400, candidate_sha256=sha256:f88bf2bd7400eeb227d2af19bf10382c036a32cf..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   22.689  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@f88bf2bd7400, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   22.689  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@f88bf2bd7400, comparison_ids=['cmp-case-runner-rust:ADD@f88bf2bd7400'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   22.689  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   22.690  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+38
  t=   22.690  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:ADDPD@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   22.704  case_runs -> SESSION_COMPLETED (accepted_count=174, discrepancy_count=0, elapsed_seconds=22.441638125, outcome=completed)
  ... CANDIDATE_GENERATED 171 more
  ... COMPARISON_COMPLETED 171 more
  ... CANDIDATE_ACCEPTED 171 more
case-runner-rust-1793160362000000000: SESSION_INIT 1, CANDIDATE_GENERATED 107, COMPARISON_COMPLETED 107, CANDIDATE_ACCEPTED 107, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.774  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz; ..., purpose=conformance, vocab_version=0.10)
  t=   13.274  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@f88bf2bd7400, candidate_sha256=sha256:f88bf2bd7400eeb227d2af19bf10382c036a32cf..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   13.274  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@f88bf2bd7400, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, rel
  t=   13.274  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@f88bf2bd7400, comparison_ids=['cmp-case-runner-rust:ADD@f88bf2bd7400'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   13.274  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   13.274  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   13.274  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:ADDPD@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   13.274  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPS@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   13.274  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPS@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   13.274  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPS@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:ADDPS@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   13.283  case_runs -> SESSION_COMPLETED (accepted_count=107, discrepancy_count=0, elapsed_seconds=12.990659208, outcome=completed)
  ... CANDIDATE_GENERATED 104 more
  ... COMPARISON_COMPLETED 104 more
  ... CANDIDATE_ACCEPTED 104 more
case-runner-rust-1793160377000000000: SESSION_INIT 1, CANDIDATE_GENERATED 26, COMPARISON_COMPLETED 26, CANDIDATE_ACCEPTED 26, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.704  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) 6973P-C; gha-windows-2025-x64-..., purpose=conformance, vocab_version=0.10)
  t=    5.419  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:MOVHPS@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=    5.419  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:MOVHPS@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exa
  t=    5.419  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:MOVHPS@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:MOVHPS@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.419  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PACKUSWB@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_v
  t=    5.420  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PACKUSWB@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=e
  t=    5.420  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PACKUSWB@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:PACKUSWB@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.420  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PADDQ@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=    5.420  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PADDQ@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exac
  t=    5.420  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PADDQ@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:PADDQ@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.422  case_runs -> SESSION_COMPLETED (accepted_count=26, discrepancy_count=0, elapsed_seconds=5.190280667, outcome=completed)
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

> - The `case-runner-rust` sessions, four reference profiles (`harness/case-runs-rust.toml`): SESSION_INIT,
>   CANDIDATE_GENERATED, COMPARISON_COMPLETED and CANDIDATE_ACCEPTED per class, with group 3's classes present in the
>   cases now the Rust translator's candidates; SESSION_COMPLETED completed. The judges' ledger stays empty.

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 512 times
- `CANDIDATE_GENERATED`: fired 512 times
- `COMPARISON_COMPLETED`: fired 512 times
- `SESSION_COMPLETED`: fired 4 times
- `SESSION_INIT`: fired 4 times

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 159: its Rubber Duck Pass and close.
