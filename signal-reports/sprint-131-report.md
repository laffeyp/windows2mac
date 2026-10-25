# SIGNAL_REPORT — Sprint 131 — Vocabulary 0.10: the 24 tags no code fires, deprecated

Made by `signal_report 131` from 2 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 131.

## 1. Observed

### scope_confirmation

> Fix (e) of the direction of 2026-10-24 (BLACKBOARD Decision 2026-10-24 user), step 2 of
> `docs/rust-port-plan-2026-10-24.md`. Vocabulary 0.10 deprecates, by one typed proposal (TAG_DEPRECATION_PROPOSED,
> `signals/proposals.json` P21), the 24 tags of 0.9 that no code fires: 21 named only in generated code
> (`src/w2m/events.py`) and GUEST_CLOCK_READ, GUEST_MEMORY_OBSERVED and GUEST_THREAD_EXITED, named only in tests. Their
> 1,080 occurrences each under `store/runs/` are the schema copied into each record's baseline and Sprint 126's samples
> run, not firings. A deprecated tag keeps its entry, marked with the date, the proposal and the reason (TECHNIQUES.md
> #45): records that carry it still replay under their own baselines. The Rust generator carries it no further: no Rust
> type, no entry in `harness/schemas/harness-events.json`, and a guest line with its tag is refused by name. The frozen
> Python speaks 0.9 (`src/w2m/__init__.py` pins `VOCAB_VERSION = "0.9"`; `gen_events.py`, `check_records.py` and
> `tests/test_events.py` read `signals/current.json`), so `current.json` keeps naming 0.9, the oracle's vocabulary,
> and a second pointer, `signals/rust-current.json`, names the version Rust code speaks: 0.10. No Python file and no
> Python gate changes; at plan step 4, when the Python host moves under `oracle/`, the two pointers become one.

### work_performed

- cbb810847 Sprint 131: vocabulary 0.10, the 24 tags no code fires deprecated

### signal_trace

```
session-check-20261025T004412: SESSION_INIT 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    0.004  session_events -> SESSION_INIT (contract_version=0.0, harness_version=0.1.0, profile=w2m-session-check, purpose=conformance, vocab_version=0.10)
  t=    0.004  session_events -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=6.834e-06, outcome=completed)
vocabulary-samples-20261025T004412: SESSION_INIT 2, SESSION_COMPLETED 2, EVIDENCE_RECORDED 2, CONTRACT_PROPOSED 2, CONTRACT_ACCEPTED 2, CONTRACT_REVISED 2, DEPENDENTS_INVALIDATED 2, PROBE_AUTHORED 2, EXECUTION_STARTED 2, EXECUTION_COMPLETED 2, EXECUTION_FAILED 2, TRACE_INGESTED 2, TRACE_REJECTED 2, COMPARISON_COMPLETED 2, REDUCTION_COMPLETED 2, DISCREPANCY_REPORTED 2, DISCREPANCY_ROUTED 2, DISCREPANCY_RESOLVED 2, CANDIDATE_GENERATED 2, CANDIDATE_ACCEPTED 2, CANDIDATE_REJECTED 2, ACCEPTANCE_CONDITION_ALTERED 2, CODE_TARGET_DISCOVERED 2, TRANSLATION_PUBLISHED 2, ARTIFACT_ACCEPTED 2, CAPABILITY_PROBED 2, BACKEND_EVALUATED 2, BACKEND_SELECTED 2, MEASUREMENT_RECORDED 2, DISTRIBUTION_RESOLVED 2, DISTRIBUTION_ACQUIRED 2, ACQUISITION_FAILED 2, SERVICE_UNIMPLEMENTED 2; producers failed 0; not finalised 0
  t=    0.005  vocabulary_samples -> SESSION_INIT (contract_version=0.9.1, harness_version=0.9.1, profile=01JSAMPLE, purpose=product, vocab_version=0.9.1)
  t=    0.005  vocabulary_samples -> SESSION_INIT (contract_version=0.9.1, harness_version=0.9.1, profile=01JSAMPLE, purpose=product, vocab_version=0.9.1)
  t=    0.005  vocabulary_samples -> SESSION_COMPLETED (accepted_count=9007199254740991, discrepancy_count=9007199254740991, elapsed_seconds=0.5, outcome=aborted)
  t=    0.005  vocabulary_samples -> SESSION_COMPLETED (accepted_count=9007199254740991, discrepancy_count=9007199254740991, elapsed_seconds=0.5, outcome=aborted)
  t=    0.005  vocabulary_samples -> EVIDENCE_RECORDED (applicability=a "quoted" line\nthen é, contract_id=windows.sample.contract, distinguishing_check=a "quoted" line\nthen é, evidence_category=hypothesis, evidence_id=01JSAMPLE, sour
  t=    0.005  vocabulary_samples -> EVIDENCE_RECORDED (applicability=a "quoted" line\nthen é, contract_id=windows.sample.contract, distinguishing_check=None, evidence_category=hypothesis, evidence_id=01JSAMPLE, source_ref=a "quoted" l
  t=    0.005  vocabulary_samples -> CONTRACT_PROPOSED (contract_id=windows.sample.contract, evidence_ids=['01JSAMPLE', '01JSAMPLE'], proposer=generator, version=0.9.1)
  t=    0.005  vocabulary_samples -> CONTRACT_PROPOSED (contract_id=windows.sample.contract, evidence_ids=['01JSAMPLE', '01JSAMPLE'], proposer=generator, version=0.9.1)
  t=    0.005  vocabulary_samples -> CONTRACT_ACCEPTED (applicability=a "quoted" line\nthen é, contract_id=windows.sample.contract, evidence_ids=['01JSAMPLE', '01JSAMPLE'], open_question_count=9007199254740991, version=0.9.1)
  t=    0.005  vocabulary_samples -> CONTRACT_ACCEPTED (applicability=a "quoted" line\nthen é, contract_id=windows.sample.contract, evidence_ids=['01JSAMPLE', '01JSAMPLE'], open_question_count=9007199254740991, version=0.9.1)
  t=    0.005  vocabulary_samples -> CONTRACT_REVISED (contract_id=windows.sample.contract, evidence_ids=['01JSAMPLE', '01JSAMPLE'], from_version=0.9.1, rationale_ref=a "quoted" line\nthen é, to_version=0.9.1, triggering_discrepancy_id
  t=    0.005  vocabulary_samples -> CONTRACT_REVISED (contract_id=windows.sample.contract, evidence_ids=['01JSAMPLE', '01JSAMPLE'], from_version=0.9.1, rationale_ref=a "quoted" line\nthen é, to_version=0.9.1, triggering_discrepancy_id
  t=    0.006  vocabulary_samples -> DEPENDENTS_INVALIDATED (artifact_count=9007199254740991, candidate_count=9007199254740991, contract_id=windows.sample.contract, evidence_count=9007199254740991, from_version=0.9.1, to_version=0.9.1)
  t=    0.006  vocabulary_samples -> DEPENDENTS_INVALIDATED (artifact_count=9007199254740991, candidate_count=9007199254740991, contract_id=windows.sample.contract, evidence_count=9007199254740991, from_version=0.9.1, to_version=0.9.1)
  t=    0.006  vocabulary_samples -> PROBE_AUTHORED (contract_ids=['windows.sample.contract', 'windows.sample.contract'], origin=reduced, parent_probe_id=01JSAMPLE, probe_id=01JSAMPLE, probe_sha256=sha256:0123456789abcdef0123456789abcd
  t=    0.006  vocabulary_samples -> PROBE_AUTHORED (contract_ids=['windows.sample.contract', 'windows.sample.contract'], origin=reduced, parent_probe_id=None, probe_id=01JSAMPLE, probe_sha256=sha256:0123456789abcdef0123456789abcdef012
  t=    0.006  vocabulary_samples -> EXECUTION_STARTED (execution_id=01JSAMPLE, initial_state_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., injected_inputs_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., kill_gr
  t=    0.006  vocabulary_samples -> EXECUTION_STARTED (execution_id=01JSAMPLE, initial_state_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., injected_inputs_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., kill_gr
  t=    0.006  vocabulary_samples -> EXECUTION_COMPLETED (elapsed_seconds=0.5, execution_id=01JSAMPLE, exit_code=-9007199254740991, exit_status=killed, guest_line_count=9007199254740991, trace_sha256=sha256:0123456789abcdef0123456789ab
  t=    0.006  vocabulary_samples -> EXECUTION_COMPLETED (elapsed_seconds=None, execution_id=01JSAMPLE, exit_code=None, exit_status=killed, guest_line_count=9007199254740991, trace_sha256=sha256:0123456789abcdef0123456789abcdef01234567
  t=    0.006  vocabulary_samples -> EXECUTION_FAILED (diagnostic=a "quoted" line\nthen é, execution_id=01JSAMPLE, failure=harness_error)
  t=    0.006  vocabulary_samples -> EXECUTION_FAILED (diagnostic=a "quoted" line\nthen é, execution_id=01JSAMPLE, failure=harness_error)
  t=    0.006  vocabulary_samples -> TRACE_INGESTED (execution_id=01JSAMPLE, line_count=9007199254740991, thread_count=9007199254740991, trace_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., vocab_version=0.9.1)
  t=    0.006  vocabulary_samples -> TRACE_INGESTED (execution_id=01JSAMPLE, line_count=9007199254740991, thread_count=9007199254740991, trace_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., vocab_version=0.9.1)
  t=    0.006  vocabulary_samples -> TRACE_REJECTED (execution_id=01JSAMPLE, first_invalid_line=9007199254740991, reason=a "quoted" line\nthen é, trace_sha256=sha256:0123456789abcdef0123456789abcdef01234567...)
  t=    0.006  vocabulary_samples -> TRACE_REJECTED (execution_id=01JSAMPLE, first_invalid_line=9007199254740991, reason=a "quoted" line\nthen é, trace_sha256=sha256:0123456789abcdef0123456789abcdef01234567...)
  t=    0.006  vocabulary_samples -> COMPARISON_COMPLETED (comparison_id=01JSAMPLE, contract_id=windows.sample.contract, contract_version=0.9.1, first_divergence=a "quoted" line\nthen é, reference_ref=01JSAMPLE, relation=progress_oblig
  t=    0.006  vocabulary_samples -> COMPARISON_COMPLETED (comparison_id=01JSAMPLE, contract_id=windows.sample.contract, contract_version=0.9.1, first_divergence=None, reference_ref=01JSAMPLE, relation=progress_obligation, rule_ref=Non
  t=    0.006  vocabulary_samples -> REDUCTION_COMPLETED (comparison_id=01JSAMPLE, minimized_program_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., preserved_mismatch=True, reduction_id=01JSAMPLE, source_program_sha256=sha2
  t=    0.006  vocabulary_samples -> REDUCTION_COMPLETED (comparison_id=01JSAMPLE, minimized_program_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., preserved_mismatch=True, reduction_id=01JSAMPLE, source_program_sha256=sha2
  t=    0.006  vocabulary_samples -> DISCREPANCY_REPORTED (comparison_id=01JSAMPLE, contract_id=windows.sample.contract, contract_version=0.9.1, destination_profile_id=01JSAMPLE, discrepancy_id=01JSAMPLE, first_divergence=a "quoted" li
  t=    0.006  vocabulary_samples -> DISCREPANCY_REPORTED (comparison_id=01JSAMPLE, contract_id=windows.sample.contract, contract_version=0.9.1, destination_profile_id=01JSAMPLE, discrepancy_id=01JSAMPLE, first_divergence=a "quoted" li
  t=    0.006  vocabulary_samples -> DISCREPANCY_ROUTED (discrepancy_id=01JSAMPLE, grounds=a "quoted" line\nthen é, reference_conforms=True, repair_path=undetermined)
  t=    0.006  vocabulary_samples -> DISCREPANCY_ROUTED (discrepancy_id=01JSAMPLE, grounds=a "quoted" line\nthen é, reference_conforms=True, repair_path=undetermined)
  t=    0.006  vocabulary_samples -> DISCREPANCY_RESOLVED (discrepancy_id=01JSAMPLE, resolution=profile_excluded, resolving_ref=01JSAMPLE)
  t=    0.006  vocabulary_samples -> DISCREPANCY_RESOLVED (discrepancy_id=01JSAMPLE, resolution=profile_excluded, resolving_ref=01JSAMPLE)
  t=    0.006  vocabulary_samples -> CANDIDATE_GENERATED (attempt=9007199254740991, candidate_id=01JSAMPLE, candidate_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., contract_id=windows.sample.contract, contract_version=0.9.
  t=    0.006  vocabulary_samples -> CANDIDATE_GENERATED (attempt=9007199254740991, candidate_id=01JSAMPLE, candidate_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., contract_id=windows.sample.contract, contract_version=0.9.
  t=    0.006  vocabulary_samples -> CANDIDATE_ACCEPTED (candidate_id=01JSAMPLE, comparison_ids=['01JSAMPLE', '01JSAMPLE'], contract_id=windows.sample.contract, contract_version=0.9.1)
  t=    0.006  vocabulary_samples -> CANDIDATE_ACCEPTED (candidate_id=01JSAMPLE, comparison_ids=['01JSAMPLE', '01JSAMPLE'], contract_id=windows.sample.contract, contract_version=0.9.1)
  t=    0.006  vocabulary_samples -> CANDIDATE_REJECTED (candidate_id=01JSAMPLE, contract_id=windows.sample.contract, contract_version=0.9.1, discrepancy_id=01JSAMPLE, reason=a "quoted" line\nthen é)
  t=    0.006  vocabulary_samples -> CANDIDATE_REJECTED (candidate_id=01JSAMPLE, contract_id=windows.sample.contract, contract_version=0.9.1, discrepancy_id=None, reason=a "quoted" line\nthen é)
  t=    0.006  vocabulary_samples -> ACCEPTANCE_CONDITION_ALTERED (altered=evidence, candidate_id=01JSAMPLE, expected_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., observed_sha256=sha256:0123456789abcdef0123456789abcdef012
  t=    0.006  vocabulary_samples -> ACCEPTANCE_CONDITION_ALTERED (altered=evidence, candidate_id=01JSAMPLE, expected_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., observed_sha256=sha256:0123456789abcdef0123456789abcdef012
  t=    0.006  vocabulary_samples -> CODE_TARGET_DISCOVERED (discovery=generated_code, guest_address=9007199254740991, image_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., stage=execution)
  t=    0.007  vocabulary_samples -> CODE_TARGET_DISCOVERED (discovery=generated_code, guest_address=9007199254740991, image_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., stage=execution)
  t=    0.007  vocabulary_samples -> TRANSLATION_PUBLISHED (artifact_id=01JSAMPLE, code_version=9007199254740991, contract_version=0.9.1, guest_range_end=9007199254740991, guest_range_start=9007199254740991, image_sha256=sha256:0123456
  t=    0.007  vocabulary_samples -> TRANSLATION_PUBLISHED (artifact_id=01JSAMPLE, code_version=9007199254740991, contract_version=0.9.1, guest_range_end=9007199254740991, guest_range_start=9007199254740991, image_sha256=sha256:0123456
  t=    0.007  vocabulary_samples -> ARTIFACT_ACCEPTED (artifact_id=01JSAMPLE, comparison_ids=['01JSAMPLE', '01JSAMPLE'])
  t=    0.007  vocabulary_samples -> ARTIFACT_ACCEPTED (artifact_id=01JSAMPLE, comparison_ids=['01JSAMPLE', '01JSAMPLE'])
  t=    0.007  vocabulary_samples -> CAPABILITY_PROBED (capability=guest_fp_to_int_x86, detail=a "quoted" line\nthen é, execution_id=01JSAMPLE, host_id=01JSAMPLE, outcome=not_established)
  t=    0.007  vocabulary_samples -> CAPABILITY_PROBED (capability=guest_fp_to_int_x86, detail=a "quoted" line\nthen é, execution_id=None, host_id=01JSAMPLE, outcome=not_established)
  t=    0.007  vocabulary_samples -> BACKEND_EVALUATED (application_profile_id=01JSAMPLE, backend=managed_vm, conformance_fail_count=9007199254740991, conformance_pass_count=9007199254740991, destination_profile_id=01JSAMPLE, evaluatio
  t=    0.007  vocabulary_samples -> BACKEND_EVALUATED (application_profile_id=01JSAMPLE, backend=managed_vm, conformance_fail_count=9007199254740991, conformance_pass_count=9007199254740991, destination_profile_id=01JSAMPLE, evaluatio
  t=    0.007  vocabulary_samples -> BACKEND_SELECTED (application_profile_id=01JSAMPLE, backend=managed_vm, decision_ref=a "quoted" line\nthen é, destination_profile_id=01JSAMPLE, evaluation_ids=['01JSAMPLE', '01JSAMPLE'])
  t=    0.007  vocabulary_samples -> BACKEND_SELECTED (application_profile_id=01JSAMPLE, backend=managed_vm, decision_ref=a "quoted" line\nthen é, destination_profile_id=01JSAMPLE, evaluation_ids=['01JSAMPLE', '01JSAMPLE'])
  t=    0.007  vocabulary_samples -> MEASUREMENT_RECORDED (application_profile_id=01JSAMPLE, artifact_state=warm, backend=managed_vm, destination_profile_id=01JSAMPLE, measurement_id=01JSAMPLE, metric=load_average, phase=update, unit=a
  t=    0.007  vocabulary_samples -> MEASUREMENT_RECORDED (application_profile_id=01JSAMPLE, artifact_state=warm, backend=managed_vm, destination_profile_id=01JSAMPLE, measurement_id=01JSAMPLE, metric=load_average, phase=update, unit=a
  t=    0.007  vocabulary_samples -> DISTRIBUTION_RESOLVED (app_id=9007199254740991, build_id=9007199254740991, depot_ids=[9007199254740991, 9007199254740991], platform=a "quoted" line\nthen é)
  t=    0.007  vocabulary_samples -> DISTRIBUTION_RESOLVED (app_id=9007199254740991, build_id=9007199254740991, depot_ids=[9007199254740991, 9007199254740991], platform=a "quoted" line\nthen é)
  t=    0.007  vocabulary_samples -> DISTRIBUTION_ACQUIRED (app_id=9007199254740991, build_id=9007199254740991, bytes=9007199254740991, content_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., tool=steamcmd)
  t=    0.007  vocabulary_samples -> DISTRIBUTION_ACQUIRED (app_id=9007199254740991, build_id=9007199254740991, bytes=9007199254740991, content_sha256=sha256:0123456789abcdef0123456789abcdef01234567..., tool=steamcmd)
  t=    0.007  vocabulary_samples -> ACQUISITION_FAILED (app_id=9007199254740991, diagnostic=a "quoted" line\nthen é, reason=tool_error)
  t=    0.007  vocabulary_samples -> ACQUISITION_FAILED (app_id=9007199254740991, diagnostic=a "quoted" line\nthen é, reason=tool_error)
  t=    0.007  vocabulary_samples -> SERVICE_UNIMPLEMENTED (execution_id=01JSAMPLE, guest_address=9007199254740991, instructions_executed=9007199254740991, service=a "quoted" line\nthen é)
  t=    0.007  vocabulary_samples -> SERVICE_UNIMPLEMENTED (execution_id=01JSAMPLE, guest_address=9007199254740991, instructions_executed=9007199254740991, service=a "quoted" line\nthen é)
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

> The vocabulary samples run (Sprint 126) fires each harness kind 0.10 keeps, twice, through its Rust type; the 19
> deprecated harness kinds no longer appear on it. Its card lists the counts.

## 3. Delta

### emits_check

- fired, not named in Emits: `ACCEPTANCE_CONDITION_ALTERED` (2), `ACQUISITION_FAILED` (2), `ARTIFACT_ACCEPTED` (2), `BACKEND_EVALUATED` (2), `BACKEND_SELECTED` (2), `CANDIDATE_ACCEPTED` (2), `CANDIDATE_GENERATED` (2), `CANDIDATE_REJECTED` (2), `CAPABILITY_PROBED` (2), `CODE_TARGET_DISCOVERED` (2), `COMPARISON_COMPLETED` (2), `CONTRACT_ACCEPTED` (2), `CONTRACT_PROPOSED` (2), `CONTRACT_REVISED` (2), `DEPENDENTS_INVALIDATED` (2), `DISCREPANCY_REPORTED` (2), `DISCREPANCY_RESOLVED` (2), `DISCREPANCY_ROUTED` (2), `DISTRIBUTION_ACQUIRED` (2), `DISTRIBUTION_RESOLVED` (2), `EVIDENCE_RECORDED` (2), `EXECUTION_COMPLETED` (2), `EXECUTION_FAILED` (2), `EXECUTION_STARTED` (2), `MEASUREMENT_RECORDED` (2), `PROBE_AUTHORED` (2), `REDUCTION_COMPLETED` (2), `SERVICE_UNIMPLEMENTED` (2), `SESSION_COMPLETED` (3), `SESSION_INIT` (3), `TRACE_INGESTED` (2), `TRACE_REJECTED` (2), `TRANSLATION_PUBLISHED` (2)
- the card's Emits names no tag, and the sprint's records fired the ones above

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 131: its Rubber Duck Pass and close.
