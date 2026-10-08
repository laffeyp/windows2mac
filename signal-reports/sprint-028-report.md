# SIGNAL_REPORT — Sprint 028

## 1. Observed

### scope_confirmation

> Replaced the event probe's Sleep(200) assumption with a recorded observation that both waiters are in the kernel's Wait state for a user request before SetEvent, and made the comparator refuse to count runs without it.

### work_performed

- Vocabulary 0.5: guest tag `GUEST_THREAD_STATE_OBSERVED` (P14).
- Probe: polls `NtQuerySystemInformation(SystemProcessInformation)`, linked at run time, with struct layouts declared from the documentation and size-checked at compile time.
- Contract format `preconditions`; `compare.unwitnessed`; contracts 0.3.0 (proposed) and 0.3.1 (accepted).
- 20 runner executions (run 37854891607), judged; `scripts/gates.sh`.

### signal_trace

Read from the records by command, not typed:

```
runs/judge-1791499291481741000: SESSION_INIT 1, EXECUTION_STARTED 20, EXECUTION_COMPLETED 20, TRACE_INGESTED 20, COMPARISON_COMPLETED 20, SESSION_COMPLETED 1 | verdicts {conforms: 20} | first SESSION_INIT last SESSION_COMPLETED
guest lines over 20 traces: GUEST_TRACE_OPENED 20, GUEST_THREAD_STARTED 60, GUEST_THREAD_STATE_OBSERVED 40, GUEST_CALL_RETURNED 120, GUEST_THREAD_EXITED 40, GUEST_TRACE_CLOSED 20
```

### signal_trace_tags

- `GUEST_THREAD_STATE_OBSERVED` (first firing), and the judge tags listed above

## 2. Expected

> Two witness lines per trace before SetEvent; 20 conforms or inconclusive naming the gap; 12 earlier executions inconclusive under 0.3.0. All held: 40 witness lines, 20 conforms, 12 inconclusive.

## 3. Delta

### dual_contract_self_grade

**signal contract:** declared tags fired in the declared counts. *pass*

**artifact contract:** `scripts/gates.sh` exit 0 (155 passed); tamper tests refuse a missing, late, or wrong-state witness. Part 1 was committed with a failing test and fixed one commit later. *pass after correction*

## 4. Hypothesis

> Waiter a won 20 of 20 with both proven pending. If release follows wait order, a variant where b enters its wait first (witnessed the same way) will release b; if it follows creation or thread-id order, it will still release a.
