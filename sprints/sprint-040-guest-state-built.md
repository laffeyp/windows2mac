# Sprint 040 — guest state built

```yaml
---
id: 040
status: closed
phase: 3
pass_kind: functional
---
```

## scope

The VM back end keeps x86 state in host registers and has no way to say what the guest's state was when something stops it: a fault ends the run with a host-side message, AF is not kept at all, DF is not compiled, and an asynchronous stop (`hv_vcpus_exit`) lands at any host instruction. Review 2026-10-09 (progress) N2 and N3 and `specs/guest_state.json` 0.2.0 state the rules; this sprint builds them in the VM and judges them on Windows's own records.

1. **Commit rule in the lifter** (`src/w2m/lift.py`): no register, flag or DF write before an instruction's last memory access, and at most one memory store, last among the memory accesses. PUSH and CALL store before they move RSP; ALU, INC, DEC and SHR on memory store before they set flags. `commit_order(lifted)` checks it statically; a test runs it over every encoding the lifter accepts from `encgen` and the 039 case file.
2. **Lazy AF and DF in the VM** (`src/w2m/vm_backend.py`): ADD, SUB, INC and DEC store their operands and result in the monitor data page (AF = bit 4 of a ^ b ^ r); logic, SHR and MUL mark AF undefined (bit 5 of x24); CLD and STD store DF. Lockstep and the state differential compare AF and DF with the model.
3. **The state block and the reconstruction table in guest memory.** The table (host code address, x86 instruction address) is written as blocks compile, sorted by construction. The EL1 synchronous-exception vector saves x0-x30, ESR, FAR and ELR to a save area, leaves exception state, and calls the guest runtime.
4. **The exception path in the guest runtime** (`tools/guest_runtime/`): from the save area it maps ELR to the guest instruction through the table, fills the thread's state block (GPRs, RIP, RFLAGS with AF and DF materialized, IF and bit 1 set), and builds the EXCEPTION_RECORD (code 0xC0000005, address, two parameters) and the CONTEXT (`context_from_state` of the spec; segment selectors, MXCSR 0x1F80, x87 control word 0x27F) at Windows layouts. With no vectored handler (VEH dispatch is Sprint 047), the exception is unhandled: the process ends with the exception code as its exit code, as on Windows, and the records stay in guest memory.
5. **The forward rule.** Each block's entry checks the instruction counter against a stop point in the monitor data page; a host that has interrupted the vCPU (`hv_vcpus_exit`) at an arbitrary host instruction sets the stop point and resumes the vCPU unchanged, so it stops at the next block entry, a guest instruction boundary where the state is exact.
6. **The guest CPUID profile** (`specs/guest_cpuid.json`, Decision 2026-10-09 N5): Intel vendor, x86-64-v2 features (SSE through SSE4.2, POPCNT, CMPXCHG16B; no AVX, no AVX-512), served by the model and by the guest runtime from the same table.
7. **The oracle in the VM** (`scripts/vm_oracle.py`): Sprint 039's cases run one at a time in one VM; for each fault the host reads the CONTEXT and EXCEPTION_RECORD the guest runtime built and compares them with the Windows result; for each completed case it compares GPRs, flags with AF, DF and the windows. One substrate run records the verdict.

The VM compiles integer instructions only; SSE lowering is Sprint 041. Cases whose instruction the VM cannot compile are counted by class, not dropped silently.

## prerequisites

- 039 (fault cases and Windows results on Intel), 035 (guest runtime), 030 (VM back end)

## context_files

- `specs/guest_state.json` 0.2.0, `docs/review-2026-10-09-progress.md` (N2, N3, N5), `docs/plan.md`
- `src/w2m/lift.py`, `src/w2m/runtime.py`, `src/w2m/vm_backend.py`, `src/w2m/a64_encoder.py`, `src/w2m/guest_runtime.py`, `tools/guest_runtime/runtime.{c,h}`, `tools/vm_monitor/vm_monitor.c`
- `scripts/semantics_differential.py` (case and result formats 2), `store/probe-runs/38011862284/` and `38011858047/` (Intel), release `semantics-039`
- Arm ARM DDI 0487: ESR_EL1 (EC 0x25 data abort, WnR), ELR_EL1, SPSR_EL1, ERET, VBAR_EL1 vector offsets; Intel SDM Vol 1 3.4.3 (EFLAGS), Vol 2 CPUID; Microsoft Learn: CONTEXT (x64), EXCEPTION_RECORD

## signal contract

### Emits

