# SIGNAL_REPORT — Sprint 036

## 1. Observed

### scope_confirmation

> Decoder exit criterion (d) holds on Hail to the Rainbow: every one of the 718 spec rows the game's native and Mono-generated code uses is XED-confirmed, and every position decodes except two APX positions no target CPU executes. The new rows come from the SDM's own opcode lines, parsed by script, and XED judges them.

### signal_trace

Read from the records by command:

```
store/runs/gamediff-1791583316617553000: {'SESSION_INIT': 1, 'COMPARISON_COMPLETED': 1, 'DISCREPANCY_REPORTED': 1, 'DISCREPANCY_ROUTED': 1, 'SESSION_COMPLETED': 1} | rows used 718, unconfirmed 0 | the one class: 2 APX positions
store/runs/gendiff-1791583020404460000: {'SESSION_INIT': 1, 'COMPARISON_COMPLETED': 1, 'DISCREPANCY_REPORTED': 134, 'DISCREPANCY_ROUTED': 134, 'SESSION_COMPLETED': 1}
```

## 2. Expected

> length_differs 0; ours_accepts falls; confirmations rise; game gaps 0 and every used row confirmed or named. Held: ours_accepts 4,490 → 2,537, confirmations 549/809 → 971/1,061, game unconfirmed 141 → 0, game gaps 8,962 → 2 (named).

## 3. Delta

### dual_contract_self_grade

**signal contract:** pass. **artifact contract:** gates.sh 0 (231 passed). **overall:** pass.

## 4. Hypothesis

> Most decoder error lived in validity, not in opcode tables: prefixes that select or forbid, L/W/vvvv constraints, register-only forms. A generator reading the SDM's opcode line carries those constraints by construction, so hand-written vector rows should be retired the same way as the game reaches them.
