# SIGNAL_REPORT — Sprint 126 — The vocabulary in Rust

Made by `signal_report 126` on 2026-10-24, after the sprint's close (hand-author log 2026-10-24), from 2 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 126.

## 1. Observed

### scope_confirmation

> Step 2 of `docs/rust-port-plan-2026-10-21.md`, its vocabulary part. A Rust binary, `vocabulary_generator`, reads the
> current vocabulary (`signals/current.json`, then `signals/0.9.json`), checks it as `scripts/check_vocab.py` does, and
> writes two files from it: the Rust types of all 62 tags (`crates/events/src/generated.rs`) and substrate's schema file
> for the 52 harness-plane tags (`harness/schemas/harness-events.json`, which replaces the hand-written
> `harness/schemas/session.json` of Sprint 125). With `--check` it writes nothing and fails when either committed file
> differs from a fresh generation. `session_events` writes its two events through the generated types.

### work_performed

- d7831eb28 Sprint 126 closed: CI run 40123687315
- 279c29377 Sprint 126: the session-check and vocabulary-samples records, from 7621d9b7
- 7621d9b74 Sprint 126: the vocabulary in Rust

### signal_trace

```
session-check-20261024T190510: SESSION_INIT 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    0.215  session_events -> SESSION_INIT (contract_version=0.0, harness_version=0.1.0, profile=w2m-session-check, purpose=conformance, vocab_version=0.9)
  t=    0.215  session_events -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=4.225e-05, outcome=completed)
vocabulary-samples-20261024T190510: SESSION_INIT 2, SESSION_COMPLETED 2, EVIDENCE_RECORDED 2, EVIDENCE_SUPERSEDED 2, HYPOTHESIS_RESOLVED 2, CONTRACT_PROPOSED 2, CONTRACT_ACCEPTED 2, CONTRACT_REVISED 2, CONTRACT_REJECTED 2, DEPENDENTS_INVALIDATED 2, PROBE_AUTHORED 2, EXECUTION_STARTED 2, EXECUTION_COMPLETED 2, EXECUTION_FAILED 2, TRACE_INGESTED 2, TRACE_REJECTED 2, MODEL_CHECK_STARTED 2, MODEL_CHECK_COMPLETED 2, COMPARISON_COMPLETED 2, REDUCTION_COMPLETED 2, DISCREPANCY_REPORTED 2, DISCREPANCY_ROUTED 2, DISCREPANCY_RESOLVED 2, CANDIDATE_GENERATED 2, CANDIDATE_ACCEPTED 2, CANDIDATE_REJECTED 2, ACCEPTANCE_CONDITION_ALTERED 2, CODE_TARGET_DISCOVERED 2, TRANSLATION_PUBLISHED 2, TRANSLATION_INVALIDATED 2, STALE_TRANSLATION_ENTERED 2, ARTIFACT_BUILT 2, ARTIFACT_ACCEPTED 2, ARTIFACT_INVALIDATED 2, CAPABILITY_PROBED 2, BACKEND_EVALUATED 2, BACKEND_SELECTED 2, MEASUREMENT_RECORDED 2, DISTRIBUTION_RESOLVED 2, DISTRIBUTION_ACQUIRED 2, ACQUISITION_FAILED 2, APPLICATION_PREPARED 2, STEAM_SESSION_ESTABLISHED 2, STEAM_SESSION_FAILED 2, BRIDGE_CALL_UNMAPPED 2, APPLICATION_LAUNCHED 2, APPLICATION_EXITED 2, SAVE_STATE_VERIFIED 2, APPLICATION_UPDATED 2, APPLICATION_REMOVED 2, PRODUCT_LIFECYCLE_QUALIFIED 2, SERVICE_UNIMPLEMENTED 2; producers failed 0; not finalised 0
  t=    0.224  vocabulary_samples -> SESSION_INIT (contract_version=0.9.1, harness_version=0.9.1, profile=01JSAMPLE, purpose=product, vocab_version=0.9.1)
  t=    0.224  vocabulary_samples -> SESSION_INIT (contract_version=0.9.1, harness_version=0.9.1, profile=01JSAMPLE, purpose=product, vocab_version=0.9.1)
  t=    0.224  vocabulary_samples -> SESSION_COMPLETED (accepted_count=9007199254740991, discrepancy_count=9007199254740991, elapsed_seconds=0.5, outcome=aborted)
  t=    0.224  vocabulary_samples -> SESSION_COMPLETED (accepted_count=9007199254740991, discrepancy_count=9007199254740991, elapsed_seconds=0.5, outcome=aborted)
  t=    0.224  vocabulary_samples -> EVIDENCE_RECORDED (applicability=a "quoted" line\nthen é, contract_id=windows.sample.contract, distinguishing_check=a "quoted" line\nthen é, evidence_category=hypothesis, evidence_id=01JSAMPLE, sour
  t=    0.224  vocabulary_samples -> EVIDENCE_RECORDED (applicability=a "quoted" line\nthen é, contract_id=windows.sample.contract, distinguishing_check=None, evidence_category=hypothesis, evidence_id=01JSAMPLE, source_ref=a "quoted" l
  t=    0.225  vocabulary_samples -> EVIDENCE_SUPERSEDED (evidence_id=01JSAMPLE, reason=a "quoted" line\nthen é, superseded_by=01JSAMPLE)
  t=    0.225  vocabulary_samples -> EVIDENCE_SUPERSEDED (evidence_id=01JSAMPLE, reason=a "quoted" line\nthen é, superseded_by=01JSAMPLE)
  t=    0.225  vocabulary_samples -> HYPOTHESIS_RESOLVED (deciding_evidence_ids=['01JSAMPLE', '01JSAMPLE'], evidence_id=01JSAMPLE, resolution=inconclusive)
  t=    0.225  vocabulary_samples -> HYPOTHESIS_RESOLVED (deciding_evidence_ids=['01JSAMPLE', '01JSAMPLE'], evidence_id=01JSAMPLE, resolution=inconclusive)
  t=    0.225  vocabulary_samples -> CONTRACT_PROPOSED (contract_id=windows.sample.contract, evidence_ids=['01JSAMPLE', '01JSAMPLE'], proposer=generator, version=0.9.1)
  t=    0.225  vocabulary_samples -> CONTRACT_PROPOSED (contract_id=windows.sample.contract, evidence_ids=['01JSAMPLE', '01JSAMPLE'], proposer=generator, version=0.9.1)
  t=    0.225  vocabulary_samples -> CONTRACT_ACCEPTED (applicability=a "quoted" line\nthen é, contract_id=windows.sample.contract, evidence_ids=['01JSAMPLE', '01JSAMPLE'], open_question_count=9007199254740991, version=0.9.1)
  t=    0.225  vocabulary_samples -> CONTRACT_ACCEPTED (applicability=a "quoted" line\nthen é, contract_id=windows.sample.contract, evidence_ids=['01JSAMPLE', '01JSAMPLE'], open_question_count=9007199254740991, version=0.9.1)
  t=    0.225  vocabulary_samples -> CONTRACT_REVISED (contract_id=windows.sample.contract, evidence_ids=['01JSAMPLE', '01JSAMPLE'], from_version=0.9.1, rationale_ref=a "quoted" line\nthen é, to_version=0.9.1, triggering_discrepancy_id
  t=    0.225  vocabulary_samples -> CONTRACT_REVISED (contract_id=windows.sample.contract, evidence_ids=['01JSAMPLE', '01JSAMPLE'], from_version=0.9.1, rationale_ref=a "quoted" line\nthen é, to_version=0.9.1, triggering_discrepancy_id
  t=    0.225  vocabulary_samples -> CONTRACT_REJECTED (contract_id=windows.sample.contract, reason=a "quoted" line\nthen é, version=0.9.1)
  t=    0.225  vocabulary_samples -> CONTRACT_REJECTED (contract_id=windows.sample.contract, reason=a "quoted" line\nthen é, version=0.9.1)
  t=    0.225  vocabulary_samples -> DEPENDENTS_INVALIDATED (artifact_count=9007199254740991, candidate_count=9007199254740991, contract_id=windows.sample.contract, evidence_count=9007199254740991, from_version=0.9.1, to_version=0.9.1)
  t=    0.225  vocabulary_samples -> DEPENDENTS_INVALIDATED (artifact_count=9007199254740991, candidate_count=9007199254740991, contract_id=windows.sample.contract, evidence_count=9007199254740991, from_version=0.9.1, to_version=0.9.1)
  t=    0.225  vocabulary_samples -> PROBE_AUTHORED (contract_ids=['windows.sample.contract', 'windows.sample.contract'], origin=reduced, parent_probe_id=01JSAMPLE, probe_id=01JSAMPLE, probe_sha256=sha256:0123456789abcdef0123456789abcd
  t=    0.225  vocabulary_samples -> PROBE_AUTHORED (contract_ids=['windows.sample.contract', 'windows.sample.contract'], origin=reduced, parent_probe_id=None, probe_id=01JSAMPLE, probe_sha256=sha256:0123456789abcdef0123456789abcdef012
  t=    0.225  vocabulary_samples -> EXECUTION_STARTED (execution_id=01JSAMPLE, initial_state_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., injected_inputs_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., kill_gr
  t=    0.225  vocabulary_samples -> EXECUTION_STARTED (execution_id=01JSAMPLE, initial_state_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., injected_inputs_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., kill_gr
  t=    0.225  vocabulary_samples -> EXECUTION_COMPLETED (elapsed_seconds=0.5, execution_id=01JSAMPLE, exit_code=-9007199254740991, exit_status=killed, guest_line_count=9007199254740991, trace_sha256=sha256:0123456789abcdef0123456789ab
  t=    0.225  vocabulary_samples -> EXECUTION_COMPLETED (elapsed_seconds=None, execution_id=01JSAMPLE, exit_code=None, exit_status=killed, guest_line_count=9007199254740991, trace_sha256=sha256:0123456789abcdef0123456789abcdef01234567
  t=    0.225  vocabulary_samples -> EXECUTION_FAILED (diagnostic=a "quoted" line\nthen é, execution_id=01JSAMPLE, failure=harness_error)
  t=    0.225  vocabulary_samples -> EXECUTION_FAILED (diagnostic=a "quoted" line\nthen é, execution_id=01JSAMPLE, failure=harness_error)
  t=    0.225  vocabulary_samples -> TRACE_INGESTED (execution_id=01JSAMPLE, line_count=9007199254740991, thread_count=9007199254740991, trace_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., vocab_version=0.9.1)
  t=    0.226  vocabulary_samples -> TRACE_INGESTED (execution_id=01JSAMPLE, line_count=9007199254740991, thread_count=9007199254740991, trace_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., vocab_version=0.9.1)
  t=    0.226  vocabulary_samples -> TRACE_REJECTED (execution_id=01JSAMPLE, first_invalid_line=9007199254740991, reason=a "quoted" line\nthen é, trace_sha256=sha256:0123456789abcdef0123456789abcdef01234567...)
  t=    0.226  vocabulary_samples -> TRACE_REJECTED (execution_id=01JSAMPLE, first_invalid_line=9007199254740991, reason=a "quoted" line\nthen é, trace_sha256=sha256:0123456789abcdef0123456789abcdef01234567...)
  t=    0.226  vocabulary_samples -> MODEL_CHECK_STARTED (contract_id=windows.sample.contract, contract_version=0.9.1, model_check_id=01JSAMPLE, program_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., state_bound=90071992547
  t=    0.226  vocabulary_samples -> MODEL_CHECK_STARTED (contract_id=windows.sample.contract, contract_version=0.9.1, model_check_id=01JSAMPLE, program_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., state_bound=90071992547
  t=    0.226  vocabulary_samples -> MODEL_CHECK_COMPLETED (exhaustive=True, model_check_id=01JSAMPLE, outcome_set_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., permitted_outcome_count=9007199254740991, states_explored=900
  t=    0.226  vocabulary_samples -> MODEL_CHECK_COMPLETED (exhaustive=True, model_check_id=01JSAMPLE, outcome_set_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., permitted_outcome_count=9007199254740991, states_explored=900
  t=    0.226  vocabulary_samples -> COMPARISON_COMPLETED (comparison_id=01JSAMPLE, contract_id=windows.sample.contract, contract_version=0.9.1, first_divergence=a "quoted" line\nthen é, reference_ref=01JSAMPLE, relation=progress_oblig
  t=    0.226  vocabulary_samples -> COMPARISON_COMPLETED (comparison_id=01JSAMPLE, contract_id=windows.sample.contract, contract_version=0.9.1, first_divergence=None, reference_ref=01JSAMPLE, relation=progress_obligation, rule_ref=Non
  t=    0.226  vocabulary_samples -> REDUCTION_COMPLETED (comparison_id=01JSAMPLE, minimized_program_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., preserved_mismatch=True, reduction_id=01JSAMPLE, source_program_sha256=sha2
  t=    0.226  vocabulary_samples -> REDUCTION_COMPLETED (comparison_id=01JSAMPLE, minimized_program_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., preserved_mismatch=True, reduction_id=01JSAMPLE, source_program_sha256=sha2
  t=    0.226  vocabulary_samples -> DISCREPANCY_REPORTED (comparison_id=01JSAMPLE, contract_id=windows.sample.contract, contract_version=0.9.1, destination_profile_id=01JSAMPLE, discrepancy_id=01JSAMPLE, first_divergence=a "quoted" li
  t=    0.226  vocabulary_samples -> DISCREPANCY_REPORTED (comparison_id=01JSAMPLE, contract_id=windows.sample.contract, contract_version=0.9.1, destination_profile_id=01JSAMPLE, discrepancy_id=01JSAMPLE, first_divergence=a "quoted" li
  t=    0.226  vocabulary_samples -> DISCREPANCY_ROUTED (discrepancy_id=01JSAMPLE, grounds=a "quoted" line\nthen é, reference_conforms=True, repair_path=undetermined)
  t=    0.226  vocabulary_samples -> DISCREPANCY_ROUTED (discrepancy_id=01JSAMPLE, grounds=a "quoted" line\nthen é, reference_conforms=True, repair_path=undetermined)
  t=    0.226  vocabulary_samples -> DISCREPANCY_RESOLVED (discrepancy_id=01JSAMPLE, resolution=profile_excluded, resolving_ref=01JSAMPLE)
  t=    0.226  vocabulary_samples -> DISCREPANCY_RESOLVED (discrepancy_id=01JSAMPLE, resolution=profile_excluded, resolving_ref=01JSAMPLE)
  t=    0.226  vocabulary_samples -> CANDIDATE_GENERATED (attempt=9007199254740991, candidate_id=01JSAMPLE, candidate_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., contract_id=windows.sample.contract, contract_version=0.9.
  t=    0.226  vocabulary_samples -> CANDIDATE_GENERATED (attempt=9007199254740991, candidate_id=01JSAMPLE, candidate_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., contract_id=windows.sample.contract, contract_version=0.9.
  t=    0.226  vocabulary_samples -> CANDIDATE_ACCEPTED (candidate_id=01JSAMPLE, comparison_ids=['01JSAMPLE', '01JSAMPLE'], contract_id=windows.sample.contract, contract_version=0.9.1)
  t=    0.226  vocabulary_samples -> CANDIDATE_ACCEPTED (candidate_id=01JSAMPLE, comparison_ids=['01JSAMPLE', '01JSAMPLE'], contract_id=windows.sample.contract, contract_version=0.9.1)
  t=    0.226  vocabulary_samples -> CANDIDATE_REJECTED (candidate_id=01JSAMPLE, contract_id=windows.sample.contract, contract_version=0.9.1, discrepancy_id=01JSAMPLE, reason=a "quoted" line\nthen é)
  t=    0.226  vocabulary_samples -> CANDIDATE_REJECTED (candidate_id=01JSAMPLE, contract_id=windows.sample.contract, contract_version=0.9.1, discrepancy_id=None, reason=a "quoted" line\nthen é)
  t=    0.226  vocabulary_samples -> ACCEPTANCE_CONDITION_ALTERED (altered=evidence, candidate_id=01JSAMPLE, expected_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., observed_sha256=sha256:0123456789abcdef0123456789abcdef012
  t=    0.227  vocabulary_samples -> ACCEPTANCE_CONDITION_ALTERED (altered=evidence, candidate_id=01JSAMPLE, expected_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., observed_sha256=sha256:0123456789abcdef0123456789abcdef012
  t=    0.227  vocabulary_samples -> CODE_TARGET_DISCOVERED (discovery=generated_code, guest_address=9007199254740991, image_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., stage=execution)
  t=    0.227  vocabulary_samples -> CODE_TARGET_DISCOVERED (discovery=generated_code, guest_address=9007199254740991, image_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., stage=execution)
  t=    0.227  vocabulary_samples -> TRANSLATION_PUBLISHED (artifact_id=01JSAMPLE, code_version=9007199254740991, contract_version=0.9.1, guest_range_end=9007199254740991, guest_range_start=9007199254740991, image_sha256=sha256:0123456
  t=    0.227  vocabulary_samples -> TRANSLATION_PUBLISHED (artifact_id=01JSAMPLE, code_version=9007199254740991, contract_version=0.9.1, guest_range_end=9007199254740991, guest_range_start=9007199254740991, image_sha256=sha256:0123456
  t=    0.227  vocabulary_samples -> TRANSLATION_INVALIDATED (cause=assumption_failed, translation_id=01JSAMPLE)
  t=    0.227  vocabulary_samples -> TRANSLATION_INVALIDATED (cause=assumption_failed, translation_id=01JSAMPLE)
  t=    0.227  vocabulary_samples -> STALE_TRANSLATION_ENTERED (actual_code_version=9007199254740991, expected_code_version=9007199254740991, guest_address=9007199254740991, translation_id=01JSAMPLE)
  t=    0.227  vocabulary_samples -> STALE_TRANSLATION_ENTERED (actual_code_version=9007199254740991, expected_code_version=9007199254740991, guest_address=9007199254740991, translation_id=01JSAMPLE)
  t=    0.227  vocabulary_samples -> ARTIFACT_BUILT (application_profile_id=01JSAMPLE, artifact_id=01JSAMPLE, artifact_kind=bundle, compiler_version=0.9.1, source_contract_version=0.9.1, source_sha256=sha256:0123456789abcdef0123456789a
  t=    0.227  vocabulary_samples -> ARTIFACT_BUILT (application_profile_id=None, artifact_id=01JSAMPLE, artifact_kind=bundle, compiler_version=0.9.1, source_contract_version=0.9.1, source_sha256=sha256:0123456789abcdef0123456789abcdef
  t=    0.227  vocabulary_samples -> ARTIFACT_ACCEPTED (artifact_id=01JSAMPLE, comparison_ids=['01JSAMPLE', '01JSAMPLE'])
  t=    0.227  vocabulary_samples -> ARTIFACT_ACCEPTED (artifact_id=01JSAMPLE, comparison_ids=['01JSAMPLE', '01JSAMPLE'])
  t=    0.227  vocabulary_samples -> ARTIFACT_INVALIDATED (artifact_id=01JSAMPLE, cause=validity_condition_failed)
  t=    0.227  vocabulary_samples -> ARTIFACT_INVALIDATED (artifact_id=01JSAMPLE, cause=validity_condition_failed)
  t=    0.227  vocabulary_samples -> CAPABILITY_PROBED (capability=guest_fp_to_int_x86, detail=a "quoted" line\nthen é, execution_id=01JSAMPLE, host_id=01JSAMPLE, outcome=not_established)
  t=    0.227  vocabulary_samples -> CAPABILITY_PROBED (capability=guest_fp_to_int_x86, detail=a "quoted" line\nthen é, execution_id=None, host_id=01JSAMPLE, outcome=not_established)
  t=    0.227  vocabulary_samples -> BACKEND_EVALUATED (application_profile_id=01JSAMPLE, backend=managed_vm, conformance_fail_count=9007199254740991, conformance_pass_count=9007199254740991, destination_profile_id=01JSAMPLE, evaluatio
  t=    0.227  vocabulary_samples -> BACKEND_EVALUATED (application_profile_id=01JSAMPLE, backend=managed_vm, conformance_fail_count=9007199254740991, conformance_pass_count=9007199254740991, destination_profile_id=01JSAMPLE, evaluatio
  t=    0.227  vocabulary_samples -> BACKEND_SELECTED (application_profile_id=01JSAMPLE, backend=managed_vm, decision_ref=a "quoted" line\nthen é, destination_profile_id=01JSAMPLE, evaluation_ids=['01JSAMPLE', '01JSAMPLE'])
  t=    0.227  vocabulary_samples -> BACKEND_SELECTED (application_profile_id=01JSAMPLE, backend=managed_vm, decision_ref=a "quoted" line\nthen é, destination_profile_id=01JSAMPLE, evaluation_ids=['01JSAMPLE', '01JSAMPLE'])
  t=    0.227  vocabulary_samples -> MEASUREMENT_RECORDED (application_profile_id=01JSAMPLE, artifact_state=warm, backend=managed_vm, destination_profile_id=01JSAMPLE, measurement_id=01JSAMPLE, metric=load_average, phase=update, unit=a
  t=    0.227  vocabulary_samples -> MEASUREMENT_RECORDED (application_profile_id=01JSAMPLE, artifact_state=warm, backend=managed_vm, destination_profile_id=01JSAMPLE, measurement_id=01JSAMPLE, metric=load_average, phase=update, unit=a
  t=    0.227  vocabulary_samples -> DISTRIBUTION_RESOLVED (app_id=9007199254740991, build_id=9007199254740991, depot_ids=[9007199254740991, 9007199254740991], platform=a "quoted" line\nthen é)
  t=    0.227  vocabulary_samples -> DISTRIBUTION_RESOLVED (app_id=9007199254740991, build_id=9007199254740991, depot_ids=[9007199254740991, 9007199254740991], platform=a "quoted" line\nthen é)
  t=    0.227  vocabulary_samples -> DISTRIBUTION_ACQUIRED (app_id=9007199254740991, build_id=9007199254740991, bytes=9007199254740991, content_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., tool=steamcmd)
  t=    0.227  vocabulary_samples -> DISTRIBUTION_ACQUIRED (app_id=9007199254740991, build_id=9007199254740991, bytes=9007199254740991, content_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., tool=steamcmd)
  t=    0.227  vocabulary_samples -> ACQUISITION_FAILED (app_id=9007199254740991, diagnostic=a "quoted" line\nthen é, reason=tool_error)
  t=    0.227  vocabulary_samples -> ACQUISITION_FAILED (app_id=9007199254740991, diagnostic=a "quoted" line\nthen é, reason=tool_error)
  t=    0.227  vocabulary_samples -> APPLICATION_PREPARED (application_profile_id=01JSAMPLE, artifact_count=9007199254740991, backend=managed_vm, bundle_id=01JSAMPLE, elapsed_seconds=0.5)
  t=    0.227  vocabulary_samples -> APPLICATION_PREPARED (application_profile_id=01JSAMPLE, artifact_count=9007199254740991, backend=managed_vm, bundle_id=01JSAMPLE, elapsed_seconds=0.5)
  t=    0.227  vocabulary_samples -> STEAM_SESSION_ESTABLISHED (app_id=9007199254740991, bridge_contract_version=0.9.1, user_context_present=True)
  t=    0.227  vocabulary_samples -> STEAM_SESSION_ESTABLISHED (app_id=9007199254740991, bridge_contract_version=0.9.1, user_context_present=True)
  t=    0.227  vocabulary_samples -> STEAM_SESSION_FAILED (app_id=9007199254740991, diagnostic=a "quoted" line\nthen é, reason=bridge_error)
  t=    0.227  vocabulary_samples -> STEAM_SESSION_FAILED (app_id=9007199254740991, diagnostic=a "quoted" line\nthen é, reason=bridge_error)
  t=    0.227  vocabulary_samples -> BRIDGE_CALL_UNMAPPED (interface_name=a "quoted" line\nthen é, interface_version=a "quoted" line\nthen é, method=a "quoted" line\nthen é)
  t=    0.227  vocabulary_samples -> BRIDGE_CALL_UNMAPPED (interface_name=a "quoted" line\nthen é, interface_version=a "quoted" line\nthen é, method=a "quoted" line\nthen é)
  t=    0.227  vocabulary_samples -> APPLICATION_LAUNCHED (application_profile_id=01JSAMPLE, backend=managed_vm, bundle_id=01JSAMPLE, launch_kind=warm)
  t=    0.227  vocabulary_samples -> APPLICATION_LAUNCHED (application_profile_id=01JSAMPLE, backend=managed_vm, bundle_id=01JSAMPLE, launch_kind=warm)
  t=    0.227  vocabulary_samples -> APPLICATION_EXITED (application_profile_id=01JSAMPLE, exit_code=-9007199254740991, exit_kind=killed)
  t=    0.227  vocabulary_samples -> APPLICATION_EXITED (application_profile_id=01JSAMPLE, exit_code=-9007199254740991, exit_kind=killed)
  t=    0.227  vocabulary_samples -> SAVE_STATE_VERIFIED (application_profile_id=01JSAMPLE, state_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., verified=True)
  t=    0.227  vocabulary_samples -> SAVE_STATE_VERIFIED (application_profile_id=01JSAMPLE, state_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., verified=True)
  t=    0.227  vocabulary_samples -> APPLICATION_UPDATED (app_id=9007199254740991, artifacts_invalidated=9007199254740991, from_build_id=9007199254740991, to_build_id=9007199254740991)
  t=    0.227  vocabulary_samples -> APPLICATION_UPDATED (app_id=9007199254740991, artifacts_invalidated=9007199254740991, from_build_id=9007199254740991, to_build_id=9007199254740991)
  t=    0.228  vocabulary_samples -> APPLICATION_REMOVED (app_id=9007199254740991, bytes_removed=9007199254740991, state_retained=True)
  t=    0.228  vocabulary_samples -> APPLICATION_REMOVED (app_id=9007199254740991, bytes_removed=9007199254740991, state_retained=True)
  t=    0.228  vocabulary_samples -> PRODUCT_LIFECYCLE_QUALIFIED (application_profile_id=01JSAMPLE, failed_step=remove, passed=True, profile_scope=a "quoted" line\nthen é, steps_passed=['remove', 'remove'])
  t=    0.228  vocabulary_samples -> PRODUCT_LIFECYCLE_QUALIFIED (application_profile_id=01JSAMPLE, failed_step=None, passed=True, profile_scope=a "quoted" line\nthen é, steps_passed=['remove', 'remove'])
  t=    0.228  vocabulary_samples -> SERVICE_UNIMPLEMENTED (execution_id=01JSAMPLE, guest_address=9007199254740991, instructions_executed=9007199254740991, service=a "quoted" line\nthen é)
  t=    0.228  vocabulary_samples -> SERVICE_UNIMPLEMENTED (execution_id=01JSAMPLE, guest_address=9007199254740991, instructions_executed=9007199254740991, service=a "quoted" line\nthen é)
```

