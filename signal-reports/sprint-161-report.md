# SIGNAL_REPORT — Sprint 161 — The SSE2 integer block: link 5's fifth group, lifted and lowered in Rust

Made by `signal_report 161` from 4 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 161.

## 1. Observed

### scope_confirmation

> Link 5's fifth group (`docs/design-host-compile-path-2026-10-24.md`): the 77 classes whose rule is
> `src/w2m/rules/_sse.py`'s `int_rule` (or `memory_only_rule`, which falls to it) and whose lowering is `_lower_int`
> (`SSE2_INT_LOWERED`, `_sse.py` 100-126), counted from `store/rules/registry.tsv` and the sets as written. By the C words
> judge's census after Sprint 160, SHUFPS 2,841, MOVDQA 2,528, PSHUFD 1,161, UNPCKLPS 574, MOVDQU 447, PXOR 372, ORPS
> 341, ANDNPS 311, PSUBUSB 226 lead.

### work_performed

- 54a20cdba Sprint 161: the SSE2 integer block in Rust (link 5, group 5)
- afa0cca28 Sprint 161: its card, before its code (the SSE2 integer block)

### signal_trace

```
case-runner-rust-1793165537000000000: SESSION_INIT 1, CANDIDATE_GENERATED 205, COMPARISON_COMPLETED 205, CANDIDATE_ACCEPTED 205, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    4.023  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.10)
  t=   40.834  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD_LOCK@da42a12d846d, candidate_sha256=sha256:da42a12d846dbae93b304c16752b22d45e6fe22b..., contract_id=w2m.guest_state.fault_context, contract_v
  t=   40.835  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD_LOCK@da42a12d846d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955
  t=   40.835  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD_LOCK@da42a12d846d, comparison_ids=['cmp-case-runner-rust:ADD_LOCK@da42a12d846d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   40.835  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDN@da42a12d846d, candidate_sha256=sha256:da42a12d846dbae93b304c16752b22d45e6fe22b..., contract_id=w2m.guest_state.fault_context, contract_versi
  t=   40.835  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDN@da42a12d846d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+380
  t=   40.835  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDN@da42a12d846d, comparison_ids=['cmp-case-runner-rust:ANDN@da42a12d846d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   40.835  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDNPS@da42a12d846d, candidate_sha256=sha256:da42a12d846dbae93b304c16752b22d45e6fe22b..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=   40.835  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDNPS@da42a12d846d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+3
  t=   40.835  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDNPS@da42a12d846d, comparison_ids=['cmp-case-runner-rust:ANDNPS@da42a12d846d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   40.852  case_runs -> SESSION_COMPLETED (accepted_count=205, discrepancy_count=0, elapsed_seconds=40.367918416, outcome=completed)
  ... CANDIDATE_GENERATED 202 more
  ... COMPARISON_COMPLETED 202 more
  ... CANDIDATE_ACCEPTED 202 more
case-runner-rust-1793165579000000000: SESSION_INIT 1, CANDIDATE_GENERATED 174, COMPARISON_COMPLETED 174, CANDIDATE_ACCEPTED 174, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    4.100  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.10)
  t=   22.596  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADC@da42a12d846d, candidate_sha256=sha256:da42a12d846dbae93b304c16752b22d45e6fe22b..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   22.597  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADC@da42a12d846d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   22.597  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADC@da42a12d846d, comparison_ids=['cmp-case-runner-rust:ADC@da42a12d846d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   22.597  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@da42a12d846d, candidate_sha256=sha256:da42a12d846dbae93b304c16752b22d45e6fe22b..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   22.597  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@da42a12d846d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   22.597  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@da42a12d846d, comparison_ids=['cmp-case-runner-rust:ADD@da42a12d846d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   22.597  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@da42a12d846d, candidate_sha256=sha256:da42a12d846dbae93b304c16752b22d45e6fe22b..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   22.597  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@da42a12d846d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+38
  t=   22.597  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@da42a12d846d, comparison_ids=['cmp-case-runner-rust:ADDPD@da42a12d846d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   22.611  case_runs -> SESSION_COMPLETED (accepted_count=174, discrepancy_count=0, elapsed_seconds=22.33941875, outcome=completed)
  ... CANDIDATE_GENERATED 171 more
  ... COMPARISON_COMPLETED 171 more
  ... CANDIDATE_ACCEPTED 171 more
case-runner-rust-1793165603000000000: SESSION_INIT 1, CANDIDATE_GENERATED 107, COMPARISON_COMPLETED 107, CANDIDATE_ACCEPTED 107, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.827  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz; ..., purpose=conformance, vocab_version=0.10)
  t=   12.928  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@da42a12d846d, candidate_sha256=sha256:da42a12d846dbae93b304c16752b22d45e6fe22b..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   12.928  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@da42a12d846d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, rel
  t=   12.928  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@da42a12d846d, comparison_ids=['cmp-case-runner-rust:ADD@da42a12d846d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.928  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@da42a12d846d, candidate_sha256=sha256:da42a12d846dbae93b304c16752b22d45e6fe22b..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   12.928  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@da42a12d846d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   12.928  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@da42a12d846d, comparison_ids=['cmp-case-runner-rust:ADDPD@da42a12d846d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.928  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPS@da42a12d846d, candidate_sha256=sha256:da42a12d846dbae93b304c16752b22d45e6fe22b..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   12.928  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPS@da42a12d846d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   12.928  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPS@da42a12d846d, comparison_ids=['cmp-case-runner-rust:ADDPS@da42a12d846d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.937  case_runs -> SESSION_COMPLETED (accepted_count=107, discrepancy_count=0, elapsed_seconds=12.681917958, outcome=completed)
  ... CANDIDATE_GENERATED 104 more
  ... COMPARISON_COMPLETED 104 more
  ... CANDIDATE_ACCEPTED 104 more
case-runner-rust-1793165617000000000: SESSION_INIT 1, CANDIDATE_GENERATED 26, COMPARISON_COMPLETED 26, CANDIDATE_ACCEPTED 26, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.610  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) 6973P-C; gha-windows-2025-x64-..., purpose=conformance, vocab_version=0.10)
  t=    5.285  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:MOVHPS@da42a12d846d, candidate_sha256=sha256:da42a12d846dbae93b304c16752b22d45e6fe22b..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=    5.286  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:MOVHPS@da42a12d846d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exa
  t=    5.286  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:MOVHPS@da42a12d846d, comparison_ids=['cmp-case-runner-rust:MOVHPS@da42a12d846d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.286  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PACKUSWB@da42a12d846d, candidate_sha256=sha256:da42a12d846dbae93b304c16752b22d45e6fe22b..., contract_id=w2m.guest_state.fault_context, contract_v
  t=    5.286  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PACKUSWB@da42a12d846d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=e
  t=    5.286  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PACKUSWB@da42a12d846d, comparison_ids=['cmp-case-runner-rust:PACKUSWB@da42a12d846d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.286  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PADDQ@da42a12d846d, candidate_sha256=sha256:da42a12d846dbae93b304c16752b22d45e6fe22b..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=    5.286  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PADDQ@da42a12d846d, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exac
  t=    5.286  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PADDQ@da42a12d846d, comparison_ids=['cmp-case-runner-rust:PADDQ@da42a12d846d'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.288  case_runs -> SESSION_COMPLETED (accepted_count=26, discrepancy_count=0, elapsed_seconds=5.036342875, outcome=completed)
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
>   CANDIDATE_GENERATED, COMPARISON_COMPLETED and CANDIDATE_ACCEPTED per class, with this group's classes present in the
>   cases now the Rust translator's candidates; SESSION_COMPLETED completed. The judges' ledger stays empty.

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 512 times
- `CANDIDATE_GENERATED`: fired 512 times
- `COMPARISON_COMPLETED`: fired 512 times
- `SESSION_COMPLETED`: fired 4 times
- `SESSION_INIT`: fired 4 times

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 161: its Rubber Duck Pass and close.
