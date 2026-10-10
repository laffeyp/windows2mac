# SIGNAL_REPORT — Sprint 038

## 1. Observed

### scope_confirmation

> What Hail to the Rainbow needs is on the record from both sides. In w2m, the frontier stops at GetSystemTimeAsFileTime after 13 instructions; on Windows, the game calls 213 (batch) to 258 (play) imported functions 60-77 million times, resolves 341 names at run time including D3D11 and XInput, peaks at 41 threads, and in play mode renders on the software renderer and compiles 5,309 methods. Acquisition, the contract lifecycle and probe authorship now fire their tags on real runs.

### signal_trace

Read from the records by command:

```
store/runs/frontier-643a6283510c-1791588877289222000: SESSION_INIT, EXECUTION_STARTED, SERVICE_UNIMPLEMENTED (KERNEL32.dll!GetSystemTimeAsFileTime, 13 instructions), SESSION_COMPLETED
store/runs/acquire-1699230-1791588962620913000: DISTRIBUTION_RESOLVED (build 24247624, depot 1699231), DISTRIBUTION_ACQUIRED (15,584,319,636 bytes, steamcmd)
store/runs/acquire-409720-1791588973967301000: DISTRIBUTION_RESOLVED (build 8552776, depot 409721), DISTRIBUTION_ACQUIRED (21,582,474,233 bytes, steamcmd)
store/runs/contracts-1791589065188495000: EVIDENCE_RECORDED 37, CONTRACT_PROPOSED 7, CONTRACT_ACCEPTED 5, CONTRACT_REVISED 3, DEPENDENTS_INVALIDATED 3
store/runs/probes-*: PROBE_AUTHORED 11
```

## 2. Expected

> First stop GetSystemTimeAsFileTime after 13 instructions; the runner log includes GetCurrentThreadId and QueryPerformanceCounter; peak threads a number; both depots acquired at their builds; one CONTRACT_ACCEPTED per accepted version and one CONTRACT_REVISED per revision; the play capture larger than startup's. Held.

## 3. Delta

### dual_contract_self_grade

**signal contract:** pass; vocabulary 0.6 adds SERVICE_UNIMPLEMENTED and steamcmd. **artifact contract:** gates.sh 0. **overall:** pass.

## 4. Hypothesis

> The game's hot imports (GetCurrentThreadId, TlsGetValue, QueryPerformanceCounter: 68 million calls in 180 s) decide the guest runtime's shape more than its breadth does: these must cost a few nanoseconds in the guest, so the services the pools generate must default to running without leaving the VM.
