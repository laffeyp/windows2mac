# SIGNAL_REPORT — Sprint 030

## 1. Observed

### scope_confirmation

> Compiled the skeleton's lifted operations to AArch64 and ran them in a Hypervisor.framework VM, with kernel32 in the host; compared the VM with the model after every instruction and from random states; judged the VM's traces against the same contract.

### signal_trace

Read from the records by command:

```
runs/judge-1791505504109764000: SESSION_INIT 1, EXECUTION_STARTED 3, EXECUTION_COMPLETED 3, TRACE_INGESTED 3, COMPARISON_COMPLETED 3, SESSION_COMPLETED 1 | verdicts {'conforms': 3} | profile_kind {'windows_reference': 3}
runs/judge-1791505504702339000: SESSION_INIT 1, EXECUTION_STARTED 3, EXECUTION_COMPLETED 3, TRACE_INGESTED 3, COMPARISON_COMPLETED 3, SESSION_COMPLETED 1 | verdicts {'conforms': 3} | profile_kind {'model': 3}
runs/judge-1791505504772307000: SESSION_INIT 1, EXECUTION_STARTED 3, EXECUTION_COMPLETED 3, TRACE_INGESTED 3, COMPARISON_COMPLETED 3, SESSION_COMPLETED 1 | verdicts {'conforms': 3} | profile_kind {'managed_vm': 3}
```

Guest plane (each profile, per run): GUEST_TRACE_OPENED, GUEST_THREAD_STARTED, GUEST_CALL_RETURNED ×2, GUEST_THREAD_EXITED, GUEST_TRACE_CLOSED.

## 2. Expected

> Conforms 3/3 under profile_kind managed_vm; 0 lockstep differences over 5,153 instructions; the model's instruction count and distinct addresses. Held.

## 3. Delta

### dual_contract_self_grade

**signal contract:** pass. **artifact contract:** gates.sh 0 (177 passed); encoder equal to clang on 67 forms; planted faults found. **overall:** pass, with one addition to the declared observation: the random-state differential, added after a planted carry fault passed lockstep.

## 4. Hypothesis

> A single program path is a weak check on a compiler: the skeleton's SUBs never see operands of different sign, so signed and unsigned borrow agree on all of them. Per-instruction random states close that for the instructions a program reaches; instructions a program does not reach need the same differential over generated encodings (encgen), which is the next widening of this check when the lifter grows.
