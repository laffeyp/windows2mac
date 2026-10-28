# SIGNAL_REPORT — Sprint 162 — Vocabulary 0.11: the metrics step 3's measurements need

Made by `signal_report 162` from 1 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 162.

## 1. Observed

### scope_confirmation

> Decision 2026-10-27 (agent-resolved; reviewer-2's review of Sprints 155-161, High 1) closes step 3 of
> `docs/rust-port-plan-2026-10-24.md` with a measurement sprint: an exit's time by kind, the runtime word's hold and
> wait times, and whether a guest WFE exits, each on a record as MEASUREMENT_RECORDED. Vocabulary 0.10's `metric` enum
> has no value for any of them (`signals/0.10.json` 181-203: launch and frame times, `dispatch_ns`, `ns_per_access`,
> `vm_compile_blocks` and P20's batch-run numbers). This sprint adds them by one typed proposal, P24, of the kind P19 and
> P20 used (ENUM_EXTENDED, layer 2), modeled on `frame_time_ms_p50` and `frame_time_ms_p99`:

### work_performed

- 4f43f0805 Sprint 162: vocabulary 0.11, the metrics step 3's measurements need (P24)
- dcd55218a Sprint 162: its card, before its code (vocabulary 0.11)

### signal_trace

```
vocabulary-samples-20261028T055041: SESSION_INIT 2, SESSION_COMPLETED 2, EVIDENCE_RECORDED 2, CONTRACT_PROPOSED 2, CONTRACT_ACCEPTED 2, CONTRACT_REVISED 2, DEPENDENTS_INVALIDATED 2, PROBE_AUTHORED 2, EXECUTION_STARTED 2, EXECUTION_COMPLETED 2, EXECUTION_FAILED 2, TRACE_INGESTED 2, TRACE_REJECTED 2, COMPARISON_COMPLETED 2, REDUCTION_COMPLETED 2, DISCREPANCY_REPORTED 2, DISCREPANCY_ROUTED 2, DISCREPANCY_RESOLVED 2, CANDIDATE_GENERATED 2, CANDIDATE_ACCEPTED 2, CANDIDATE_REJECTED 2, ACCEPTANCE_CONDITION_ALTERED 2, CODE_TARGET_DISCOVERED 2, TRANSLATION_PUBLISHED 2, ARTIFACT_ACCEPTED 2, CAPABILITY_PROBED 2, BACKEND_EVALUATED 2, BACKEND_SELECTED 2, MEASUREMENT_RECORDED 30, DISTRIBUTION_RESOLVED 2, DISTRIBUTION_ACQUIRED 2, ACQUISITION_FAILED 2, SERVICE_UNIMPLEMENTED 2; producers failed 0; not finalised 0
  t=    0.406  vocabulary_samples -> SESSION_INIT (contract_version=0.9.1, harness_version=0.9.1, profile=01JSAMPLE, purpose=product, vocab_version=0.9.1)
  t=    0.407  vocabulary_samples -> SESSION_INIT (contract_version=0.9.1, harness_version=0.9.1, profile=01JSAMPLE, purpose=product, vocab_version=0.9.1)
  t=    0.407  vocabulary_samples -> SESSION_COMPLETED (accepted_count=9007199254740991, discrepancy_count=9007199254740991, elapsed_seconds=0.5, outcome=aborted)
  t=    0.407  vocabulary_samples -> SESSION_COMPLETED (accepted_count=9007199254740991, discrepancy_count=9007199254740991, elapsed_seconds=0.5, outcome=aborted)
  t=    0.407  vocabulary_samples -> EVIDENCE_RECORDED (applicability=a "quoted" line\nthen é, contract_id=windows.sample.contract, distinguishing_check=a "quoted" line\nthen é, evidence_category=hypothesis, evidence_id=01JSAMPLE, sour
  t=    0.407  vocabulary_samples -> EVIDENCE_RECORDED (applicability=a "quoted" line\nthen é, contract_id=windows.sample.contract, distinguishing_check=None, evidence_category=hypothesis, evidence_id=01JSAMPLE, source_ref=a "quoted" l
  t=    0.407  vocabulary_samples -> CONTRACT_PROPOSED (contract_id=windows.sample.contract, evidence_ids=['01JSAMPLE', '01JSAMPLE'], proposer=generator, version=0.9.1)
  t=    0.407  vocabulary_samples -> CONTRACT_PROPOSED (contract_id=windows.sample.contract, evidence_ids=['01JSAMPLE', '01JSAMPLE'], proposer=generator, version=0.9.1)
  t=    0.407  vocabulary_samples -> CONTRACT_ACCEPTED (applicability=a "quoted" line\nthen é, contract_id=windows.sample.contract, evidence_ids=['01JSAMPLE', '01JSAMPLE'], open_question_count=9007199254740991, version=0.9.1)
  t=    0.407  vocabulary_samples -> CONTRACT_ACCEPTED (applicability=a "quoted" line\nthen é, contract_id=windows.sample.contract, evidence_ids=['01JSAMPLE', '01JSAMPLE'], open_question_count=9007199254740991, version=0.9.1)
  t=    0.407  vocabulary_samples -> CONTRACT_REVISED (contract_id=windows.sample.contract, evidence_ids=['01JSAMPLE', '01JSAMPLE'], from_version=0.9.1, rationale_ref=a "quoted" line\nthen é, to_version=0.9.1, triggering_discrepancy_id
  t=    0.408  vocabulary_samples -> CONTRACT_REVISED (contract_id=windows.sample.contract, evidence_ids=['01JSAMPLE', '01JSAMPLE'], from_version=0.9.1, rationale_ref=a "quoted" line\nthen é, to_version=0.9.1, triggering_discrepancy_id
  t=    0.408  vocabulary_samples -> DEPENDENTS_INVALIDATED (artifact_count=9007199254740991, candidate_count=9007199254740991, contract_id=windows.sample.contract, evidence_count=9007199254740991, from_version=0.9.1, to_version=0.9.1)
  t=    0.408  vocabulary_samples -> DEPENDENTS_INVALIDATED (artifact_count=9007199254740991, candidate_count=9007199254740991, contract_id=windows.sample.contract, evidence_count=9007199254740991, from_version=0.9.1, to_version=0.9.1)
  t=    0.408  vocabulary_samples -> PROBE_AUTHORED (contract_ids=['windows.sample.contract', 'windows.sample.contract'], origin=reduced, parent_probe_id=01JSAMPLE, probe_id=01JSAMPLE, probe_sha256=sha256:0123456789abcdef0123456789abcd
  t=    0.408  vocabulary_samples -> PROBE_AUTHORED (contract_ids=['windows.sample.contract', 'windows.sample.contract'], origin=reduced, parent_probe_id=None, probe_id=01JSAMPLE, probe_sha256=sha256:0123456789abcdef0123456789abcdef012
  t=    0.408  vocabulary_samples -> EXECUTION_STARTED (execution_id=01JSAMPLE, initial_state_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., injected_inputs_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., kill_gr
  t=    0.409  vocabulary_samples -> EXECUTION_STARTED (execution_id=01JSAMPLE, initial_state_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., injected_inputs_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., kill_gr
  t=    0.409  vocabulary_samples -> EXECUTION_COMPLETED (elapsed_seconds=0.5, execution_id=01JSAMPLE, exit_code=-9007199254740991, exit_status=killed, guest_line_count=9007199254740991, trace_sha256=sha256:0123456789abcdef0123456789ab
  t=    0.409  vocabulary_samples -> EXECUTION_COMPLETED (elapsed_seconds=None, execution_id=01JSAMPLE, exit_code=None, exit_status=killed, guest_line_count=9007199254740991, trace_sha256=sha256:0123456789abcdef0123456789abcdef01234567
  t=    0.409  vocabulary_samples -> EXECUTION_FAILED (diagnostic=a "quoted" line\nthen é, execution_id=01JSAMPLE, failure=harness_error)
  t=    0.409  vocabulary_samples -> EXECUTION_FAILED (diagnostic=a "quoted" line\nthen é, execution_id=01JSAMPLE, failure=harness_error)
  t=    0.409  vocabulary_samples -> TRACE_INGESTED (execution_id=01JSAMPLE, line_count=9007199254740991, thread_count=9007199254740991, trace_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., vocab_version=0.9.1)
  t=    0.409  vocabulary_samples -> TRACE_INGESTED (execution_id=01JSAMPLE, line_count=9007199254740991, thread_count=9007199254740991, trace_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., vocab_version=0.9.1)
  t=    0.409  vocabulary_samples -> TRACE_REJECTED (execution_id=01JSAMPLE, first_invalid_line=9007199254740991, reason=a "quoted" line\nthen é, trace_sha256=sha256:0123456789abcdef0123456789abcdef01234567...)
  t=    0.409  vocabulary_samples -> TRACE_REJECTED (execution_id=01JSAMPLE, first_invalid_line=9007199254740991, reason=a "quoted" line\nthen é, trace_sha256=sha256:0123456789abcdef0123456789abcdef01234567...)
  t=    0.409  vocabulary_samples -> COMPARISON_COMPLETED (comparison_id=01JSAMPLE, contract_id=windows.sample.contract, contract_version=0.9.1, first_divergence=a "quoted" line\nthen é, reference_ref=01JSAMPLE, relation=progress_oblig
  t=    0.409  vocabulary_samples -> COMPARISON_COMPLETED (comparison_id=01JSAMPLE, contract_id=windows.sample.contract, contract_version=0.9.1, first_divergence=None, reference_ref=01JSAMPLE, relation=progress_obligation, rule_ref=Non
  t=    0.409  vocabulary_samples -> REDUCTION_COMPLETED (comparison_id=01JSAMPLE, minimized_program_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., preserved_mismatch=True, reduction_id=01JSAMPLE, source_program_sha256=sha2
  t=    0.409  vocabulary_samples -> REDUCTION_COMPLETED (comparison_id=01JSAMPLE, minimized_program_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., preserved_mismatch=True, reduction_id=01JSAMPLE, source_program_sha256=sha2
  t=    0.409  vocabulary_samples -> DISCREPANCY_REPORTED (comparison_id=01JSAMPLE, contract_id=windows.sample.contract, contract_version=0.9.1, destination_profile_id=01JSAMPLE, discrepancy_id=01JSAMPLE, first_divergence=a "quoted" li
  t=    0.409  vocabulary_samples -> DISCREPANCY_REPORTED (comparison_id=01JSAMPLE, contract_id=windows.sample.contract, contract_version=0.9.1, destination_profile_id=01JSAMPLE, discrepancy_id=01JSAMPLE, first_divergence=a "quoted" li
  t=    0.409  vocabulary_samples -> DISCREPANCY_ROUTED (discrepancy_id=01JSAMPLE, grounds=a "quoted" line\nthen é, reference_conforms=True, repair_path=undetermined)
  t=    0.409  vocabulary_samples -> DISCREPANCY_ROUTED (discrepancy_id=01JSAMPLE, grounds=a "quoted" line\nthen é, reference_conforms=True, repair_path=undetermined)
  t=    0.409  vocabulary_samples -> DISCREPANCY_RESOLVED (discrepancy_id=01JSAMPLE, resolution=profile_excluded, resolving_ref=01JSAMPLE)
  t=    0.409  vocabulary_samples -> DISCREPANCY_RESOLVED (discrepancy_id=01JSAMPLE, resolution=profile_excluded, resolving_ref=01JSAMPLE)
  t=    0.410  vocabulary_samples -> CANDIDATE_GENERATED (attempt=9007199254740991, candidate_id=01JSAMPLE, candidate_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., contract_id=windows.sample.contract, contract_version=0.9.
  t=    0.410  vocabulary_samples -> CANDIDATE_GENERATED (attempt=9007199254740991, candidate_id=01JSAMPLE, candidate_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., contract_id=windows.sample.contract, contract_version=0.9.
  t=    0.410  vocabulary_samples -> CANDIDATE_ACCEPTED (candidate_id=01JSAMPLE, comparison_ids=['01JSAMPLE', '01JSAMPLE'], contract_id=windows.sample.contract, contract_version=0.9.1)
  t=    0.410  vocabulary_samples -> CANDIDATE_ACCEPTED (candidate_id=01JSAMPLE, comparison_ids=['01JSAMPLE', '01JSAMPLE'], contract_id=windows.sample.contract, contract_version=0.9.1)
  t=    0.410  vocabulary_samples -> CANDIDATE_REJECTED (candidate_id=01JSAMPLE, contract_id=windows.sample.contract, contract_version=0.9.1, discrepancy_id=01JSAMPLE, reason=a "quoted" line\nthen é)
  t=    0.410  vocabulary_samples -> CANDIDATE_REJECTED (candidate_id=01JSAMPLE, contract_id=windows.sample.contract, contract_version=0.9.1, discrepancy_id=None, reason=a "quoted" line\nthen é)
  t=    0.410  vocabulary_samples -> ACCEPTANCE_CONDITION_ALTERED (altered=evidence, candidate_id=01JSAMPLE, expected_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., observed_sha256=sha256:0123456789abcdef0123456789abcdef012
  t=    0.410  vocabulary_samples -> ACCEPTANCE_CONDITION_ALTERED (altered=evidence, candidate_id=01JSAMPLE, expected_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., observed_sha256=sha256:0123456789abcdef0123456789abcdef012
  t=    0.410  vocabulary_samples -> CODE_TARGET_DISCOVERED (discovery=generated_code, guest_address=9007199254740991, image_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., stage=execution)
  t=    0.410  vocabulary_samples -> CODE_TARGET_DISCOVERED (discovery=generated_code, guest_address=9007199254740991, image_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., stage=execution)
  t=    0.410  vocabulary_samples -> TRANSLATION_PUBLISHED (artifact_id=01JSAMPLE, code_version=9007199254740991, contract_version=0.9.1, guest_range_end=9007199254740991, guest_range_start=9007199254740991, image_sha256=sha256:0123456
  t=    0.410  vocabulary_samples -> TRANSLATION_PUBLISHED (artifact_id=01JSAMPLE, code_version=9007199254740991, contract_version=0.9.1, guest_range_end=9007199254740991, guest_range_start=9007199254740991, image_sha256=sha256:0123456
  t=    0.410  vocabulary_samples -> ARTIFACT_ACCEPTED (artifact_id=01JSAMPLE, comparison_ids=['01JSAMPLE', '01JSAMPLE'])
  t=    0.410  vocabulary_samples -> ARTIFACT_ACCEPTED (artifact_id=01JSAMPLE, comparison_ids=['01JSAMPLE', '01JSAMPLE'])
  t=    0.410  vocabulary_samples -> CAPABILITY_PROBED (capability=guest_fp_to_int_x86, detail=a "quoted" line\nthen é, execution_id=01JSAMPLE, host_id=01JSAMPLE, outcome=not_established)
  t=    0.410  vocabulary_samples -> CAPABILITY_PROBED (capability=guest_fp_to_int_x86, detail=a "quoted" line\nthen é, execution_id=None, host_id=01JSAMPLE, outcome=not_established)
  t=    0.411  vocabulary_samples -> BACKEND_EVALUATED (application_profile_id=01JSAMPLE, backend=managed_vm, conformance_fail_count=9007199254740991, conformance_pass_count=9007199254740991, destination_profile_id=01JSAMPLE, evaluatio
  t=    0.411  vocabulary_samples -> BACKEND_EVALUATED (application_profile_id=01JSAMPLE, backend=managed_vm, conformance_fail_count=9007199254740991, conformance_pass_count=9007199254740991, destination_profile_id=01JSAMPLE, evaluatio
  t=    0.411  vocabulary_samples -> BACKEND_SELECTED (application_profile_id=01JSAMPLE, backend=managed_vm, decision_ref=a "quoted" line\nthen é, destination_profile_id=01JSAMPLE, evaluation_ids=['01JSAMPLE', '01JSAMPLE'])
  t=    0.411  vocabulary_samples -> BACKEND_SELECTED (application_profile_id=01JSAMPLE, backend=managed_vm, decision_ref=a "quoted" line\nthen é, destination_profile_id=01JSAMPLE, evaluation_ids=['01JSAMPLE', '01JSAMPLE'])
  t=    0.411  vocabulary_samples -> MEASUREMENT_RECORDED (application_profile_id=01JSAMPLE, artifact_state=warm, backend=managed_vm, destination_profile_id=01JSAMPLE, measurement_id=01JSAMPLE, metric=wfe_exits, phase=update, unit=a "q
  t=    0.411  vocabulary_samples -> MEASUREMENT_RECORDED (application_profile_id=01JSAMPLE, artifact_state=warm, backend=managed_vm, destination_profile_id=01JSAMPLE, measurement_id=01JSAMPLE, metric=wfe_exits, phase=update, unit=a "q
  t=    0.411  vocabulary_samples -> DISTRIBUTION_RESOLVED (app_id=9007199254740991, build_id=9007199254740991, depot_ids=[9007199254740991, 9007199254740991], platform=a "quoted" line\nthen é)
  t=    0.412  vocabulary_samples -> DISTRIBUTION_RESOLVED (app_id=9007199254740991, build_id=9007199254740991, depot_ids=[9007199254740991, 9007199254740991], platform=a "quoted" line\nthen é)
  t=    0.412  vocabulary_samples -> DISTRIBUTION_ACQUIRED (app_id=9007199254740991, build_id=9007199254740991, bytes=9007199254740991, content_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., tool=steamcmd)
  t=    0.412  vocabulary_samples -> DISTRIBUTION_ACQUIRED (app_id=9007199254740991, build_id=9007199254740991, bytes=9007199254740991, content_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., tool=steamcmd)
  t=    0.412  vocabulary_samples -> ACQUISITION_FAILED (app_id=9007199254740991, diagnostic=a "quoted" line\nthen é, reason=tool_error)
  t=    0.412  vocabulary_samples -> ACQUISITION_FAILED (app_id=9007199254740991, diagnostic=a "quoted" line\nthen é, reason=tool_error)
  t=    0.412  vocabulary_samples -> SERVICE_UNIMPLEMENTED (execution_id=01JSAMPLE, guest_address=9007199254740991, instructions_executed=9007199254740991, service=a "quoted" line\nthen é)
  t=    0.412  vocabulary_samples -> SERVICE_UNIMPLEMENTED (execution_id=01JSAMPLE, guest_address=9007199254740991, instructions_executed=9007199254740991, service=a "quoted" line\nthen é)
  t=    0.412  vocabulary_samples -> MEASUREMENT_RECORDED (application_profile_id=01JSAMPLE, artifact_state=warm, backend=managed_vm, destination_profile_id=01JSAMPLE, measurement_id=01JSAMPLE, metric=install_seconds, phase=update, uni
  ... MEASUREMENT_RECORDED 27 more
```

### signal_trace_tags

- `ACCEPTANCE_CONDITION_ALTERED`
- `ACQUISITION_FAILED`
- `ARTIFACT_ACCEPTED`
- `BACKEND_EVALUATED`
- `BACKEND_SELECTED`
- `CANDIDATE_ACCEPTED`
- `CANDIDATE_GENERATED`
- `CANDIDATE_REJECTED`
- `CAPABILITY_PROBED`
- `CODE_TARGET_DISCOVERED`
- `COMPARISON_COMPLETED`
- `CONTRACT_ACCEPTED`
- `CONTRACT_PROPOSED`
- `CONTRACT_REVISED`
- `DEPENDENTS_INVALIDATED`
- `DISCREPANCY_REPORTED`
- `DISCREPANCY_RESOLVED`
- `DISCREPANCY_ROUTED`
- `DISTRIBUTION_ACQUIRED`
- `DISTRIBUTION_RESOLVED`
- `EVIDENCE_RECORDED`
- `EXECUTION_COMPLETED`
- `EXECUTION_FAILED`
- `EXECUTION_STARTED`
- `MEASUREMENT_RECORDED`
- `PROBE_AUTHORED`
- `REDUCTION_COMPLETED`
- `SERVICE_UNIMPLEMENTED`
- `SESSION_COMPLETED`
- `SESSION_INIT`
- `TRACE_INGESTED`
- `TRACE_REJECTED`
- `TRANSLATION_PUBLISHED`

## 2. Expected

> - The vocabulary samples run (`harness/vocabulary-samples.toml`, Sprint 126) fires MEASUREMENT_RECORDED with each of
>   the seven new metric values through its Rust type, on a record that replays; SESSION_INIT names `vocab_version`
>   0.11.

## 3. Delta

### emits_check

- `MEASUREMENT_RECORDED`: fired 30 times
- `SESSION_INIT`: fired 2 times
- fired, not named in Emits: `ACCEPTANCE_CONDITION_ALTERED` (2), `ACQUISITION_FAILED` (2), `ARTIFACT_ACCEPTED` (2), `BACKEND_EVALUATED` (2), `BACKEND_SELECTED` (2), `CANDIDATE_ACCEPTED` (2), `CANDIDATE_GENERATED` (2), `CANDIDATE_REJECTED` (2), `CAPABILITY_PROBED` (2), `CODE_TARGET_DISCOVERED` (2), `COMPARISON_COMPLETED` (2), `CONTRACT_ACCEPTED` (2), `CONTRACT_PROPOSED` (2), `CONTRACT_REVISED` (2), `DEPENDENTS_INVALIDATED` (2), `DISCREPANCY_REPORTED` (2), `DISCREPANCY_RESOLVED` (2), `DISCREPANCY_ROUTED` (2), `DISTRIBUTION_ACQUIRED` (2), `DISTRIBUTION_RESOLVED` (2), `EVIDENCE_RECORDED` (2), `EXECUTION_COMPLETED` (2), `EXECUTION_FAILED` (2), `EXECUTION_STARTED` (2), `PROBE_AUTHORED` (2), `REDUCTION_COMPLETED` (2), `SERVICE_UNIMPLEMENTED` (2), `SESSION_COMPLETED` (2), `TRACE_INGESTED` (2), `TRACE_REJECTED` (2), `TRANSLATION_PUBLISHED` (2)

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 162: its Rubber Duck Pass and close.
