# Sprint 024 — vocabulary 0.4 and evidence defects

```yaml
---
id: 024
status: closed
phase: 3
pass_kind: functional
---
```

## scope

Ratify vocabulary 0.4 and repair the evidence defects the project review confirmed (§4.5-4.8). 0.4: P11 (`BACKEND_EVALUATED.cost_objective_ref` → `objective_ref`), P12 (`capability_outcome` gains `not_established`), P13 (`EXECUTION_STARTED.timeout_seconds`, `kill_grace_seconds` and `EXECUTION_COMPLETED.exit_code`, `elapsed_seconds` become optional; both tags gain optional `unobserved`), and the `GUEST_CALL_RETURNED.args_sha256` note defines the canonical argument rendering. Code: `litmus_mp.py` records `present` only when the EnTSO-off rounds show a forbidden outcome; `probe_capabilities.py` records `native_process_tso` as `not_established`; `judge_traces.py` stops emitting placeholder values; the Windows workflow records each execution's exit code and elapsed time and pins actions by commit SHA. The Mac hardware run records cited by Built entries move into the committed store.

## prerequisites

- 023

## context_files

- `sdd-kit-3/AGENTS.md`, `sdd-kit-3/TECHNIQUES.md` #47, `signals/0.3.json`, `signals/proposals.json`, `BLACKBOARD.md`, `WORKING_AGREEMENT.md`
- `docs/review-2026-10-08-project.md` §4.5-4.8
- `scripts/litmus_mp.py`, `scripts/probe_capabilities.py`, `scripts/judge_traces.py`, `.github/workflows/windows-probe.yml`, `probes/event_auto_reset/event_auto_reset.c`
- v0.2 §8.1 ("an open destination-interface question rather than an asserted absence"), §14

## signal contract

### Emits

- `CAPABILITY_PROBED` (capability, host_id, outcome ∈ {present, absent, not_established, error}, detail) from `probe_capabilities.py` and `litmus_mp.py`
- `EXECUTION_STARTED` / `EXECUTION_COMPLETED` from `judge_traces.py`, with `unobserved` naming each omitted field
- existing judge-path tags unchanged: `SESSION_INIT`, `TRACE_INGESTED`, `COMPARISON_COMPLETED`, `SESSION_COMPLETED`

### Invariants

- No emitted payload value is a constant standing in for an unobserved quantity (TECHNIQUES.md #47).
- `guest_tsoen present` requires off > 0 and on = 0 in the same run (KIT_DIARY H4).
- 0.4 ingests every committed guest trace (guest tags unchanged).

## artifact contract

### Files created

- `signals/0.4.json`, `signals/0.4-rationale.md`, `store/runs/` (Mac hardware run records), `signal-reports/sprint-024-report.md`

### Files modified

- `signals/proposals.json`, `signals/current.json`, `src/w2m/events.py` (generated), `src/w2m/__init__.py`, `scripts/litmus_mp.py`, `scripts/probe_capabilities.py`, `scripts/judge_traces.py`, `.github/workflows/windows-probe.yml`, `BLACKBOARD.md`, `WORKING_AGREEMENT.md`

Hard rule 6 exception: one concept (honest evidence fields) across the vocabulary and its three emitters; recorded here.

### Content assertions

- `signals/0.4.json`: `capability_outcome` = [present, absent, error, not_established]; no field or text named `cost`.
- `judge_traces.py` source holds no `600.0`, `elapsed_seconds=0.0`, or `kill_grace_seconds=0.0`.
- `litmus_mp.py` outcome expression tests `off["forbidden_outcomes"] > 0`.

### Command exit codes

- `python3 scripts/check_vocab.py` → 0; `python3 scripts/gen_events.py --check` → 0; `uv run pytest` → 0; ruff, mypy → 0

## observation contract

### Expected runtime signals

- `probe_capabilities.py` run on this Mac: `CAPABILITY_PROBED capability=native_process_tso outcome=not_established`; the other capabilities keep their 2026-10-07 outcomes.
- `judge_traces.py` rerun: `EXECUTION_COMPLETED` payloads carry no `elapsed_seconds`; `unobserved` names it; `COMPARISON_COMPLETED verdict=conforms` for the 10 committed traces, as before.
- `litmus_mp.py` rerun: off > 0, on = 0, `guest_tsoen present`.

## done criteria

Every typed field the project emits is either observed or omitted with its reason, and the Mac evidence behind Phase 2 is committed.

## notes

The event probe's unwitnessed precondition (review §4.7: both waiters pending at `SetEvent`) needs a Windows-side witness and a runner rerun; it is recorded as Deferred with a candidate distinguishing check, not fixed here.

## self-review checklist

- [x] Scope is concrete and bounded.
- [x] `context_files` covers TECHNIQUES.md #47 and v0.2 §8.1.
- [x] `Emits` references only tags in 0.4 (after ratification in this sprint).
- [x] Artifact contract is gradable.
- [x] Observation contract present, declared before running.
- [ ] ≤2 files: exception recorded above.
