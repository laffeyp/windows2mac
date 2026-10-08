# Sprint 029 — walking skeleton: one Windows PE through every stage

```yaml
---
id: 029
status: closed
phase: 3
pass_kind: functional
---
```

## scope

One no-CRT Windows probe (`probes/skeleton/skeleton.c`: GetCurrentThreadId, SetLastError, GetLastError, GetStdHandle, WriteFile, ExitProcess) travels the whole v0.2 §2 path once. (1) Windows reference: the runner builds and runs it; it writes its own guest-trace body to stdout. (2) Contract `windows.thread.last_error.set_get@0.1.0` on documented evidence. (3) Mac: `src/w2m/loader.py` maps the runner-built PE into guest memory and binds its imports to runtime entry points; `src/w2m/lift.py` lifts each reached x86-64 instruction (decoded by w2m's own decoder) into explicit operations; `src/w2m/runtime.py` executes the lifted operations and services kernel32 calls from documented behavior. (4) Both traces are wrapped, ingested and judged by `judge_traces.py`; the Mac run is profile_kind `model`. Every stage exists once; AArch64 code generation inside the Hypervisor.framework VM (profile `managed_vm`) is the next step and reuses the lifted operations.

## prerequisites

- 028

## context_files

- v0.2 §2, §5, §7, §11.4; review §8 item 2
- Microsoft Learn (retrieved 2026-10-08): GetLastError, SetLastError, WriteFile, GetStdHandle, ExitProcess; Microsoft x64 calling convention (RCX, RDX, R8, R9, 32-byte shadow space, RAX return)
- `src/w2m/pe.py`, `src/w2m/x86dec.py`, `src/w2m/x86ops.py`, `scripts/judge_traces.py`, `.github/workflows/windows-probe.yml`

## signal contract

### Emits

- guest plane, both profiles: GUEST_TRACE_OPENED, GUEST_THREAD_STARTED, GUEST_CALL_RETURNED ×2, GUEST_THREAD_EXITED, GUEST_TRACE_CLOSED
- harness plane: judge tags (SESSION_INIT … COMPARISON_COMPLETED … SESSION_COMPLETED); the Mac run's EXECUTION_STARTED carries profile_kind `model`

### Invariants

- The Mac runtime executes the exact bytes the runner built (`store/binaries/<sha256>.exe`).
- Every instruction the runtime executes was decoded by `x86dec` and lifted; an unlifted instruction stops the run with its address and iclass (no silent skip).
- Each kernel32 entry point cites the page it implements.

## artifact contract

### Files created

- `probes/skeleton/skeleton.c`, `probes/skeleton/probe.json`, `contracts/windows.thread.last_error.set_get/0.1.0.json`, `src/w2m/loader.py`, `src/w2m/lift.py`, `src/w2m/runtime.py`, `scripts/run_skeleton.py`, tests

### Files modified

- `.github/workflows/windows-probe.yml` (per-probe build flags; stdout mode; profile.txt), `scripts/judge_traces.py` (profile_kind)

Hard rule 6 exception: a walking skeleton is by definition one thin slice through every stage; recorded here.

### Command exit codes

- `scripts/gates.sh` → 0; CI success

## observation contract

### Expected runtime signals

- Runner: exit code 0; body of 4 lines; GetLastError returns 1234; judge conforms.
- Mac: same 4 body lines except thread_id and start_address; judge conforms; the run reports how many distinct instructions were lifted and executed.

## done criteria

The runner-built `.exe` runs on the Mac through w2m's own decoder, lifter, runtime and kernel32, and its trace conforms to the same contract as the Windows run.

## self-review checklist

- [x] Scope concrete. [x] Sources named. [x] Emits only 0.5 tags. [x] Observation declared before running. [ ] ≤2 files: exception recorded.

## observed

- Runner run 37856786040: the no-CRT probe built (3,584 bytes; 6 kernel32 imports; no relocations), ran 3 times, exit 0, 46 ms; GetLastError returned 1234.
- The same bytes (`store/binaries/18f721e66de4…exe`) on the Mac: loader at preferred base 0x140000000, 217 distinct instructions in 23 instruction classes reached, 5,153 executed per run, every one decoded by `x86dec`, lifted by `lift.py`, executed by `runtime.py`; six kernel32 calls serviced in the host; exit 0; 6 ms per run.
- Judge, read from the records:
  - runs/judge-1791500576975719000: SESSION_INIT 1, EXECUTION_STARTED 3, EXECUTION_COMPLETED 3, TRACE_INGESTED 3, COMPARISON_COMPLETED 3, SESSION_COMPLETED 1 | verdicts {'conforms': 3} | profile_kind {'windows_reference'}
  - runs/judge-1791500577024299000: SESSION_INIT 1, EXECUTION_STARTED 3, EXECUTION_COMPLETED 3, TRACE_INGESTED 3, COMPARISON_COMPLETED 3, SESSION_COMPLETED 1 | verdicts {'conforms': 3} | profile_kind {'model'}
- The Windows and Mac traces differ only in thread_id (980 vs 4) and start_address (ASLR on Windows; preferred base on the Mac).
- Planted faults: SetLastError that forgets → exit 1 and verdict discrepancy; `mul_hi` computed as `mul_lo` → the digit loop runs off the stack and the run stops with GuestFault, no trace; an unliftable instruction (CPUID) at the entry → LiftError, no exit code.
- Not yet: execution inside the Hypervisor.framework VM. The lifted operations are the input to that back end (next sprint).
- Adding the no-CRT binary to `store/binaries` broke four fixture tests written for CRT-linked probes. One was a real oracle defect: `tests/test_x86ref.py::llvm_insns` took each length from the next row and so dropped the last instruction of `.text`, which CRT probes always follow with padding. Fixed (the last row ends at the section end). Three assertions (≥ 20,000 instructions, DIR64 relocations present, > 400 `.pdata` starts, WaitForSingleObject imported) were CRT facts stated as universal; now scoped to CRT-linked probes, with the skeleton's own facts asserted (KERNEL32 only, no relocations).
