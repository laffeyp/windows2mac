# SIGNAL_REPORT — Sprint 024

## 1. Observed

### scope_confirmation

> Locked vocabulary 0.4 (P11 rename, P12 `not_established`, P13 optional unobserved fields) and made the three emitters stop asserting what they did not observe; committed the Mac hardware run records and the runner role maps.

### work_performed

- Wrote `signals/0.4.json`, `0.4-rationale.md`; ratified P11-P13 in `proposals.json`; regenerated `src/w2m/events.py`.
- Reworked `judge_traces.py`: no constants for timeout, grace, exit code, elapsed; execution metadata stored per trace; `store:<run id>` re-judges from committed data.
- `litmus_mp.py` records `present` only with power; `probe_capabilities.py` records `native_process_tso` not_established.
- Workflow records exit code and elapsed time per execution; actions pinned to v4.4.0 and v4.6.2 commit SHAs.

### signal_trace

From `runs/judge-1791496361` (re-judge from store), `runs/capability-1791496383`, `runs/litmus-1791496387`:

```
judge:      SESSION_INIT; EXECUTION_STARTED x10 (unobserved=timeout_seconds, kill_grace_seconds ...); EXECUTION_COMPLETED x10 (exit_code, elapsed_seconds omitted; unobserved names them); TRACE_INGESTED x10; COMPARISON_COMPLETED x10 verdict=conforms; SESSION_COMPLETED
capability: CAPABILITY_PROBED ipa_granule_4k present, guest_tsoen present, guest_tsoen_context_switch present, host_page_16k present, native_process_tso not_established
litmus:     CAPABILITY_PROBED guest_tsoen present (off 35/50,000,000; on 0/50,000,000), guest_tsoen_context_switch present (0 mismatches in 200 exits)
```

### signal_trace_tags

- `SESSION_INIT`, `EXECUTION_STARTED`, `EXECUTION_COMPLETED`, `TRACE_INGESTED`, `COMPARISON_COMPLETED`, `CAPABILITY_PROBED`, `MEASUREMENT_RECORDED`, `SESSION_COMPLETED`

## 2. Expected

> The card declared `CAPABILITY_PROBED` with `native_process_tso not_established`, the judge path with `unobserved` on both execution tags and no `elapsed_seconds`, and `guest_tsoen present` with off > 0. All fired as declared.

## 3. Delta

### dual_contract_self_grade

**signal contract:** every declared tag fired; payloads read back from the records match the declared values. *pass*

**artifact contract:**

- `signals/0.4.json`: `capability_outcome` ends with `not_established`; no `cost` anywhere [pass]
- `judge_traces.py` holds no `600.0` or `elapsed_seconds=0.0` [pass]
- check_vocab 0, gen_events --check 0, pytest 140 passed, ruff 0, mypy 0 [pass]

**overall:** pass

## 4. Hypothesis

> The workflow's new `exec-<i>.json` has not run yet; its PowerShell quoting is unexercised. The next runner dispatch is its test, and `judge_traces.py` will omit nothing on that run if it works.
