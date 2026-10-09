# SIGNAL_REPORT — Sprint 037

## 1. Observed

### scope_confirmation

> The lifter is now judged by a real x86 CPU: 20,116 single-instruction cases over the 53 iclasses it handles ran on the runner's Intel and AMD CPUs from stated states, and the model agrees with the Intel CPU on every case. The oracle found a lifter defect that lockstep could not (16-bit PUSH), a second one by its own self-check (FS/GS segment bases dropped), and a vendor difference XED does not show (0F 0D register forms).

### signal_trace

Read from the records by command:

```
store/runs/semantics-1791586629398365000: {'SESSION_INIT': 1, 'COMPARISON_COMPLETED': 1, 'SESSION_COMPLETED': 1} | verdict conforms | Intel Xeon Platinum 8573C
store/runs/semantics-1791586641965639000: {'SESSION_INIT': 1, 'COMPARISON_COMPLETED': 1, 'DISCREPANCY_REPORTED': 1, 'DISCREPANCY_ROUTED': 1, 'SESSION_COMPLETED': 1} | 0F 0D register forms fault, routed contract_revision | AMD EPYC 7763
```

## 2. Expected

> Every case completes; results stable across runs; a planted SUB carry fault reported by the CPU alone; real defects fixed and the run repeated. Held, with one class left that is a vendor rule, not a defect.

## 3. Delta

### dual_contract_self_grade

**signal contract:** pass. **artifact contract:** gates.sh 0. **overall:** pass. Review 2026-10-09 H3 is closed.

## 4. Hypothesis

> One reference CPU is not "the CPU": two vendors disagree on defined behavior (0F 0D) and on undefined flags (SHR, MUL). Reference profiles must name the vendor and model, and contracts must say which they cover.
