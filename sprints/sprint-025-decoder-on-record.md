# Sprint 025 — decoder differential on the signal plane; judge sessions

```yaml
---
id: 025
status: closed
phase: 3
pass_kind: functional
---
```

## scope

`scripts/generated_differential.py --record` writes one substrate run per invocation: SESSION_INIT (purpose conformance), one COMPARISON_COMPLETED (relation exact, contract `x86.decode` at the spec version, reference XED), one DISCREPANCY_REPORTED and one DISCREPANCY_ROUTED per disagreement class, SESSION_COMPLETED. Routing is deterministic from the outcome class (review §3.2, recommendation 4): `length_differs` → implementation_repair (the spec row and XED agree on the encoding's fields, the decoder routine is wrong); `ours_accepts`, `ours_gap`, `iclass_differs`, operand differences → contract_revision (the spec lacks the row or rule XED applies). Each class's example bytes go into the content store (`store/programs/<sha256>.bin`) so `minimized_program_sha256` resolves. `scripts/judge_traces.py` gains the SESSION_INIT and SESSION_COMPLETED it never emitted (vocabulary invariants T1, T2).

## prerequisites

- 024

## context_files

- `sdd-kit-3/AGENTS.md`, `signals/0.4.json` (T1, T2; COMPARISON_COMPLETED, DISCREPANCY_REPORTED, DISCREPANCY_ROUTED), `BLACKBOARD.md`, `WORKING_AGREEMENT.md`
- `docs/review-2026-10-08-project.md` §3.2, §8 item 4
- `scripts/generated_differential.py`, `scripts/judge_traces.py`, `scripts/litmus_mp.py` (session pattern)

## signal contract

### Emits

- `SESSION_INIT` (vocab_version 0.4, profile, contract_version, harness_version, purpose=conformance)
- `COMPARISON_COMPLETED` (relation=exact, contract_id=x86.decode, verdict=discrepancy when any class exists)
- `DISCREPANCY_REPORTED` (mismatch_class=exact, minimized_program_sha256 of the class example) — one per class
- `DISCREPANCY_ROUTED` (repair_path per the rule above, reference_conforms, grounds) — one per class
- `SESSION_COMPLETED` (outcome, discrepancy_count = number of classes)
- judge path: `SESSION_INIT` first, `SESSION_COMPLETED` last, around the existing tags

### Invariants

- T1/T2: SESSION_INIT is the first and SESSION_COMPLETED the last application event of both runs.
- Count of DISCREPANCY_REPORTED = count of DISCREPANCY_ROUTED = count of classes in the JSON worklist. (Card said "classes the text report prints"; the text prints the top 12 per outcome only. Corrected at close.)

## artifact contract

### Files modified

- `scripts/generated_differential.py`, `scripts/judge_traces.py`
- found during the sprint: `scripts/litmus_mp.py`, `ordering_cost.py`, `page_geometry.py`, `probe_capabilities.py` (run root named by `time.time_ns()`; two judge runs in one second wrote into one directory and substrate refused the record with RecordGapError). Hard rule 6 exception: one-token change per file, same defect.

### Files created

- `store/programs/*.bin` (class examples), `signal-reports/sprint-025-report.md`

### Content assertions

- Every `minimized_program_sha256` in the record names a file under `store/programs/`.

### Command exit codes

- `uv run pytest` → 0; ruff, mypy → 0

## observation contract

### Expected runtime signals

- generated run: SESSION_INIT, COMPARISON_COMPLETED verdict=discrepancy, N × (DISCREPANCY_REPORTED, DISCREPANCY_ROUTED) with N equal to the printed class count; no `implementation_repair` (length_differs is 0); SESSION_COMPLETED outcome=completed_with_discrepancies.
- judge run `store:37738035459`: first application event SESSION_INIT, last SESSION_COMPLETED outcome=completed, discrepancy_count 0.

Read back from the record, not from memory (Sprint 024 report correction).

## done criteria

The decoder's disagreements exist as typed, routed discrepancies on a substrate record, and both harness runs open and close their sessions.

## self-review checklist

- [x] Scope is concrete and bounded.
- [x] `Emits` references only tags in 0.4.
- [x] Artifact contract gradable.
- [x] Observation contract declared before running.
- [ ] ≤2 source files: exception recorded above (run-root defect).

## observed

- `runs/gendiff-1791496598044137000`: SESSION_INIT; COMPARISON_COMPLETED verdict=discrepancy; DISCREPANCY_REPORTED ×300, DISCREPANCY_ROUTED ×300 (all contract_revision; length_differs 0, so no implementation_repair); SESSION_COMPLETED completed_with_discrepancies, discrepancy_count 300. 300 programs under `store/programs/`, none missing.
- Judge runs: `store:37738035459` (10 executions) and the new runner run 37850017162 (2 executions, evt24): first application event SESSION_INIT, last SESSION_COMPLETED completed, discrepancy_count 0. The new executions carry observed `exit_code` 0 and `elapsed_seconds` 2.034 and 2.013 and no `unobserved`; the workflow change of Sprint 024 works on the runner. Waiter a released in both (12 of 12 overall).
