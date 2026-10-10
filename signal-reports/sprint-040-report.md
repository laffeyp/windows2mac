# SIGNAL_REPORT — Sprint 040

## 1. Observed

### scope_confirmation

> At a fault, the VM back end's guest runtime now builds the EXCEPTION_RECORD and CONTEXT that Windows built for the same instruction and state on two Intel CPUs, on every one of the 1,916 fault cases the VM compiles, and the VM agrees with those CPUs on the other 20,448 cases too, with lazy AF and DF compared. An interrupted vCPU stops only at instruction boundaries. The guest sees a fixed Intel x86-64-v2 CPUID profile.

### work_performed

- Lifter: the commit rule (no register, flag or DF write before an instruction's last memory access; one store at most, last). PUSH, CALL, ALU, INC, DEC and SHR reordered; POP to memory stored after moving RSP (a defect since Sprint 029, found by the new static check) and now computes an RSP-based destination after the increment, as the SDM POP page states. `commit_order()` holds on all 8,820 liftable generated encodings.
- VM: lazy AF (operands in the monitor data page, an undefined bit in x24), DF, the reconstruction table in guest memory, an EL1 exception entry that saves the registers and calls the guest runtime, translation flush, asynchronous interruption with the forward rule.
- Guest runtime: the state block, EXCEPTION_RECORD and CONTEXT at the spec's offsets (compile-time asserts), `w2m_exception` (access violation, unhandled: exit with the code), `w2m_cpuid` from `specs/guest_cpuid.json`.
- `scripts/vm_oracle.py`, `tests/test_vm_state.py`, `src/w2m/cpuid.py`; the fault extract `store/probe-runs/38011862284/result-1.faults.bin` (5,485 Windows results with their case indices, from the release file whose sha256 it records).

### signal_trace

```
store/runs/vmoracle-1791609837655531000 (Intel Xeon Platinum 8573C): SESSION_INIT, COMPARISON_COMPLETED verdict conforms (22,364 cases, 1,916 faults), SESSION_COMPLETED completed
store/runs/vmoracle-1791609842914609000 (Intel Xeon Platinum 8370C): SESSION_INIT, COMPARISON_COMPLETED verdict conforms (22,364 cases, 1,916 faults), SESSION_COMPLETED completed
```

## 2. Expected

> COMPARISON_COMPLETED conforms on Intel, with discrepancies reported and routed if any; planted faults caught; the forward rule's count exact with at least 100 interruptions; CPUID equal in model, VM and table. All held, after two changes the evidence forced (below).

## 3. Delta

### dual_contract_self_grade

**signal contract:** pass (both records as above; no discrepancy to report). **artifact contract:** `scripts/gates.sh` 0 — 269 tests, ruff, mypy, 39 records replay. **observation contract:**

- Faults: CONTEXT and exception record equal Windows's on Intel for every fault case the VM compiles (1,916 of 5,485 faults; the rest are SSE classes, Sprint 041) — exception code, RIP, ExceptionAddress, 16 GPRs, EFlags (CF PF AF ZF SF OF DF IF, bit 1), data, stack and edge windows. Pass.
- Completed cases: 20,448 equal in GPRs, path, flags with AF where defined, DF and windows. Pass.
- Planted faults: POP [m] moving RSP before its store → `fault r4`; lazy AF from the wrong operand → `AF`; reconstruction table one byte off → `fault rip`. All caught. The card's plant "PUSH moving RSP first" could not be caught (Rubber Duck observation 1) and was replaced by POP.
- Forward rule: 300,000,000 iterations, 763 interruptions at arbitrary host instructions, every stop at the loop block's first instruction (0x20000005), count exactly 300,000,000. The backward rule, planted, ends the run fetching from address 0 with the count at 4,691,481.
- CPUID: model, VM and table equal for 12 leaves and 6 out-of-range inputs; 18 reads logged in the ring.

**overall:** pass.

## 4. Hypothesis

### rubber_duck_observations

**Sequence narration:** the comparator built the 039 cases from the seed, checked their sha256 against the one the runner recorded, read the Intel CPU's results, ran each compilable case in one VM, read the VM's state at the next dispatch or the guest runtime's records after a fault, compared, and recorded one COMPARISON_COMPLETED per CPU.

- *Observation 1 (a planted fault outside the reference again):* PUSH faults in 039 are loads of a memory source; a stack store that faults cannot be dispatched on Windows, so no record shows a PUSH that moves RSP early. Disposition: resolved-here (POP [m], whose store faults, planted instead); KIT_DIARY #22 — the second time in two sprints, so a plant is now checked against the case inventory before it is written.
- *Observation 2 (the reference's inputs depended on the code under test):* 039's case generator kept whatever the lifter could lift; lifting CPUID changed the case set and broke every judgment against runs already made. Disposition: resolved-here (`CLASSES_039` freezes the set; new classes get a new run); KIT_DIARY #23.
- *Observation 3 (a static check found what the CPU had not):* `commit_order` found POP [m]'s order defect, a defect present since Sprint 029 that Sprint 039's CPU runs never exposed, because the model rolls back every register at a fault. Disposition: resolved-here; the VM, which does not roll back, is now judged on the same cases.
- *Observation 4 (the sprint's original judge could not run):* Sprint 034's probe needs the C runtime. Disposition: recorded (user decision 2026-10-09, plan reordered; the scheduler and suspension moved to 047; a no-CRT copy of the probe was withdrawn).

### status_and_blockers

`status: complete`