### signal_trace_tags

- `ACCEPTANCE_CONDITION_ALTERED`
- `ACQUISITION_FAILED`
- `APPLICATION_EXITED`
- `APPLICATION_LAUNCHED`
- `APPLICATION_PREPARED`
- `APPLICATION_REMOVED`
- `APPLICATION_UPDATED`
- `ARTIFACT_ACCEPTED`
- `ARTIFACT_BUILT`
- `ARTIFACT_INVALIDATED`
- `BACKEND_EVALUATED`
- `BACKEND_SELECTED`
- `BRIDGE_CALL_UNMAPPED`
- `CANDIDATE_ACCEPTED`
- `CANDIDATE_GENERATED`
- `CANDIDATE_REJECTED`
- `CAPABILITY_PROBED`
- `CODE_TARGET_DISCOVERED`
- `COMPARISON_COMPLETED`
- `CONTRACT_ACCEPTED`
- `CONTRACT_PROPOSED`
- `CONTRACT_REJECTED`
- `CONTRACT_REVISED`
- `DEPENDENTS_INVALIDATED`
- `DISCREPANCY_REPORTED`
- `DISCREPANCY_RESOLVED`
- `DISCREPANCY_ROUTED`
- `DISTRIBUTION_ACQUIRED`
- `DISTRIBUTION_RESOLVED`
- `EVIDENCE_RECORDED`
- `EVIDENCE_SUPERSEDED`
- `EXECUTION_COMPLETED`
- `EXECUTION_FAILED`
- `EXECUTION_STARTED`
- `HYPOTHESIS_RESOLVED`
- `MEASUREMENT_RECORDED`
- `MODEL_CHECK_COMPLETED`
- `MODEL_CHECK_STARTED`
- `PROBE_AUTHORED`
- `PRODUCT_LIFECYCLE_QUALIFIED`
- `REDUCTION_COMPLETED`
- `SAVE_STATE_VERIFIED`
- `SERVICE_UNIMPLEMENTED`
- `SESSION_COMPLETED`
- `SESSION_INIT`
- `STALE_TRANSLATION_ENTERED`
- `STEAM_SESSION_ESTABLISHED`
- `STEAM_SESSION_FAILED`
- `TRACE_INGESTED`
- `TRACE_REJECTED`
- `TRANSLATION_INVALIDATED`
- `TRANSLATION_PUBLISHED`

