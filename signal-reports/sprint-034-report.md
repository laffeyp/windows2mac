# SIGNAL_REPORT — Sprint 034

## 1. Observed

### scope_confirmation

> The Windows-visible state the guest runtime will hold is fixed from Windows itself: CONTEXT, XMM_SAVE_AREA32, EXCEPTION_RECORD, TEB and PEB offsets from the runner's headers and from the running thread, fault and suspend behavior from a vectored handler and GetThreadContext/SetThreadContext; w2m's state block, register map and reconstruction-table format are written as `specs/guest_state.json` and judged against the runner.

### signal_trace

Read from the record by command:

```
store/runs/gueststate-1791579238609115000: {'SESSION_INIT': 1, 'COMPARISON_COMPLETED': 1, 'SESSION_COMPLETED': 1} | verdict conforms | 62 fields
```

## 2. Expected

> Probe exit 0 three times with equal layouts; the offsets named in the card; ExceptionCode 0xC0000005 with address 0; MXCSR 0x1F80 and x87 control word 0x27F. Held, every value.

## 3. Delta

### dual_contract_self_grade

**signal contract:** pass (0.5 tags only). **artifact contract:** gates.sh 0. **overall:** pass. Gap kept open: SegCs/SegGs values are unobserved.

## 4. Hypothesis

> The layouts are not the hard part; the reconstruction table is. Sprint 035 is where a CONTEXT is first built from host registers mid-block, and the runner's handler and suspend records are the reference it must reproduce.
