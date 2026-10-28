# SIGNAL_REPORT — Sprint 187 — The judges' bindings

Made by `signal_report 187` from 4 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 187.

## 1. Observed

### scope_confirmation

> reviewer-2's Mediums 2 and 3 and Lows 3 and 5 of its review of Sprints 182-184 (the Decision "Sequence after Sprint
> 185"; Medium 4's gates stage landed in Sprint 185). Each is a judge whose input is not bound to the tree it judges.

### work_performed

- 5fb9f3534 Sprint 187: the judges' bindings
- 2464e0d40 Sprint 187's card: the judges' bindings

### signal_trace

```
case-runner-rust-1793213020000000000: SESSION_INIT 1, CANDIDATE_GENERATED 268, COMPARISON_COMPLETED 268, CANDIDATE_ACCEPTED 266, CANDIDATE_REJECTED 2, REDUCTION_COMPLETED 2, DISCREPANCY_REPORTED 2, DISCREPANCY_ROUTED 2, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    5.174  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.12)
  t=   47.169  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD_LOCK@5871674f486a, candidate_sha256=sha256:5871674f486a95b21a52d65c6141c3f1ab8139b1..., contract_id=w2m.guest_state.fault_context, contract_v
  t=   47.169  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD_LOCK@5871674f486a, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955
  t=   47.169  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD_LOCK@5871674f486a, comparison_ids=['cmp-case-runner-rust:ADD_LOCK@5871674f486a'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   47.169  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDN@5871674f486a, candidate_sha256=sha256:5871674f486a95b21a52d65c6141c3f1ab8139b1..., contract_id=w2m.guest_state.fault_context, contract_versi
  t=   47.169  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDN@5871674f486a, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+380
  t=   47.169  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDN@5871674f486a, comparison_ids=['cmp-case-runner-rust:ANDN@5871674f486a'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   47.169  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDNPS@5871674f486a, candidate_sha256=sha256:5871674f486a95b21a52d65c6141c3f1ab8139b1..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=   47.169  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDNPS@5871674f486a, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+3
  t=   47.169  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDNPS@5871674f486a, comparison_ids=['cmp-case-runner-rust:ANDNPS@5871674f486a'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   47.173  case_runs -> CANDIDATE_REJECTED (candidate_id=case-runner-rust:FCOS@5871674f486a, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, discrepancy_id=case-runner-rust-1793213026127407000-FCOS, reason=2 diff
  t=   47.173  case_runs -> REDUCTION_COMPLETED (comparison_id=cmp-case-runner-rust:FCOS@5871674f486a, minimized_program_sha256=sha256:88781aa2b72dfb74d1d27b7e8be6135f9ee5ecf7..., preserved_mismatch=True, reduction_id=case-runner-rust-
  t=   47.173  case_runs -> DISCREPANCY_REPORTED (comparison_id=cmp-case-runner-rust:FCOS@5871674f486a, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, destination_profile_id=w2m the Rust translator (crates/translato
  t=   47.173  case_runs -> DISCREPANCY_ROUTED (discrepancy_id=case-runner-rust-1793213026127407000-FCOS, grounds=the Intel CPU under Windows is the reference; t..., reference_conforms=True, repair_path=implementation_repair)
  t=   47.174  case_runs -> CANDIDATE_REJECTED (candidate_id=case-runner-rust:FSIN@5871674f486a, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, discrepancy_id=case-runner-rust-1793213026127407000-FSIN, reason=2 diff
  t=   47.174  case_runs -> REDUCTION_COMPLETED (comparison_id=cmp-case-runner-rust:FSIN@5871674f486a, minimized_program_sha256=sha256:04fdb6d6870cc53479d0a9c26b94fb42368aa9d7..., preserved_mismatch=True, reduction_id=case-runner-rust-
  t=   47.174  case_runs -> DISCREPANCY_REPORTED (comparison_id=cmp-case-runner-rust:FSIN@5871674f486a, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, destination_profile_id=w2m the Rust translator (crates/translato
  t=   47.174  case_runs -> DISCREPANCY_ROUTED (discrepancy_id=case-runner-rust-1793213026127407000-FSIN, grounds=the Intel CPU under Windows is the reference; t..., reference_conforms=True, repair_path=implementation_repair)
  t=   47.192  case_runs -> SESSION_COMPLETED (accepted_count=266, discrepancy_count=2, elapsed_seconds=46.984508667, outcome=completed_with_discrepancies)
  ... CANDIDATE_GENERATED 265 more
  ... COMPARISON_COMPLETED 265 more
  ... CANDIDATE_ACCEPTED 263 more
case-runner-rust-1793213020030000000: SESSION_INIT 1, CANDIDATE_GENERATED 177, COMPARISON_COMPLETED 177, CANDIDATE_ACCEPTED 177, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    5.174  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.12)
  t=   26.109  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADC@5871674f486a, candidate_sha256=sha256:5871674f486a95b21a52d65c6141c3f1ab8139b1..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   26.110  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADC@5871674f486a, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   26.110  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADC@5871674f486a, comparison_ids=['cmp-case-runner-rust:ADC@5871674f486a'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   26.110  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@5871674f486a, candidate_sha256=sha256:5871674f486a95b21a52d65c6141c3f1ab8139b1..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   26.110  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@5871674f486a, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   26.110  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@5871674f486a, comparison_ids=['cmp-case-runner-rust:ADD@5871674f486a'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   26.110  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@5871674f486a, candidate_sha256=sha256:5871674f486a95b21a52d65c6141c3f1ab8139b1..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   26.110  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@5871674f486a, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+38
  t=   26.110  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@5871674f486a, comparison_ids=['cmp-case-runner-rust:ADDPD@5871674f486a'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   26.125  case_runs -> SESSION_COMPLETED (accepted_count=177, discrepancy_count=0, elapsed_seconds=25.920769167, outcome=completed)
  ... CANDIDATE_GENERATED 174 more
  ... COMPARISON_COMPLETED 174 more
  ... CANDIDATE_ACCEPTED 174 more
case-runner-rust-1793213020060000000: SESSION_INIT 1, CANDIDATE_GENERATED 107, COMPARISON_COMPLETED 107, CANDIDATE_ACCEPTED 107, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    5.174  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz; ..., purpose=conformance, vocab_version=0.12)
  t=   16.890  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@5871674f486a, candidate_sha256=sha256:5871674f486a95b21a52d65c6141c3f1ab8139b1..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   16.891  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@5871674f486a, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, rel
  t=   16.891  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@5871674f486a, comparison_ids=['cmp-case-runner-rust:ADD@5871674f486a'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   16.891  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@5871674f486a, candidate_sha256=sha256:5871674f486a95b21a52d65c6141c3f1ab8139b1..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   16.891  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@5871674f486a, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   16.891  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@5871674f486a, comparison_ids=['cmp-case-runner-rust:ADDPD@5871674f486a'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   16.891  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPS@5871674f486a, candidate_sha256=sha256:5871674f486a95b21a52d65c6141c3f1ab8139b1..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   16.891  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPS@5871674f486a, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   16.891  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPS@5871674f486a, comparison_ids=['cmp-case-runner-rust:ADDPS@5871674f486a'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   16.900  case_runs -> SESSION_COMPLETED (accepted_count=107, discrepancy_count=0, elapsed_seconds=16.641399667, outcome=completed)
  ... CANDIDATE_GENERATED 104 more
  ... COMPARISON_COMPLETED 104 more
  ... CANDIDATE_ACCEPTED 104 more
case-runner-rust-1793213020090000000: SESSION_INIT 1, CANDIDATE_GENERATED 26, COMPARISON_COMPLETED 26, CANDIDATE_ACCEPTED 26, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    5.174  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) 6973P-C; gha-windows-2025-x64-..., purpose=conformance, vocab_version=0.12)
  t=    7.236  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:MOVHPS@5871674f486a, candidate_sha256=sha256:5871674f486a95b21a52d65c6141c3f1ab8139b1..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=    7.236  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:MOVHPS@5871674f486a, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exa
  t=    7.236  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:MOVHPS@5871674f486a, comparison_ids=['cmp-case-runner-rust:MOVHPS@5871674f486a'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    7.236  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PACKUSWB@5871674f486a, candidate_sha256=sha256:5871674f486a95b21a52d65c6141c3f1ab8139b1..., contract_id=w2m.guest_state.fault_context, contract_v
  t=    7.236  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PACKUSWB@5871674f486a, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=e
  t=    7.236  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PACKUSWB@5871674f486a, comparison_ids=['cmp-case-runner-rust:PACKUSWB@5871674f486a'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    7.236  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PADDQ@5871674f486a, candidate_sha256=sha256:5871674f486a95b21a52d65c6141c3f1ab8139b1..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=    7.236  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PADDQ@5871674f486a, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exac
  t=    7.236  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PADDQ@5871674f486a, comparison_ids=['cmp-case-runner-rust:PADDQ@5871674f486a'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=    7.238  case_runs -> SESSION_COMPLETED (accepted_count=26, discrepancy_count=0, elapsed_seconds=7.040276917, outcome=completed)
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

> - On four `case-runner-rust` records written through the new harness command: Sprint 183's verdicts, unchanged.

## 3. Delta

### emits_check

- fired, not named in Emits: `CANDIDATE_ACCEPTED` (576), `CANDIDATE_GENERATED` (578), `CANDIDATE_REJECTED` (2), `COMPARISON_COMPLETED` (578), `DISCREPANCY_REPORTED` (2), `DISCREPANCY_ROUTED` (2), `REDUCTION_COMPLETED` (2), `SESSION_COMPLETED` (4), `SESSION_INIT` (4)
- the card's Emits names no tag, and the sprint's records fired the ones above

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 187: its Rubber Duck Pass and close.