## 2. Expected

> - SESSION_INIT and SESSION_COMPLETED, as in Sprint 125, now built from the generated types.

## 3. Delta

### emits_check

- `SESSION_COMPLETED`: fired 3 times
- `SESSION_INIT`: fired 3 times
- fired, not named in Emits: `ACCEPTANCE_CONDITION_ALTERED` (2), `ACQUISITION_FAILED` (2), `APPLICATION_EXITED` (2), `APPLICATION_LAUNCHED` (2), `APPLICATION_PREPARED` (2), `APPLICATION_REMOVED` (2), `APPLICATION_UPDATED` (2), `ARTIFACT_ACCEPTED` (2), `ARTIFACT_BUILT` (2), `ARTIFACT_INVALIDATED` (2), `BACKEND_EVALUATED` (2), `BACKEND_SELECTED` (2), `BRIDGE_CALL_UNMAPPED` (2), `CANDIDATE_ACCEPTED` (2), `CANDIDATE_GENERATED` (2), `CANDIDATE_REJECTED` (2), `CAPABILITY_PROBED` (2), `CODE_TARGET_DISCOVERED` (2), `COMPARISON_COMPLETED` (2), `CONTRACT_ACCEPTED` (2), `CONTRACT_PROPOSED` (2), `CONTRACT_REJECTED` (2), `CONTRACT_REVISED` (2), `DEPENDENTS_INVALIDATED` (2), `DISCREPANCY_REPORTED` (2), `DISCREPANCY_RESOLVED` (2), `DISCREPANCY_ROUTED` (2), `DISTRIBUTION_ACQUIRED` (2), `DISTRIBUTION_RESOLVED` (2), `EVIDENCE_RECORDED` (2), `EVIDENCE_SUPERSEDED` (2), `EXECUTION_COMPLETED` (2), `EXECUTION_FAILED` (2), `EXECUTION_STARTED` (2), `HYPOTHESIS_RESOLVED` (2), `MEASUREMENT_RECORDED` (2), `MODEL_CHECK_COMPLETED` (2), `MODEL_CHECK_STARTED` (2), `PROBE_AUTHORED` (2), `PRODUCT_LIFECYCLE_QUALIFIED` (2), `REDUCTION_COMPLETED` (2), `SAVE_STATE_VERIFIED` (2), `SERVICE_UNIMPLEMENTED` (2), `STALE_TRANSLATION_ENTERED` (2), `STEAM_SESSION_ESTABLISHED` (2), `STEAM_SESSION_FAILED` (2), `TRACE_INGESTED` (2), `TRACE_REJECTED` (2), `TRANSLATION_INVALIDATED` (2), `TRANSLATION_PUBLISHED` (2)

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 126: its Rubber Duck Pass and close.
