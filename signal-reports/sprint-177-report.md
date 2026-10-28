# SIGNAL_REPORT — Sprint 177 — One lifter

Made by `signal_report 177` from 4 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 177.

## 1. Observed

### scope_confirmation

> reviewer-2's Medium 1 of 2026-10-28, and its repeat in Sprint 175. The Python has one lifter: `lift.py`'s `lift`
> emits `("mmx", ic, dst, src, imm)` through `lift_mmx` (307-324) and `("x87", ic, opcode, reg, st, mem)` through
> `rules/x87.py`, and both the model and the lowering read those ops. The Rust has the translator's `lift`, which
> refuses MMX forms (`lift.rs` 626-629, "the MMX forms are a later group") and has no rule for the 17 x87 classes, and
> two more lifters in `crates/w2m_vm/src/model.rs`: `lift_mmx` (Sprint 174) and `lift_x87` (Sprint 175). `MMX_ONLY` is
> defined twice (`lift.rs` 597, `model.rs` 998).

### work_performed

- 9ef861104 Sprint 177: one lifter
- 9d15d9bde Sprint 177's card: one lifter

### signal_trace

```
case-runner-rust-1793191799691580000: SESSION_INIT 1, CANDIDATE_GENERATED 235, COMPARISON_COMPLETED 235, CANDIDATE_ACCEPTED 233, CANDIDATE_REJECTED 2, REDUCTION_COMPLETED 2, DISCREPANCY_REPORTED 2, DISCREPANCY_ROUTED 2, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    9.276  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.12)
  t=   50.710  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD_LOCK@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_v
  t=   50.711  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD_LOCK@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955
  t=   50.711  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD_LOCK@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:ADD_LOCK@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   50.711  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDN@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_versi
  t=   50.711  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDN@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+380
  t=   50.711  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDN@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:ANDN@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   50.711  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ANDNPS@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=   50.711  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ANDNPS@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38029980267+38039360955+3
  t=   50.711  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ANDNPS@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:ANDNPS@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   50.715  case_runs -> CANDIDATE_REJECTED (candidate_id=case-runner-rust:FCOS@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, discrepancy_id=case-runner-rust-1793191809115963000-FCOS, reason=2 diff
  t=   50.715  case_runs -> REDUCTION_COMPLETED (comparison_id=cmp-case-runner-rust:FCOS@4698e7819ca9, minimized_program_sha256=sha256:88781aa2b72dfb74d1d27b7e8be6135f9ee5ecf7..., preserved_mismatch=True, reduction_id=case-runner-rust-
  t=   50.715  case_runs -> DISCREPANCY_REPORTED (comparison_id=cmp-case-runner-rust:FCOS@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, destination_profile_id=w2m the Rust translator (crates/translato
  t=   50.715  case_runs -> DISCREPANCY_ROUTED (discrepancy_id=case-runner-rust-1793191809115963000-FCOS, grounds=the Intel CPU under Windows is the reference; t..., reference_conforms=True, repair_path=implementation_repair)
  t=   50.716  case_runs -> CANDIDATE_REJECTED (candidate_id=case-runner-rust:FSIN@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, discrepancy_id=case-runner-rust-1793191809115963000-FSIN, reason=2 diff
  t=   50.716  case_runs -> REDUCTION_COMPLETED (comparison_id=cmp-case-runner-rust:FSIN@4698e7819ca9, minimized_program_sha256=sha256:04fdb6d6870cc53479d0a9c26b94fb42368aa9d7..., preserved_mismatch=True, reduction_id=case-runner-rust-
  t=   50.716  case_runs -> DISCREPANCY_REPORTED (comparison_id=cmp-case-runner-rust:FSIN@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, destination_profile_id=w2m the Rust translator (crates/translato
  t=   50.716  case_runs -> DISCREPANCY_ROUTED (discrepancy_id=case-runner-rust-1793191809115963000-FSIN, grounds=the Intel CPU under Windows is the reference; t..., reference_conforms=True, repair_path=implementation_repair)
  t=   50.730  case_runs -> SESSION_COMPLETED (accepted_count=233, discrepancy_count=2, elapsed_seconds=50.410371042, outcome=completed_with_discrepancies)
  ... CANDIDATE_GENERATED 232 more
  ... COMPARISON_COMPLETED 232 more
  ... CANDIDATE_ACCEPTED 230 more
case-runner-rust-1793191799719220000: SESSION_INIT 1, CANDIDATE_GENERATED 177, COMPARISON_COMPLETED 177, CANDIDATE_ACCEPTED 177, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    9.276  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-20..., purpose=conformance, vocab_version=0.12)
  t=   30.125  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADC@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   30.126  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADC@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   30.126  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADC@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:ADC@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   30.126  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   30.126  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+3803
  t=   30.126  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:ADD@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   30.126  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   30.126  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011862284+38031723882+38
  t=   30.126  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:ADDPD@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   30.141  case_runs -> SESSION_COMPLETED (accepted_count=177, discrepancy_count=0, elapsed_seconds=29.820083958, outcome=completed)
  ... CANDIDATE_GENERATED 174 more
  ... COMPARISON_COMPLETED 174 more
  ... CANDIDATE_ACCEPTED 174 more
case-runner-rust-1793191799749824000: SESSION_INIT 1, CANDIDATE_GENERATED 107, COMPARISON_COMPLETED 107, CANDIDATE_ACCEPTED 107, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    9.277  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz; ..., purpose=conformance, vocab_version=0.12)
  t=   20.532  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADD@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_versio
  t=   20.532  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADD@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, rel
  t=   20.532  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADD@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:ADD@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   20.532  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPD@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   20.532  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPD@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   20.532  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPD@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:ADDPD@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   20.532  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:ADDPS@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   20.532  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:ADDPS@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38011858047+38044974055, r
  t=   20.532  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:ADDPS@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:ADDPS@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   20.541  case_runs -> SESSION_COMPLETED (accepted_count=107, discrepancy_count=0, elapsed_seconds=20.225281917, outcome=completed)
  ... CANDIDATE_GENERATED 104 more
  ... COMPARISON_COMPLETED 104 more
  ... CANDIDATE_ACCEPTED 104 more
case-runner-rust-1793191799780988000: SESSION_INIT 1, CANDIDATE_GENERATED 26, COMPARISON_COMPLETED 26, CANDIDATE_ACCEPTED 26, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    9.256  case_runs -> SESSION_INIT (contract_version=0.2.0, harness_version=0.1.0, profile=Intel(R) Xeon(R) 6973P-C; gha-windows-2025-x64-..., purpose=conformance, vocab_version=0.12)
  t=   11.377  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:MOVHPS@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_ver
  t=   11.377  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:MOVHPS@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exa
  t=   11.377  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:MOVHPS@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:MOVHPS@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   11.377  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PACKUSWB@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_v
  t=   11.377  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PACKUSWB@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=e
  t=   11.377  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PACKUSWB@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:PACKUSWB@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   11.377  case_runs -> CANDIDATE_GENERATED (attempt=1, candidate_id=case-runner-rust:PADDQ@4698e7819ca9, candidate_sha256=sha256:4698e7819ca9d13975e904ea3b358c4232e6ffc3..., contract_id=w2m.guest_state.fault_context, contract_vers
  t=   11.377  case_runs -> COMPARISON_COMPLETED (comparison_id=cmp-case-runner-rust:PADDQ@4698e7819ca9, contract_id=w2m.guest_state.fault_context, contract_version=0.2.0, first_divergence=None, reference_ref=38052145991, relation=exac
  t=   11.377  case_runs -> CANDIDATE_ACCEPTED (candidate_id=case-runner-rust:PADDQ@4698e7819ca9, comparison_ids=['cmp-case-runner-rust:PADDQ@4698e7819ca9'], contract_id=w2m.guest_state.fault_context, contract_version=0.2.0)
  t=   11.379  case_runs -> SESSION_COMPLETED (accepted_count=26, discrepancy_count=0, elapsed_seconds=11.087511584, outcome=completed)
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

> - On four `case-runner-rust` records: the same class verdicts as Sprint 175's records, unchanged by the move.

## 3. Delta

### emits_check

- fired, not named in Emits: `CANDIDATE_ACCEPTED` (543), `CANDIDATE_GENERATED` (545), `CANDIDATE_REJECTED` (2), `COMPARISON_COMPLETED` (545), `DISCREPANCY_REPORTED` (2), `DISCREPANCY_ROUTED` (2), `REDUCTION_COMPLETED` (2), `SESSION_COMPLETED` (4), `SESSION_INIT` (4)
- the card's Emits names no tag, and the sprint's records fired the ones above

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 177: its Rubber Duck Pass and close.
