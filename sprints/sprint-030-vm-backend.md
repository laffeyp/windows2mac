# Sprint 030 — AArch64 back end in the Hypervisor.framework VM

```yaml
---
id: 030
status: closed
phase: 3
pass_kind: functional
---
```

## scope

The skeleton's lifted operations (`src/w2m/lift.py`) are compiled to AArch64 and run inside a Hypervisor.framework VM (profile_kind `managed_vm`). The split:

- `tools/vm_monitor/vm_monitor.c` is a small VM monitor. It holds no policy. It maps one shared memory file at a fixed IPA, sets system registers and general registers on command, runs the vCPU, and reports each exit (HVC immediate, PC, x0–x30, ESR_EL1, FAR_EL1, ELR_EL1) as one text line on stdout. A watchdog thread cancels a run that exceeds its time limit.
- `src/w2m/a64_encoder.py` encodes the AArch64 instructions the back end uses.
- `src/w2m/vm_backend.py` holds the rest:
  - it builds the guest's stage-1 page tables, so that every x86 virtual address maps to the same virtual address in the guest (4 KB granule, 48-bit VA);
  - it compiles one x86 basic block at a time from lifted operations;
  - it keeps an in-guest dispatch table that maps x86 addresses to compiled code;
  - it services kernel32 calls in the host through the same implementation the `model` runtime uses (`src/w2m/kernel32.py`, moved out of `runtime.py`).

The register map gives x86 GPR i to AArch64 x_i. CF, ZF, SF, OF and PF live in x19–x23, and x24 holds a mask of the flags currently undefined. The x86 condition reads follow the order of `runtime.py`, and reading an undefined flag exits to the host as UndefinedFlag, as the model does.

## prerequisites

- 029

## context_files

- v0.2 §2, §5, §8.2; Decision 2026-10-08 (first backend `model`, VM next)
- Arm ARM (DDI 0487) A64 encodings; checked against the host assembler (clang), not transcribed alone
- Apple Hypervisor.framework headers (hv_vcpu_run, hv_vcpus_exit, hv_vm_map, HV_SYS_REG_*); `probes/page_geometry/page_geometry.c` (stage-1 tables, MAIR/TCR/SCTLR values measured in Sprint 012)
- `src/w2m/lift.py`, `src/w2m/runtime.py`, `src/w2m/loader.py`, `scripts/run_skeleton.py`

## signal contract

### Emits

- guest plane: the skeleton's own 4 body lines, wrapped as in Sprint 029
- harness plane: judge tags; EXECUTION_STARTED carries profile_kind `managed_vm`

### Invariants

- The VM executes the exact bytes the runner built (`store/binaries/<sha256>.exe`).
- Every x86 instruction the VM executes was decoded by `x86dec`, lifted by `lift.py`, and compiled by `vm_backend.py`; nothing runs through the host interpreter.
- In lockstep mode, after every x86 instruction the VM's 16 GPRs, RIP, defined flags and undefined-flag mask equal the model runtime's.

## artifact contract

### Files created

- `tools/vm_monitor/vm_monitor.c`, `tools/vm_monitor/entitlements.plist`, `src/w2m/a64_encoder.py`, `src/w2m/vm_backend.py`, `src/w2m/kernel32.py`, `tests/test_a64_encoder.py`, `tests/test_vm_backend.py`
- Names changed during the sprint at the user's instruction ("name things what they are", chat 2026-10-08): `hvrun` → `vm_monitor`, `a64` → `a64_encoder`, `vm` → `vm_backend`, `k32` → `kernel32`, and in code `Asm` → `Assembler`, `POOL` → `TEMP_REGS`, `FC`… → `REG_CF`…, `UMASK` → `REG_UNDEFINED_FLAGS`, `_zsp` → `_set_zf_sf_pf`, `lift._B` → `_OpBuilder`, `Runtime.k32` → `Runtime.kernel32`.

### Files modified

- `src/w2m/runtime.py` (kernel32 moved out; a `step()` method), `src/w2m/loader.py` (a region may be a memoryview of VM memory), `src/w2m/lift.py` (one rename), `scripts/run_skeleton.py` (VM runs, lockstep and state differential before them), `tests/conftest.py` (`hvf` tests skip off macOS without failing CI), `tests/test_runtime.py` (rename)