- harness plane: SESSION_INIT, COMPARISON_COMPLETED (contract `w2m.guest_state.fault_context` at the spec version, relation exact, reference the Intel runner profile), DISCREPANCY_REPORTED / DISCREPANCY_ROUTED per differing class, SESSION_COMPLETED.

### Invariants

- No vocabulary change. The reference is the CPU's result file; the judge reads no lifter or VM code to produce it.

## artifact contract

### Files

- created: `scripts/vm_oracle.py`, `specs/guest_cpuid.json`, `tests/test_vm_state.py`
- modified: `src/w2m/lift.py`, `src/w2m/runtime.py` (CPUID), `src/w2m/vm_backend.py`, `src/w2m/a64_encoder.py` (MRS, MSR, ERET, STRB), `src/w2m/guest_runtime.py`, `tools/guest_runtime/runtime.{c,h}`, `tests/test_vm_backend.py`, `tests/test_a64_encoder.py`
- File count over 10 (project override, Decision 2026-10-09): one concept (the guest state at a stop) crosses the lifter, the AArch64 back end and the guest runtime; splitting it leaves a half that cannot be judged.

### Command exit codes

- `scripts/gates.sh` → 0

## observation contract

### Expected runtime signals

- **Faults:** for every 039 fault case the VM can compile, the guest runtime's CONTEXT equals Windows's on Intel: exception code, RIP, the 16 GPRs, and RFLAGS in every defined flag (CF PF AF ZF SF OF, DF, IF); the data, stack and edge windows equal Windows's (no store before the fault). The number of such cases is on the record; target: every integer-class fault case.
- **Completed cases:** GPRs, flags including AF where defined, DF and windows equal Windows's on every case the VM can compile.
- **Planted faults, each caught:** PUSH moving RSP before its store (a fault case's RSP differs); AF taken from bit 3 (AF differs on completed cases); the reconstruction table one entry off (fault RIP differs).
- **Forward rule:** a loop of `ADD [m], 1; PUSH; POP; DEC; JNZ` run 200,000 times while the host interrupts the vCPU at least 100 times at arbitrary host instructions and stops it at the next boundary each time: the final count is exactly 200,000, and every stop's RIP is the start of a translated block.
- **CPUID:** the model and the VM return the table's values for every leaf and subleaf in `specs/guest_cpuid.json`.
- Lockstep and the state differential on both skeleton probes: 0 differences, now including AF and DF.

## done criteria

The VM builds, at a fault, the CONTEXT and exception record Windows built for the same instruction and state on Intel, keeps AF and DF as the CPU does, and stops an interrupted vCPU only at an instruction boundary without repeating a store.

## self-review checklist

- [x] Scope concrete. [x] Reference is Windows's records, not w2m's model. [x] Planted faults named. [x] No workaround: the scheduler and suspension are judged in 047 by Sprint 034's own probe (Decision 2026-10-09).

## observed

- **Faults, judged on Windows's records:** on both Intel CPUs (Xeon Platinum 8573C run 38011862284, 8370C run 38011858047), the guest runtime's EXCEPTION_RECORD and CONTEXT equal Windows's for all 1,916 fault cases the VM compiles; the 20,448 completed cases equal in GPRs, path, flags with lazy AF, DF and windows. 17,344 cases (52 SSE classes) are not compiled by the VM yet (Sprint 041). Records `store/runs/vmoracle-1791609837655531000`, `-1791609842914609000`; 6 s per CPU.
- **Commit rule:** a static check over all 8,820 liftable generated encodings; it found POP [m] moving RSP before its store, present since Sprint 029. POP now also computes an RSP-based destination after the increment (SDM POP page, quoted in `lift.py`).
- **Planted faults caught:** POP's old order (`fault r4`), lazy AF from the wrong operand (`AF`), the reconstruction table one byte off (`fault rip`). The card's PUSH plant was outside what Windows's records can show and was replaced (KIT_DIARY #22).
- **Forward rule:** 763 asynchronous interruptions over 300,000,000 iterations, every stop at a block start, the count exact; the backward rule planted corrupts the run.
- **CPUID:** `specs/guest_cpuid.json` 0.1.0 (Intel, x86-64-v2, brand "w2m x86-64-v2 guest"); model, VM and table equal; the VM logs each read. Hand-encoded brand registers were wrong on first writing and caught by decoding them back.
- **Differs from the card:** the PUSH plant became POP; the 039 generator's class set is frozen (`CLASSES_039`) because lifting CPUID changed it; VEH dispatch, the scheduler and suspension are Sprint 047 (user decision 2026-10-09).
- Gates: `scripts/gates.sh` 0 (269 tests, 39 records replay).
