# SIGNAL_REPORT — Sprint 158 — The skeleton's gate: `sleeper`'s output and record from the Rust host equal the stored ones

Made by `signal_report 158` from 1 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 158.

## 1. Observed

### scope_confirmation

> Link 6 of `docs/design-host-compile-path-2026-10-24.md` ("the skeleton's gate: the guest's output and record equal the
> stored ones"), and step 3's gate in `docs/rust-port-plan-2026-10-24.md` ("It loads the guest, serves its exits, writes
> its record through substrate, and ends with the guest's stored output"). Since Sprint 157 the Rust host runs
> `tests/data/timed_waits/sleeper.exe` to ExitProcess(0) with the Python host's output, but only inside a test, and it
> writes no record.

### work_performed

- 3f1eada33 Sprint 158: the skeleton's gate; w2m-vm runs sleeper under substrate, and its record and output equal the frozen Python host's
- 4af48ad94 Sprint 158: its card, before its code (the skeleton's gate)

### signal_trace

```
w2m-vm-sleeper-1793149355000000000: SESSION_INIT 1, CODE_TARGET_DISCOVERED 19, TRANSLATION_PUBLISHED 19, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    0.353  w2m_vm -> SESSION_INIT (contract_version=0.11.0, harness_version=0.1.0, profile=w2m-vm-skeleton, purpose=compilation, vocab_version=0.10)
  t=    0.353  w2m_vm -> CODE_TARGET_DISCOVERED (discovery=static, guest_address=5368713254, image_sha256=sha256:6863d5ecd3a14fb5df0f1b194135fecd9e59a153..., stage=execution)
  t=    0.353  w2m_vm -> TRANSLATION_PUBLISHED (artifact_id=vm-code-cache:7f0000062920, code_version=0, contract_version=0.11.0, guest_range_end=5368713308, guest_range_start=5368713254, image_sha256=sha256:6863d5ecd3a14fb5df0f1b194135
  t=    0.353  w2m_vm -> CODE_TARGET_DISCOVERED (discovery=runtime_transfer, guest_address=5368713308, image_sha256=sha256:6863d5ecd3a14fb5df0f1b194135fecd9e59a153..., stage=execution)
  t=    0.353  w2m_vm -> TRANSLATION_PUBLISHED (artifact_id=vm-code-cache:7f0000062b14, code_version=0, contract_version=0.11.0, guest_range_end=5368713311, guest_range_start=5368713308, image_sha256=sha256:6863d5ecd3a14fb5df0f1b194135
  t=    0.353  w2m_vm -> CODE_TARGET_DISCOVERED (discovery=runtime_transfer, guest_address=5368713311, image_sha256=sha256:6863d5ecd3a14fb5df0f1b194135fecd9e59a153..., stage=execution)
  t=    0.353  w2m_vm -> TRANSLATION_PUBLISHED (artifact_id=vm-code-cache:7f0000062b3c, code_version=0, contract_version=0.11.0, guest_range_end=5368713320, guest_range_start=5368713311, image_sha256=sha256:6863d5ecd3a14fb5df0f1b194135
  t=    0.354  w2m_vm -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=0.067, outcome=completed)
  ... CODE_TARGET_DISCOVERED 16 more
  ... TRANSLATION_PUBLISHED 16 more
```

### signal_trace_tags

- `CODE_TARGET_DISCOVERED`
- `SESSION_COMPLETED`
- `SESSION_INIT`
- `TRANSLATION_PUBLISHED`

## 2. Expected

> SESSION_INIT, CODE_TARGET_DISCOVERED, TRANSLATION_PUBLISHED, SESSION_COMPLETED: the skeleton's record, from `w2m-vm`,
> on a substrate record under `store/runs/`. All four tags are in vocabulary 0.10 and fired by the Python host's
> `record_translations`.

## 3. Delta

### emits_check

- `CODE_TARGET_DISCOVERED`: fired 19 times
- `SESSION_COMPLETED`: fired 1 time
- `SESSION_INIT`: fired 1 time
- `TRANSLATION_PUBLISHED`: fired 19 times

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 158: its Rubber Duck Pass and close.
