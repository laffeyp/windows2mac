# Sprint 031 — the candidate loop: a generated implementation judged against a contract

```yaml
---
id: 031
status: closed
phase: 3
pass_kind: functional
---
```

## scope

The loop of v0.2 §11.2–11.4, run once for real on one contract (`windows.thread.last_error.set_get@0.1.0`; Decision 2026-10-08 says why this one). In one substrate run:

1. `candidate_generator` builds a prompt from the contract: its summary, outcome rule and documented quotes, the candidate interface, and on a repair attempt the structured discrepancy. It asks a substrate Responder (Ollama `qwen2.5-coder:7b`, local) for `SetLastError` and `GetLastError`, stores the reply's code in `store/candidates/<sha256>.py`, and emits CANDIDATE_GENERATED.
2. `destination_runner` admits the candidate only through `src/w2m/candidate.py`. That module parses the code and allows a fixed set of syntax: two functions with a fixed signature, arithmetic, comparisons and conditionals, no calls, imports, loops, or attributes except `thread.last_error` and `thread.tid`. The code runs with empty builtins. The runner installs the candidate into kernel32 and runs the skeleton 3 times in the managed VM, emitting EXECUTION_STARTED and EXECUTION_COMPLETED, or EXECUTION_FAILED with `launch_failed` when the candidate is refused.
3. `trace_ingester` and `comparator` (unchanged) produce TRACE_INGESTED and COMPARISON_COMPLETED.
4. `acceptance_guard`, a separate operator, hashes the acceptance condition (the contract file, `compare.py`, `contract.py`) at session start and again after each candidate's runs. It emits ACCEPTANCE_CONDITION_ALTERED then CANDIDATE_REJECTED when the hashes differ, CANDIDATE_ACCEPTED citing the 3 comparison ids when all conform, and CANDIDATE_REJECTED otherwise.
5. `reducer`: a rejection with a discrepancy gives REDUCTION_COMPLETED (the skeleton is already minimal: steps 0, preserved), then DISCREPANCY_REPORTED, then DISCREPANCY_ROUTED. The route reads the committed Windows reference executions of the same probe: they conform, so the route is `implementation_repair`. Acceptance after a discrepancy gives DISCREPANCY_RESOLVED (`implementation_repaired`, the accepted candidate's id).
6. A route or a rejection starts the next attempt, up to 3; then `harness` emits SESSION_COMPLETED.

## prerequisites

- 030

## context_files

- v0.2 §7, §11.2, §11.3, §11.4; vocabulary 0.5 (CANDIDATE_*, DISCREPANCY_*, REDUCTION_COMPLETED, ACCEPTANCE_CONDITION_ALTERED; rules S3, S4-S8; operators)
- substrate `adapters/models.py` (Responder, OllamaResponder, call_responder_metered), `topologies/applications/best_of_n_verified.py` (the shape: generate, verify deterministically, feed failure back)
- `contracts/windows.thread.last_error.set_get/0.1.0.json`, `src/w2m/kernel32.py`, `src/w2m/vm_backend.py`, `scripts/judge_traces.py`

## signal contract

### Emits

- harness: SESSION_INIT, SESSION_COMPLETED
- candidate_generator: CANDIDATE_GENERATED (generator_ref names the model and topology; repair attempts carry parent_candidate_id and discrepancy_id)
- destination_runner: EXECUTION_STARTED (profile_kind managed_vm, timeout_seconds set), EXECUTION_COMPLETED or EXECUTION_FAILED
- trace_ingester, comparator: TRACE_INGESTED, COMPARISON_COMPLETED
- acceptance_guard: CANDIDATE_ACCEPTED, CANDIDATE_REJECTED, ACCEPTANCE_CONDITION_ALTERED
- reducer: REDUCTION_COMPLETED, DISCREPANCY_REPORTED, DISCREPANCY_ROUTED, DISCREPANCY_RESOLVED

### Invariants

- S3: CANDIDATE_ACCEPTED cites only earlier conforming comparisons of the same contract version.
- S6, S7: routing follows the report; resolution follows routing and matches its path.
- S8: ACCEPTANCE_CONDITION_ALTERED is followed by CANDIDATE_REJECTED, and no CANDIDATE_ACCEPTED, for that candidate.
- No generated code runs outside the admission check.

## artifact contract

### Files created

- `src/w2m/candidate.py`, `scripts/run_candidate_loop.py`, `tests/test_candidate_loop.py`, `store/candidates/*.py`

### Files modified

- `src/w2m/kernel32.py` (public `argument(i)`), `pyproject.toml` (substrate's `openai-compat` extra: httpx, which OllamaResponder imports), `probes/skeleton/candidates/loses_value.py` (the planted wrong first candidate)

Hard rule 6: three code files plus the stored candidates, the dependency line and one fixture.

### Command exit codes

- `scripts/gates.sh` → 0; CI success

## observation contract

### Expected runtime signals

- Tests with a scripted Responder (the model is not in CI): a wrong candidate then a right one gives CANDIDATE_REJECTED, REDUCTION_COMPLETED, DISCREPANCY_REPORTED, DISCREPANCY_ROUTED (implementation_repair), CANDIDATE_GENERATED (attempt 2, with discrepancy_id), CANDIDATE_ACCEPTED, DISCREPANCY_RESOLVED, in that order; a candidate with `import os` never runs (EXECUTION_FAILED launch_failed, CANDIDATE_REJECTED); a contract changed during the session gives ACCEPTANCE_CONDITION_ALTERED and no CANDIDATE_ACCEPTED.
- One real run with `qwen2.5-coder:7b`: the record shows each attempt's candidate hash, verdicts and the session outcome, whatever they are. A first-attempt acceptance is a valid result; so is exhaustion.

## done criteria

A model-generated SetLastError/GetLastError runs inside the managed VM, is judged by the unchanged comparator, and is accepted or rejected by a guard the generator cannot influence, with every step on the record.

## self-review checklist

- [x] Scope concrete. [x] Sources named. [x] Emits only 0.5 tags. [x] Observation declared before running. [x] ≤2 code files modified.

## observed

- First real run failed before any candidate: substrate's OllamaResponder imports httpx, which the project had not installed, and the failed generator left the session without SESSION_COMPLETED (T2) and the run "stuck quiescent". Fixed: the `openai-compat` extra; a `substrate.ProducerFailed` from any loop operator now starts the closer (outcome `aborted`); test added.
- Real run 1 (`qwen2.5-coder:7b`, managed VM): the model's first candidate, 6 lines, conforms 3 of 3; CANDIDATE_ACCEPTED; outcome completed. Its source is byte-identical to the hand example used to try the admission check: both are the shortest correct form; the record's ModelUsage (458 prompt tokens, 36 completion, 1,044 ms, not estimated) and Ollama's server log show the call happened.
- Real run 2 (attempt 1 planted from `probes/skeleton/candidates/loses_value.py`, then the model): 3 of 3 discrepancies ("forbidden outcome value lost: [] at no line"); REDUCTION_COMPLETED (steps 0); DISCREPANCY_REPORTED; DISCREPANCY_ROUTED implementation_repair on grounds that the 3 committed Windows reference executions of the same probe conform; the model, given only the structured divergence, returned the correct candidate (531 prompt tokens, 36 completion, 372 ms); 3 of 3 conform; CANDIDATE_ACCEPTED; DISCREPANCY_RESOLVED implementation_repaired; outcome completed_with_discrepancies.
- A first version of run 2 named the planted wrapper as the model with estimated counts (the wrapper hid the adapter's metered call). The wrapper now passes metering through; run 2 was repeated and the superseded session's 6 store executions removed.
- Records, read by command:
```
runs/candidates-1791507278435079000: {'SESSION_INIT': 1, 'ModelUsage': 1, 'CANDIDATE_GENERATED': 1, 'EXECUTION_STARTED': 3, 'EXECUTION_COMPLETED': 3, 'TRACE_INGESTED': 3, 'COMPARISON_COMPLETED': 3, 'CANDIDATE_ACCEPTED': 1, 'SESSION_COMPLETED': 1} | verdicts {'conforms': 3} | model calls [('qwen2.5-coder:7b', 458, 36, 1044)] | outcome completed
runs/candidates-1791507396445439000: {'SESSION_INIT': 1, 'ModelUsage': 2, 'CANDIDATE_GENERATED': 2, 'EXECUTION_STARTED': 6, 'EXECUTION_COMPLETED': 6, 'TRACE_INGESTED': 6, 'COMPARISON_COMPLETED': 6, 'CANDIDATE_REJECTED': 1, 'REDUCTION_COMPLETED': 1, 'DISCREPANCY_REPORTED': 1, 'DISCREPANCY_ROUTED': 1, 'CANDIDATE_ACCEPTED': 1, 'DISCREPANCY_RESOLVED': 1, 'SESSION_COMPLETED': 1} | verdicts {'discrepancy': 3, 'conforms': 3} | model calls [('planted', 0, 0, 0), ('qwen2.5-coder:7b', 531, 36, 372)] | outcome completed_with_discrepancies
```
- Defect found: contract 0.1.0 accepts `GetLastError: return 1234`. It judges one scenario with a value fixed at build time, so a constant that matches the scenario conforms. The prompt withholds the value, which only keeps this generator from copying it. A test pins the gap (`test_one_scenario_contract_accepts_a_constant`) so a contract revision must flip it deliberately. Fix path: a contract version whose scenario the candidate cannot predict (a value chosen at run time and recorded in the trace, with a relation between SetLastError's argument and GetLastError's result).
- A check of that defect was first run against the real store and wrote 3 executions and a candidate there; they were removed by candidate id.
- Gates: 183 passed.
