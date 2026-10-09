# SIGNAL_REPORT — Sprint 033

## 1. Observed

### scope_confirmation

> Hail to the Rainbow's code is on the record and judged: the game was packed into a private release, unpacked and verified on the Windows runner, started in Unity batch mode through a launcher that hooks Mono's code generation, and every position of its seven native PE files and of the code Mono generated was compared with Intel XED; decoder criterion (d) is measured by spec row.

### signal_trace

Read from the record by command:

```
store/runs/gamediff-1791576553479425000: {'SESSION_INIT': 1, 'COMPARISON_COMPLETED': 1, 'DISCREPANCY_REPORTED': 125, 'DISCREPANCY_ROUTED': 125, 'SESSION_COMPLETED': 1} | verdicts ['discrepancy'] | routes {'contract_revision': 125}
```

## 2. Expected

> Release parts equal `release.json`; the unpacked tree equals `files.json` (the card said 298 files; the depot is 297); the game starts in batch mode; the capture holds at least one Assembly-CSharp method; the differential lists every gap and disagreement. Held: 564 Assembly-CSharp methods; 0 length or iclass differences in 6,192,478 positions; 8,962 gap positions in 125 kinds.

## 3. Delta

### dual_contract_self_grade

**signal contract:** pass (0.5 tags only; one COMPARISON_COMPLETED, each DISCREPANCY_REPORTED followed by its DISCREPANCY_ROUTED). **artifact contract:** gates.sh 0 (208 passed). **overall:** pass, with the decoder found short on this game: criterion (b)'s "spec gaps 0" does not hold here, and criterion (d) stands at 438 of 579 rows confirmed.

## 4. Hypothesis

> The decoder's remaining distance to this game is mostly two validity rules (LEA mod=3; F2/F3 precedence over 66) plus the SSE, MMX and FMA rows the Python corpus never needed. Run-time code adds little decoder work (101 positions) but most of the runtime work: trampolines that read their argument from the instruction stream, RWX code patched in place, and code that differs by CPU.