Hard rule 6 exception: a back end is a monitor, an encoder and a compiler together; none can be judged alone. Recorded here.

### Command exit codes

- `scripts/gates.sh` → 0; CI success (VM tests skip on Linux, marked `hvf`)

## observation contract

### Expected runtime signals

- Encoder: every encoding the back end emits equals clang's assembly of the same text.
- Lockstep: 0 state differences over the skeleton's full run (5,153 instructions).
- Normal mode: exit 0; stdout equal to the model's except thread_id; judge conforms 3 of 3 under profile_kind `managed_vm`; the same instruction count (5,153) and distinct addresses (217) as the model.
- Planted faults: a wrong flag rule in the compiler shows up as a lockstep difference at a named x86 address.

## done criteria

The runner-built `.exe` runs inside the Hypervisor.framework VM as AArch64 code compiled from the lifted operations, its trace conforms to the same contract, and lockstep comparison with the model finds no state difference.

## self-review checklist

- [x] Scope concrete. [x] Sources named. [x] Emits only 0.5 tags. [x] Observation declared before running. [ ] ≤2 files: exception recorded.
- [ ] The random-state differential was not in the declared observation contract; it was added after the planted carry fault passed lockstep. Recorded in observed.

## observed

- Encoder: 67 forms in `tests/test_a64_encoder.py` equal clang's assembly on the first run.
- Monitor facts measured before use (scratch experiment, then relied on): on an HVC exit the vCPU PC is the address after the `hvc` (hvc at 0x10000004, exit PC 0x10000008); a file-backed `MAP_SHARED` mapping is accepted by `hv_vm_map`; the watchdog cancels a `b .` loop at its 200 ms limit.
- First VM run stopped after 68 instructions with an undefined-instruction exception (EC 0x0) in the block for a JNZ at 0x140001175: a conditional branch was treated as ending control flow, so its not-taken path ran into the next block's bytes. Fixed (only an unconditional branch omits the fall-through tail).
- Normal mode: exit 0, 5,153 instructions executed, 217 distinct addresses, the same six kernel32 calls, stdout byte-identical to the model's; 43 blocks compiled; 16-18 ms per run including the monitor's start, against 5 ms for the model.
- Lockstep: 5,159 comparisons (5,153 instructions and 6 kernel32 returns), 0 differences in RIP, GPRs, flags or the undefined-flag mask; guest memory equal at the end; 0.16 s.
- A planted carry fault (SUB's borrow compiled as signed less-than) passed the full lockstep run: every SUB and CMP on the skeleton's path has operands of equal sign. The random-state differential was added in response: each of the 217 instructions from 40 random states (sign and width boundary values, random bits, all flags defined, memory operands pointed into the stack, 32 bytes around every effective address seeded with the same random bytes and compared). Clean: 8,680 cases, 0 differences, 0.36 s. Planted carry fault: found at case 1 of 0x140001171. Planted 32-bit-store-as-64-bit fault: first missed (the extra bytes were zeros over a zero stack window, and the skeleton's 32-bit stores go to .data, not the stack); found after seeding and comparing bytes around each effective address, at 0x14000117b. Planted parity fault: found by lockstep.
- Fault paths: a store to address 0 stops the VM with `GuestFault: access at 0x0` (data abort through the guest vector table); CPUID at the entry stops it with LiftError.
- Judge, read from the records:
  - runs/judge-1791505504109764000: SESSION_INIT 1, EXECUTION_STARTED 3, EXECUTION_COMPLETED 3, TRACE_INGESTED 3, COMPARISON_COMPLETED 3, SESSION_COMPLETED 1 | verdicts {'conforms': 3} | profile_kind {'windows_reference': 3}
  - runs/judge-1791505504702339000: same counts | verdicts {'conforms': 3} | profile_kind {'model': 3}
  - runs/judge-1791505504772307000: same counts | verdicts {'conforms': 3} | profile_kind {'managed_vm': 3}
- Gates: `scripts/gates.sh` 0, 177 passed.
- Not yet: the monitor shares the guest's address space and privilege level (EL1); a guest store to MONITOR_VA would reach the dispatch table. Flags are computed eagerly after every flag-writing instruction. One thread. These are the next widenings, recorded, not hidden.
