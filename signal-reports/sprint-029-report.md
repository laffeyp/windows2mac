# SIGNAL_REPORT — Sprint 029

## 1. Observed

### scope_confirmation

> Carried one runner-built Windows program through every stage: reference run, contract, load, decode, lift, execute with host kernel32, trace, judge.

### signal_trace

Read from the records by command:

```
runs/judge-1791500576975719000: SESSION_INIT 1, EXECUTION_STARTED 3, EXECUTION_COMPLETED 3, TRACE_INGESTED 3, COMPARISON_COMPLETED 3, SESSION_COMPLETED 1 | verdicts {'conforms': 3} | profile_kind {'windows_reference'}
runs/judge-1791500577024299000: SESSION_INIT 1, EXECUTION_STARTED 3, EXECUTION_COMPLETED 3, TRACE_INGESTED 3, COMPARISON_COMPLETED 3, SESSION_COMPLETED 1 | verdicts {'conforms': 3} | profile_kind {'model'}
```

Guest plane (both profiles, per run): GUEST_TRACE_OPENED, GUEST_THREAD_STARTED, GUEST_CALL_RETURNED ×2, GUEST_THREAD_EXITED, GUEST_TRACE_CLOSED.

## 2. Expected

> Conforms on both profiles; the Mac execution recorded as profile_kind model. Held: 3/3 and 3/3.

## 3. Delta

### dual_contract_self_grade

**signal contract:** pass. **artifact contract:** gates.sh 0; planted faults change the result. **overall:** pass

## 4. Hypothesis

> The IR covers what one compiler emitted for one small function set. The next program that widens it should be chosen by the game profile (BioShock 2 Remastered's reachable iclasses), not by what is easy to lift.
