# SIGNAL_REPORT — Sprint 160 — The SSE float core: link 5's fourth group, lifted and lowered in Rust

Made by `signal_report 160` from 4 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 160.

## 1. Observed

### scope_confirmation

> Link 5's fourth group (`docs/design-host-compile-path-2026-10-24.md`): the 52 classes whose rule is
> `src/w2m/rules/_sse.py`'s `rule` (one module each, `store/rules/registry.tsv`), all lowered in `_sse.lower`'s own body,
> none by `_lower_int`. After Sprint 159 the stencil record's calls the Rust rules do not lift are 89,584; measured by the
> C words judge's new census (decoded class of each call no Rust rule lifts), this group holds the largest share:
> MOVAPS 21,645, MOVSS 15,536, MOVUPS 8,498, MOVSD_XMM 4,784, MULSS 3,287, MULPS 3,134, CVTSS2SD 2,719, CVTSD2SS 2,389,
> ADDPS 2,139, ADDSS 1,872, XORPS 1,776, COMISS 1,026.

### work_performed

- e9869ca0c Sprint 160: the SSE float core in Rust (link 5, group 4)
- 6e80885f1 Sprint 160: its card, before its code (the SSE float core)

### signal_trace

```
case-runner-rust-1793163955000000000: SESSION_INIT 1, CANDIDATE_GENERATED 205, COMPARISON_COMPLETED 205, CANDIDATE_ACCEPTED 205, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    7.858  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.10)
  t=   44.643  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD_LOCK@c87cddd3f94c, candidate_sha256=sha256:c87cddd3f94c93d2fd8c5183240cb7298453ed2d..., contract_id=w2m.guest_state.fault_context, contract_v
  t=   44.643  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD_LOCK@c87cddd3f94c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955
  t=   44.643  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD_LOCK@c87cddd3f94c, comparison_ids=['cmp-case-runner-rust:ADD_LOCK@c87cddd3f94c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   44.643  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDN@c87cddd3f94c, candidate_sha256=sha256:c87cddd3f94c93d2fd8c5183240cb7298453ed2d..., contract_id=w2m.guest_state.fault_context, contract_versi
  t=   44.643  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDN@c87cddd3f94c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+380
  t=   44.643  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDN@c87cddd3f94c, comparison_ids=['cmp-case-runner-rust:ANDN@c87cddd3f94c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   44.643  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDNPS@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=   44.643  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDNPS@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+3
  t=   44.643  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDNPS@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:ANDNPS@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   44.660  case_runs -> SESSION_COMPLETED (accepted_count=205, discrepancy_count=0, elapsed_seconds=44.002166042, outcome=completed)
  ... CANDIDATE_GENERATED 202 more
  ... COMPARISON_COMPLETED 202 more
  ... CANDIDATE_ACCEPTED 202 more
case-runner-rust-1793164001000000000: SESSION_INIT 1, CANDIDATE_GENERATED 174, COMPARISON_COMPLETED 174, CANDIDATE_ACCEPTED 174, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.627  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.10)
  t=   22.067  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADC@c87cddd3f94c, candidate_sha256=sha256:c87cddd3f94c93d2fd8c5183240cb7298453ed2d..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   22.067  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADC@c87cddd3f94c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   22.067  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADC@c87cddd3f94c, comparison_ids=['cmp-case-runner-rust:ADC@c87cddd3f94c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   22.068  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@c87cddd3f94c, candidate_sha256=sha256:c87cddd3f94c93d2fd8c5183240cb7298453ed2d..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   22.068  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@c87cddd3f94c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   22.068  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@c87cddd3f94c, comparison_ids=['cmp-case-runner-rust:ADD@c87cddd3f94c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   22.068  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@c87cddd3f94c, candidate_sha256=sha256:c87cddd3f94c93d2fd8c5183240cb7298453ed2d..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   22.068  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@c87cddd3f94c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+38
  t=   22.068  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@c87cddd3f94c, comparison_ids=['cmp-case-runner-rust:ADDPD@c87cddd3f94c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   22.082  case_runs -> SESSION_COMPLETED (accepted_count=174, discrepancy_count=0, elapsed_seconds=21.8334855, outcome=completed)
  ... CANDIDATE_GENERATED 171 more
  ... COMPARISON_COMPLETED 171 more
  ... CANDIDATE_ACCEPTED 171 more
case-runner-rust-1793164024000000000: SESSION_INIT 1, CANDIDATE_GENERATED 107, COMPARISON_COMPLETED 107, CANDIDATE_ACCEPTED 107, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.679  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz; ..., purpose=conformance, vocab_version=0.10)
  t=   12.812  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@c87cddd3f94c, candidate_sha256=sha256:c87cddd3f94c93d2fd8c5183240cb7298453ed2d..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   12.813  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@c87cddd3f94c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, rel
  t=   12.813  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@c87cddd3f94c, comparison_ids=['cmp-case-runner-rust:ADD@c87cddd3f94c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.813  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@c87cddd3f94c, candidate_sha256=sha256:c87cddd3f94c93d2fd8c5183240cb7298453ed2d..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   12.813  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@c87cddd3f94c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   12.813  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@c87cddd3f94c, comparison_ids=['cmp-case-runner-rust:ADDPD@c87cddd3f94c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.813  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPS@c87cddd3f94c, candidate_sha256=sha256:c87cddd3f94c93d2fd8c5183240cb7298453ed2d..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   12.813  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPS@c87cddd3f94c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   12.813  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPS@c87cddd3f94c, comparison_ids=['cmp-case-runner-rust:ADDPS@c87cddd3f94c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.822  case_runs -> SESSION_COMPLETED (accepted_count=107, discrepancy_count=0, elapsed_seconds=12.569452667, outcome=completed)
  ... CANDIDATE_GENERATED 104 more
  ... COMPARISON_COMPLETED 104 more
  ... CANDIDATE_ACCEPTED 104 more
case-runner-rust-1793164038000000000: SESSION_INIT 1, CANDIDATE_GENERATED 26, COMPARISON_COMPLETED 26, CANDIDATE_ACCEPTED 26, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.634  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) 6973P-C; gha-windows-2025-x64-..., purpose=conformance, vocab_version=0.10)
  t=    5.305  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:MOVHPS@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=    5.306  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:MOVHPS@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exa
  t=    5.306  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:MOVHPS@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:MOVHPS@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.306  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PACKUSWB@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_v
  t=    5.306  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PACKUSWB@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=e
  t=    5.306  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PACKUSWB@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:PACKUSWB@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.306  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PADDQ@a20dc6efe84b, candidate_sha256=sha256:a20dc6efe84b176d3b4215837013e506361315e5..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=    5.306  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PADDQ@a20dc6efe84b, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exac
  t=    5.306  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PADDQ@a20dc6efe84b, comparison_ids=['cmp-case-runner-rust:PADDQ@a20dc6efe84b'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.308  case_runs -> SESSION_COMPLETED (accepted_count=26, discrepancy_count=0, elapsed_seconds=5.058273708, outcome=completed)
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

See the BLACKBOARD tail entry for Sprint 160: its Rubber Duck Pass and close.
