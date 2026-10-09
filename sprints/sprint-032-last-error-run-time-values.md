# Sprint 032 — last-error contract judged on values chosen at run time

```yaml
---
id: 032
status: closed
phase: 3
pass_kind: functional
---
```

## scope

Contract `windows.thread.last_error.set_get@0.1.0` accepts a candidate whose GetLastError returns the constant 1234 (Sprint 031; open question; `test_one_scenario_contract_accepts_a_constant`). Version 0.2.0 replaces the one fixed value with three values the probe picks at run time:

1. A new probe, `probes/last_error_values/last_error_values.c` (no C runtime, as the skeleton). It reads QueryPerformanceCounter once, before any SetLastError, and mixes the count into three table indices. Consecutive indices differ, so consecutive values differ. It then makes three pairs: SetLastError(v), then GetLastError(), with only string-building code between them.
2. The table is `probes/last_error_values/values.h`, written by `scripts/gen_last_error_values.py`. It holds 256 distinct 32-bit values with the args_sha256 of each one's rendering `SetLastError(<decimal>)` (vocabulary 0.4 note): 0, 1, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFF, and 251 values from a seeded generator. The probe writes the true args_sha256 of each call without hashing in the guest, so the trace stays honest under the 0.5 vocabulary with no new field.
3. A second acceptance relation in `src/w2m/contract.py` and `src/w2m/compare.py`: `exact` (v0.2 §3, "Exact value or bits ... error codes"), in the form "each `read_api` call by a role returns exactly the integer the role's most recent `write_api` call was given". The comparator checks it by hashing `SetLastError(<returned value>)` and comparing with the args_sha256 of that SetLastError line. It never learns the value except from the return; a constant passes a run with probability at most 1 in 256 per pair, and consecutive pairs differ.
4. `src/w2m/kernel32.py`: QueryPerformanceCounter (10 MHz counts from the host's monotonic clock), with the clock injectable so lockstep and the state differential give the VM and the model the same count. A handler that raises stops the run as `HandlerFailed` instead of crashing the runner.
5. `src/w2m/candidate.py`: `args` holds only the function's declared parameters (SetLastError 1, GetLastError 0, per the Microsoft Learn signatures). A GetLastError that returns `args[0]` would otherwise read RCX, which may still hold the SetLastError argument.
6. `scripts/run_candidate_loop.py`: the contract chooses the probe and the relation; the default becomes 0.2.0.

The Windows reference: three runner executions of the new probe (windows-probe.yml), judged under 0.2.0.

## prerequisites

- 031

## context_files

- v0.2 §3 (relations), §11.2; vocabulary 0.4 note on args_sha256; KIT_DIARY #15
- Microsoft Learn: SetLastError, GetLastError (as 0.1.0); QueryPerformanceCounter (page updated 2026-05-15; retrieved 2026-10-09)
- `probes/skeleton/skeleton.c`, `src/w2m/compare.py`, `src/w2m/contract.py`, `src/w2m/candidate.py`, `scripts/run_candidate_loop.py`

## signal contract

### Emits

- guest plane: GUEST_THREAD_STARTED, 6 GUEST_CALL_RETURNED (3 SetLastError, 3 GetLastError), GUEST_THREAD_EXITED
- harness plane: as Sprint 031; COMPARISON_COMPLETED and DISCREPANCY_REPORTED carry relation / mismatch_class `exact`

### Invariants

- Every args_sha256 the probe writes equals sha256 of the canonical rendering of the argument it passed (checked by a test over the whole table).
- No vocabulary change: 0.5 tags and fields only.

## artifact contract

### Files created

- `probes/last_error_values/last_error_values.c`, `probes/last_error_values/values.h`, `probes/last_error_values/probe.json`, `scripts/gen_last_error_values.py`, `contracts/windows.thread.last_error.set_get/0.2.0.json`, `tests/test_last_error_values.py`

### Files modified

- `src/w2m/contract.py`, `src/w2m/compare.py`, `src/w2m/kernel32.py`, `src/w2m/runtime.py`, `src/w2m/vm_backend.py`, `src/w2m/candidate.py`, `scripts/run_candidate_loop.py`, `tests/test_candidate_loop.py`

Hard rule 6 exception: a relation is a format rule and a comparator together, and the probe, its table and its contract are judged as one. Recorded here.

### Command exit codes

- `scripts/gates.sh` → 0; CI success

## observation contract

### Expected runtime signals

- Windows reference: 3 of 3 conform under 0.2.0, with 3 different triples of SetLastError hashes across the 3 runs (the values vary at run time).
- Model and managed VM: the probe runs to exit 0 with host kernel32; 3 of 3 conform each; lockstep and the state differential find 0 differences.
- Candidate loop, scripted: the constant `return 1234` is rejected with a discrepancy; a GetLastError returning `thread.tid` is rejected; a GetLastError reading `args[0]` stops with HandlerFailed and is rejected; the correct candidate is accepted.
- Candidate loop, real (`qwen2.5-coder:7b`, managed VM): whatever the record shows.

## done criteria

No candidate that ignores SetLastError's argument conforms to `windows.thread.last_error.set_get@0.2.0`, the Windows reference conforms to it, and the candidate loop runs against it.

## self-review checklist

- [x] Scope concrete. [x] Sources named. [x] Emits only 0.5 tags. [x] Observation declared before running. [ ] ≤2 files: exception recorded.
- [ ] Two items differ from the scope as written: a GetLastError reading `args[0]` is refused at admission (the parameter count is checked statically), so HandlerFailed is exercised by a candidate that divides by zero; and two harness bugs were fixed (lockstep, the loop's guard trigger). Recorded in observed.

## observed
- Codegen checked before the runner build: the first table layout (one struct of value and hash, 76 bytes) compiled to `imul $0x4c`, which the lifter has no rule for. Two arrays (4-byte values, 72-byte hash strings) index with LEA and SIB scaling only; the built probe needed no new lifting rule.
- Windows reference (windows-probe.yml run 37897186217, gha-windows-2025-x64-10.0.26100.33438-Server): exit 0 three times; 9 different SetLastError values over the 3 runs, consecutive values different in each; 3 of 3 conform under 0.2.0 (`exact`).
- Lockstep, first run: 23 differences from step 4,404, the VM one x86 instruction ahead of the model. Cause in the lockstep harness, not the compiler: after a kernel32 call the VM re-dispatched in the guest, and when the return address's block was already compiled (the third SetLastError returns where the first did) the block ran before the next check. The skeleton returns to each address once, so Sprint 030 never reached this path. Fixed: in lockstep mode the return address is checked in the host as its own step. After the fix: skeleton 5,159 comparisons (unchanged), new probe 11,931, 0 differences each; with the fix reverted, the new test fails at step 4,404.
- State differential on the new probe: 16,400 cases over 410 instructions, 0 differences.
- Model and managed VM, 3 runs each: exit 0; 12,060 to 12,100 instructions (the count varies with the decimal length of the chosen values); 410 distinct addresses; 11 ms (model), 21-24 ms (VM); 3 of 3 conform each.
- Candidate loop bug found by the new test for a candidate that raises: three failed runs gave three CANDIDATE_REJECTED for one candidate. The runner recorded each failure in shared state before the trigger read the first event, so "all runs in" held on each of the three events. Sprint 031's tests never had every run of a candidate fail. Fixed: the trigger records each failure as its event arrives, as it already did for comparisons.
- Scripted candidates under 0.2.0: `return 1234` rejected 3 of 3 ("returned 1234, which is not the argument of kernel32!SetLastError at 4:1") and the same candidate still accepted under 0.1.0; `return thread.tid` rejected with a discrepancy; `args[0]` in GetLastError refused at admission ("GetLastError has 0 parameters"); a division by zero stops each run with HandlerFailed and the candidate is rejected without a discrepancy.
- Records, read by command:
```
runs/judge-1791529699873420000: {'SESSION_INIT': 1, 'EXECUTION_STARTED': 3, 'EXECUTION_COMPLETED': 3, 'TRACE_INGESTED': 3, 'COMPARISON_COMPLETED': 3, 'SESSION_COMPLETED': 1} | verdicts {'conforms': 3} | relation {'exact': 3} | profile_kind {'windows_reference': 3}
runs/judge-1791529701004655000: same counts | verdicts {'conforms': 3} | relation {'exact': 3} | profile_kind {'model': 3}
runs/judge-1791529701091251000: same counts | verdicts {'conforms': 3} | relation {'exact': 3} | profile_kind {'managed_vm': 3}
runs/candidates-1791529836408254000: {'SESSION_INIT': 1, 'ModelUsage': 1, 'CANDIDATE_GENERATED': 1, 'EXECUTION_STARTED': 3, 'EXECUTION_COMPLETED': 3, 'TRACE_INGESTED': 3, 'COMPARISON_COMPLETED': 3, 'CANDIDATE_ACCEPTED': 1, 'SESSION_COMPLETED': 1} | verdicts {'conforms': 3} | relation {'exact': 3} | profile_kind {'managed_vm': 3} | model calls [('qwen2.5-coder:7b', 461, 36, 518)] | outcome completed
runs/candidates-1791529841605209000: {'SESSION_INIT': 1, 'ModelUsage': 2, 'CANDIDATE_GENERATED': 2, 'EXECUTION_STARTED': 6, 'EXECUTION_COMPLETED': 6, 'TRACE_INGESTED': 6, 'COMPARISON_COMPLETED': 6, 'CANDIDATE_REJECTED': 1, 'REDUCTION_COMPLETED': 1, 'DISCREPANCY_REPORTED': 1, 'DISCREPANCY_ROUTED': 1, 'CANDIDATE_ACCEPTED': 1, 'DISCREPANCY_RESOLVED': 1, 'SESSION_COMPLETED': 1} | verdicts {'discrepancy': 3, 'conforms': 3} | relation {'exact': 6} | profile_kind {'managed_vm': 6} | model calls [('planted', 0, 0, 0), ('qwen2.5-coder:7b', 565, 36, 431)] | outcome completed_with_discrepancies | grounds ['3 of 3 committed Windows reference executions of sha256:d210df33adce2f08 conform']
```
- Real run 1: the model's first candidate (sha256:ff4136e7…, the same source as in Sprint 031) conforms 3 of 3 under 0.2.0. Real run 2: the planted constant (`probes/last_error_values/candidates/returns_constant.py`) is rejected 3 of 3, reported with mismatch_class `exact`, routed implementation_repair, and the model's repair from the divergence alone is accepted and the discrepancy resolved.
- Gates: `scripts/gates.sh` 0, 202 passed.
