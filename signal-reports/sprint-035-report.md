# SIGNAL_REPORT — Sprint 035

## 1. Observed

### scope_confirmation

> kernel32 for both probes now runs as freestanding C inside the managed VM, with the last-error code in the guest TEB at 0x68; the existing contracts judge it unchanged; candidates are C and run in the guest; and the translator's work is on the signal plane for the first time.

### signal_trace

Read from the records by command:

```
store/runs/translations-1791579909159674000: {'SESSION_INIT': 1, 'CODE_TARGET_DISCOVERED': 62, 'TRANSLATION_PUBLISHED': 62, 'SESSION_COMPLETED': 1}
store/runs/candidates-1791579927493057000: CANDIDATE_GENERATED 1, EXECUTION_COMPLETED 3, COMPARISON_COMPLETED 3 (conforms), CANDIDATE_ACCEPTED 1 | model qwen2.5-coder:7b 494/57 tokens
store/runs/candidates-1791579948983398000: CANDIDATE_REJECTED 1 (planted constant, 3 discrepancies), REDUCTION_COMPLETED 1, DISCREPANCY_REPORTED 1, DISCREPANCY_ROUTED 1 (implementation_repair), CANDIDATE_ACCEPTED 1, DISCREPANCY_RESOLVED 1, outcome completed_with_discrepancies
```

## 2. Expected

> Probes exit 0 in the VM; 3 of 3 conform under 0.1.0 and 0.2.0; lockstep and state differential 0 differences; calls from the guest ring equal the model's; TEB+0x68 holds the last value; one discovery and one publication per block. Held.

## 3. Delta

### dual_contract_self_grade

**signal contract:** pass (0.5 tags only; CODE_TARGET_DISCOVERED and TRANSLATION_PUBLISHED emitted for the first time). **artifact contract:** gates.sh 0. **overall:** pass. Amendment: TRANSLATION_INVALIDATED moves to the Mono JIT-region sprint.

## 4. Hypothesis

> The bridge saves and restores all 16 GPRs around every service call. That is what lockstep needs now; when services multiply, the reconstruction table (specs/guest_state.json) should let a bridge save only what the service can clobber.
