# SIGNAL_REPORT — Sprint 167 — A case runner's discrepancy names its case, after a reduction of none

Made by `signal_report 167` from 4 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 167.

## 1. Observed

### scope_confirmation

> reviewer-2's Medium 4 of 2026-10-27 (Decision 2026-10-28's order): the Rust case runner (`crates/w2m_vm/src/bin/
> case_runs.rs` 344-384) reports a class's discrepancy with no REDUCTION_COMPLETED before it, which rule S5 refuses, and
> with `minimized_program_sha256` and `initial_state_sha256` both the empty hash, which rule E1 refuses: the faults the
> Python's class runs had until Sprint 112 (BLACKBOARD Deferred 2026-10-17, Sprint 109). The model's port (step 4) will
> report the first discrepancies the Rust records hold, so the form is set first.

### work_performed

- 6c14b7044 Sprint 167: a case runner's discrepancy names its case, after a reduction of none
- 54db919c9 Sprint 167: its card, before its code (a discrepancy names its case)

### signal_trace

```
case-runner-rust-1793175258000000000: SESSION_INIT 1, CANDIDATE_GENERATED 205, COMPARISON_COMPLETED 205, CANDIDATE_ACCEPTED 205, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.976  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.11)
  t=   40.716  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD_LOCK@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_v
  t=   40.716  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD_LOCK@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955
  t=   40.716  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD_LOCK@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:ADD_LOCK@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   40.716  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDN@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_versi
  t=   40.716  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDN@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+380
  t=   40.716  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDN@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:ANDN@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   40.716  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDNPS@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=   40.716  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDNPS@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+3
  t=   40.716  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDNPS@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:ANDNPS@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   40.737  case_runs -> SESSION_COMPLETED (accepted_count=205, discrepancy_count=0, elapsed_seconds=40.103941125, outcome=completed)
  ... CANDIDATE_GENERATED 202 more
  ... COMPARISON_COMPLETED 202 more
  ... CANDIDATE_ACCEPTED 202 more
case-runner-rust-1793175300000000000: SESSION_INIT 1, CANDIDATE_GENERATED 174, COMPARISON_COMPLETED 174, CANDIDATE_ACCEPTED 174, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.637  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.11)
  t=   22.139  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADC@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   22.139  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADC@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   22.139  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADC@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:ADC@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   22.139  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   22.139  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   22.139  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:ADD@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   22.139  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   22.139  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+38
  t=   22.139  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:ADDPD@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   22.154  case_runs -> SESSION_COMPLETED (accepted_count=174, discrepancy_count=0, elapsed_seconds=21.895844832999998, outcome=completed)
  ... CANDIDATE_GENERATED 171 more
  ... COMPARISON_COMPLETED 171 more
  ... CANDIDATE_ACCEPTED 171 more
case-runner-rust-1793175323000000000: SESSION_INIT 1, CANDIDATE_GENERATED 107, COMPARISON_COMPLETED 107, CANDIDATE_ACCEPTED 107, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.618  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz; ..., purpose=conformance, vocab_version=0.11)
  t=   12.812  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   12.812  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, rel
  t=   12.812  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:ADD@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.812  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   12.812  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   12.813  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:ADDPD@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.813  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPS@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   12.813  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPS@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   12.813  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPS@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:ADDPS@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   12.821  case_runs -> SESSION_COMPLETED (accepted_count=107, discrepancy_count=0, elapsed_seconds=12.58324225, outcome=completed)
  ... CANDIDATE_GENERATED 104 more
  ... COMPARISON_COMPLETED 104 more
  ... CANDIDATE_ACCEPTED 104 more
case-runner-rust-1793175337000000000: SESSION_INIT 1, CANDIDATE_GENERATED 26, COMPARISON_COMPLETED 26, CANDIDATE_ACCEPTED 26, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    3.772  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) 6973P-C; gha-windows-2025-x64-..., purpose=conformance, vocab_version=0.11)
  t=    5.460  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:MOVHPS@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=    5.461  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:MOVHPS@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exa
  t=    5.461  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:MOVHPS@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:MOVHPS@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.461  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PACKUSWB@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_v
  t=    5.461  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PACKUSWB@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=e
  t=    5.461  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PACKUSWB@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:PACKUSWB@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.461  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PADDQ@3f9efa30a55c, candidate_sha256=sha256:3f9efa30a55c45001c3f9aa18b65aa472c583a4b..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=    5.461  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PADDQ@3f9efa30a55c, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exac
  t=    5.461  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PADDQ@3f9efa30a55c, comparison_ids=['cmp-case-runner-rust:PADDQ@3f9efa30a55c'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    5.463  case_runs -> SESSION_COMPLETED (accepted_count=26, discrepancy_count=0, elapsed_seconds=5.228017333, outcome=completed)
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

> - On the four `case-runner-rust` records: SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED,
>   SESSION_COMPLETED, as before (no discrepancy expected).
> - On a planted run's record (a lowering made wrong on purpose): CANDIDATE_REJECTED, REDUCTION_COMPLETED,
>   DISCREPANCY_REPORTED, DISCREPANCY_ROUTED, SESSION_COMPLETED (completed_with_discrepancies).

## 3. Delta

### emits_check

- `CANDIDATE_ACCEPTED`: fired 512 times
- `CANDIDATE_GENERATED`: fired 512 times
- `CANDIDATE_REJECTED`: NOT FIRED
- `COMPARISON_COMPLETED`: fired 512 times
- `DISCREPANCY_REPORTED`: NOT FIRED
- `DISCREPANCY_ROUTED`: NOT FIRED
- `REDUCTION_COMPLETED`: NOT FIRED
- `SESSION_COMPLETED`: fired 4 times
- `SESSION_INIT`: fired 4 times

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 167: its Rubber Duck Pass and close.
