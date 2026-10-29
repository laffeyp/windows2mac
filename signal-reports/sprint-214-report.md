# SIGNAL_REPORT — Sprint 214 — The call comparison against Windows

Made by `signal_report 214` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 214.

## 1. Observed

### scope_confirmation

> The Decision "Sequence after Sprint 211" (census row 7; reviewer-3's Medium 3 of its review of Sprint 211: the ground
> truth before more comparisons to the oracle). The Rust host's batch-run stream is held to the frozen host's, which
> conforms to Windows over the reference's 168 log lines; nothing holds the Rust stream to Windows itself, so a frozen-host
> defect the stream copies passes. `scripts/call_compare.py` (781 lines) is that comparison in the frozen host: the
> Windows calls of the reference set (game run 38216081440 and its laps, `reference_set`), each run's capture thread left
> out (`excluded`), threads mapped by name and order (`thread_names`, `thread_map`), each value judged by its kind and the
> API's signature (`judge`, `rendering`, `carries_address`), the values Windows itself varies across the set held as
> such (`undetermined`, contracts/windows.game.calls 0.1.0), and the first divergence by cause (`compare_set`,
> `by_cause`).

### work_performed

- 80d769953 Sprint 214: the call comparison against Windows
- 9e028a520 Sprint 214 card: the call comparison against Windows

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - COMPARISON_COMPLETED on the batch run's record, as the frozen host's frontier writes it (run_frontier.py), with the
>   verdict and the first divergence; TRACE_INGESTED only if the Rust host writes a guest-plane trace, else the
>   comparison's input is named in `unobserved` (P22).

## 3. Delta

### emits_check

- `COMPARISON_COMPLETED`: NOT FIRED
- `TRACE_INGESTED`: NOT FIRED

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 214: its Rubber Duck Pass and close.
