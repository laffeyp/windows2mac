# SIGNAL_REPORT — Sprint 166 — An unknown condition refused, not lowered as another

Made by `signal_report 166` from 4 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 166.

## 1. Observed

### scope_confirmation

> The code review of 2026-10-28, finding 7's two silent fallbacks (Decision 2026-10-28): `crates/translator/src/lower.rs`
> `inverse` (582-598) gives "gt" for any condition it does not name, and `cond_flags` (610-621) gives `le`'s flags for
> any x86 condition it does not name. The frozen Python raises on both, since `INVERSE` and `COND_FLAGS` are dicts
> (`src/w2m/vm_backend.py` 830-831, 852-853): an unknown key is a KeyError, which refuses the block. The Rust lowers the
> block with a wrong branch or a wrong flags liveness, on one vCPU.

### work_performed

- ad0a3b431 Sprint 166: an unknown condition refused, not lowered as another
- c211af09b Sprint 166: its card, before its code (an unknown condition refused)

### signal_trace

```
case-runner-rust-1793173959000000000: SESSION_INIT 1, CANDIDATE_GENERATED 205, COMPARISON_COMPLETED 205, CANDIDATE_ACCEPTED 205, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.799  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.11)
  t=   40.663  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD_LOCK@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_v
  t=   40.663  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD_LOCK@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955
  t=   40.663  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD_LOCK@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:ADD_LOCK@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   40.663  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDN@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_versi
  t=   40.663  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDN@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+380
  t=   40.663  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDN@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:ANDN@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   40.663  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDNPS@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=   40.663  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDNPS@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+3
  t=   40.663  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDNPS@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:ANDNPS@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   40.680  case_runs -> SESSION_COMPLETED (accepted_count=205, discrepancy_count=0, elapsed_seconds=40.280426167, outcome=completed)
  ... CANDIDATE_GENERATED 202 more
  ... COMPARISON_COMPLETED 202 more
  ... CANDIDATE_ACCEPTED 202 more
case-runner-rust-1793174000000000000: SESSION_INIT 1, CANDIDATE_GENERATED 174, COMPARISON_COMPLETED 174, CANDIDATE_ACCEPTED 174, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.582  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.11)
  t=   21.983  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADC@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   21.984  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADC@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   21.984  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADC@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:ADC@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   21.984  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   21.984  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   21.984  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:ADD@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   21.984  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   21.984  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+38
  t=   21.984  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:ADDPD@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   21.998  case_runs -> SESSION_COMPLETED (accepted_count=174, discrepancy_count=0, elapsed_seconds=21.773339791, outcome=completed)
  ... CANDIDATE_GENERATED 171 more
  ... COMPARISON_COMPLETED 171 more
  ... CANDIDATE_ACCEPTED 171 more
case-runner-rust-1793174024000000000: SESSION_INIT 1, CANDIDATE_GENERATED 107, COMPARISON_COMPLETED 107, CANDIDATE_ACCEPTED 107, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.615  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz; ..., purpose=conformance, vocab_version=0.11)
  t=   12.766  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   12.766  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, rel
  t=   12.766  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:ADD@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.766  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   12.766  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   12.766  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:ADDPD@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.766  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPS@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   12.766  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPS@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   12.767  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPS@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:ADDPS@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.775  case_runs -> SESSION_COMPLETED (accepted_count=107, discrepancy_count=0, elapsed_seconds=12.541184166, outcome=completed)
  ... CANDIDATE_GENERATED 104 more
  ... COMPARISON_COMPLETED 104 more
  ... CANDIDATE_ACCEPTED 104 more
case-runner-rust-1793174038000000000: SESSION_INIT 1, CANDIDATE_GENERATED 26, COMPARISON_COMPLETED 26, CANDIDATE_ACCEPTED 26, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.626  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) 6973P-C; gha-windows-2025-x64-..., purpose=conformance, vocab_version=0.11)
  t=    5.309  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:MOVHPS@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=    5.309  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:MOVHPS@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exa
  t=    5.309  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:MOVHPS@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:MOVHPS@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.309  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PACKUSWB@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_v
  t=    5.310  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PACKUSWB@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=e
  t=    5.310  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PACKUSWB@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:PACKUSWB@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.310  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PADDQ@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=    5.310  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PADDQ@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exac
  t=    5.310  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PADDQ@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:PADDQ@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.312  case_runs -> SESSION_COMPLETED (accepted_count=26, discrepancy_count=0, elapsed_seconds=5.084071208, outcome=completed)
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

> - On four `store/runs/case-runner-rust-<ns>` records (the four reference profiles): SESSION_INIT, CANDIDATE_GENERATED,
>   COMPARISON_COMPLETED, CANDIDATE_ACCEPTED, SESSION_COMPLETED, as Sprints 159-161's.

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 512 times
- `CANDIDATE_GENERATED`: fired 512 times
- `COMPARISON_COMPLETED`: fired 512 times
- `SESSION_COMPLETED`: fired 4 times
- `SESSION_INIT`: fired 4 times

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 166: its Rubber Duck Pass and close.
