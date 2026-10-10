# SIGNAL_REPORT — Sprint 039

## 1. Observed

### scope_confirmation

> The model now holds XMM0-15, MXCSR, AF and DF, with SSE floating-point semantics computed exactly (`x86fp`), and an Intel CPU agrees with it on all 39,708 single-instruction cases, faults included. The CPU corrected three of the model's rules on the way. In an Apple M5 Max guest, FPCR.AH=1 gives x86's answer for MIN/MAX, NaNs, tininess and denormals; float-to-integer conversion still needs a fix-up.

### signal_trace

Read from the records by command:

```
store/runs/semantics-1791594801310547000 (Intel Xeon Platinum 8573C): SESSION_INIT, COMPARISON_COMPLETED verdict conforms, SESSION_COMPLETED completed
store/runs/semantics-1791594810963056000 (Intel Xeon Platinum 8370C): SESSION_INIT, COMPARISON_COMPLETED verdict conforms, SESSION_COMPLETED completed
store/runs/semantics-17915948{21,28,37}* (AMD EPYC 7763, 9V74, 9V45): COMPARISON_COMPLETED discrepancy, DISCREPANCY_REPORTED 1 (NOP cpu fault), DISCREPANCY_ROUTED contract_revision
store/runs/capability-fp-1791593555694176000: CAPABILITY_PROBED 6 (5 present, guest_fp_to_int_x86 absent)
```

## 2. Expected

> Four planted faults caught by the CPU alone; Sprint 037's results agree with AF compared; AFP present; with AH=1, FMIN on (number, NaN) returns the NaN as x86 does. Held, except that one planted fault (AF undefined) could not be caught by a judge that skips undefined flags, and was replaced by AF from the wrong bit.

## 3. Delta

### dual_contract_self_grade

**signal contract:** pass; vocabulary 0.7 adds six capability values (P17), which the card had not foreseen. **artifact contract:** gates.sh 0. **overall:** pass.

## 4. Hypothesis

> The rules the CPU had to teach (Windows's #XM codes, flags at an unmasked fault, PE under trapped overflow) are rules no text the project read stated plainly. Each class the pool drafts in Sprint 041 will carry rules like these, so a class should not be accepted on SDM reading alone: its acceptance is an oracle verdict from an Intel run.
