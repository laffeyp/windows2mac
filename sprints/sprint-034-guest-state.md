# Sprint 034 — guest state, TEB, PEB and CONTEXT, measured on Windows

```yaml
---
id: 034
status: closed
phase: 3
pass_kind: functional
---
```

## scope

The runtime architecture (BLACKBOARD 2026-10-09) puts Windows-visible state in guest memory at Windows layouts. This sprint fixes those layouts from Windows itself and writes them down as one spec the guest runtime (Sprint 035) is built from:

1. `probes/guest_state/guest_state.c` (no C runtime), run on the windows-2025 runner, prints one JSON object per line:
   - **layout** — `offsetof` and `sizeof` from the runner's SDK headers: `CONTEXT` (winnt.h: ContextFlags, MxCsr, SegGs, EFlags, Rax-R15, Rip, FltSave, Xmm0-15, the XMM_SAVE_AREA32 fields ControlWord, StatusWord, TagWord, MxCsr), `TEB` and `PEB` as winternl.h declares them (ProcessEnvironmentBlock, TlsSlots, TlsExpansionSlots; Ldr, ProcessParameters).
   - **teb** — the running thread's TEB read through `gs:` against the API that reports the same value: `gs:[0x30]` (Self) = `NtCurrentTeb()`; `gs:[0x60]` = the PEB; `gs:[0x48]` = `GetCurrentThreadId()`; `gs:[0x40]` = `GetCurrentProcessId()`; `gs:[0x68]` = the value just passed to `SetLastError`; `gs:[0x08]`/`gs:[0x10]` = `GetCurrentThreadStackLimits` high/low; `gs:[0x1480 + i*8]` = `TlsGetValue(i)` for a slot below 64, and `gs:[0x1780]` → expansion array `[i - 64]` for a slot at 64 or above (the two forms Mono's JIT emits, `mono_amd64_emit_tls_get`); PEB `+0x10` = `GetModuleHandleW(NULL)`.
   - **fault** — a vectored exception handler (`AddVectoredExceptionHandler(1, ...)`, as Mono registers) catches an access violation from `mov rcx, [rax]` with RAX = 0 and RBX, RSI, RDI, R12-R15, RBP and XMM6 set to known values; it records ExceptionCode, ExceptionAddress, the faulting address in ExceptionInformation[1], and CONTEXT's registers, EFlags, MxCsr, the x87 control word and SegGs; it sets RIP past the 3-byte instruction and RCX to a marker, and the probe checks the marker arrived (CONTEXT write-back, as Mono's handler relies on).
   - **suspend** — a thread spinning with RBX and R12 set to known values is suspended with `SuspendThread`; `GetThreadContext(CONTEXT_FULL)` must return those values and a RIP inside the spin loop; `SetThreadContext` then changes RBX and the thread, resumed, must report the new value (Boehm GC and Mono's thread layer use all three).
2. `specs/guest_state.json`: the guest state spec. w2m's per-thread state block (field, offset, size), the Windows CONTEXT, TEB and PEB fields the guest runtime must hold, each with its offset, its source (header or observation) and the runner record that confirms it, and the reconstruction-table format (host PC range → guest RIP and, per guest register, its host register or its slot in the state block).
3. `scripts/check_guest_state.py`: compares the spec with the runner's output, field by field, and records one substrate run (COMPARISON_COMPLETED, contract `windows.guest_state.layout`, relation `exact`; a DISCREPANCY_REPORTED/ROUTED per differing field).

## prerequisites

- 033; runtime architecture Decision 2026-10-09

## context_files

- Microsoft Learn (MicrosoftDocs/sdk-api, branch docs): `winternl/ns-winternl-teb.md` (commit 7659b39b6294, "This structure may be altered in future versions of Windows"), `winternl/ns-winternl-peb.md`, `winnt/ns-winnt-context.md`, `processthreadsapi/nf-processthreadsapi-getthreadcontext.md`, `errhandlingapi/nf-errhandlingapi-addvectoredexceptionhandler.md`
- Unity-Technologies/mono `unity-2019.4-mbe`: `mono/mini/mini-amd64.c` (`mono_amd64_emit_tls_get`), `mono/mini/exceptions-amd64.c` (VEH), `mono/utils/mono-threads-windows.c` (suspend and context)
- `probes/skeleton/skeleton.c` (no-CRT probe pattern), `.github/workflows/windows-probe.yml`

## signal contract

### Emits

- harness plane: SESSION_INIT, COMPARISON_COMPLETED (relation `exact`), DISCREPANCY_REPORTED and DISCREPANCY_ROUTED per differing field, SESSION_COMPLETED.

### Invariants

- No vocabulary change.
- Every spec field with source "observation" names the runner record that confirms it.

## artifact contract

### Files created

- `sprints/sprint-034-guest-state.md`, `probes/guest_state/guest_state.c`, `probes/guest_state/probe.json`, `specs/guest_state.json`, `scripts/check_guest_state.py`, `tests/test_guest_state.py`, `store/probe-runs/<run id>/` (the runner's output)

### Command exit codes

- `scripts/gates.sh` → 0

## observation contract

### Expected runtime signals

- Runner: the probe exits 0 three times (3 runs of one build); the layout lines are equal across runs; every teb, fault and suspend check reports equal.
- Expected values, from the sources before the run: CONTEXT size 0x4D0, Rip at 0xF8, EFlags at 0x44, MxCsr at 0x34, FltSave at 0x100, Xmm0 at 0x1A0; TEB ProcessEnvironmentBlock 0x60, TlsSlots 0x1480, TlsExpansionSlots 0x1780; ExceptionCode 0xC0000005 with ExceptionInformation[1] = 0; MxCsr 0x1F80 and x87 control word 0x27F at the fault (Windows x64 defaults).
- check_guest_state: 0 discrepancies if the spec is right; any difference is a spec defect, reported and routed `contract_revision`.

## done criteria

Every Windows-visible field the guest runtime will hold has an offset confirmed on the runner, the spec says where each guest register lives at any instruction boundary, and the comparison is on the record.

## self-review checklist

- [x] Scope concrete. [x] Sources named. [x] No new vocabulary. [x] Observation declared before running, with expected values. [x] File count recorded (7).

## observed
- Runner run 37989842529 (windows-probe.yml, probe `guest_state`, 3 executions of one build, gha-windows-2025-x64-10.0.26100.33438-Server): exit 0 three times (0.23-0.26 s); 41 checks equal in every run; layout lines identical across runs. Output and the probe binary in `store/probe-runs/37989842529/`.
- Every expected value held: CONTEXT 0x4D0 bytes; ContextFlags 0x30, MxCsr 0x34, SegCs 0x38, SegGs 0x40, EFlags 0x44, Rax 0x78 through R15 0xF0, Rip 0xF8, FltSave 0x100, Xmm0 0x1A0, Xmm6 0x200, Xmm15 0x290, VectorRegister 0x300; XMM_SAVE_AREA32 512 bytes with MxCsr at 0x18 and XmmRegisters at 0xA0; EXCEPTION_RECORD 0x98 bytes, ExceptionInformation at 0x20; TEB ProcessEnvironmentBlock 0x60, TlsSlots 0x1480, TlsExpansionSlots 0x1780; PEB Ldr 0x18, ProcessParameters 0x20, SessionId 0x2C0.
- Observed through gs: and equal to the API: Self 0x30, PEB 0x60, ClientId process 0x40 and thread 0x48, LastErrorValue 0x68 (32-bit), StackBase 0x08, StackLimit 0x10 inside the reserve; a TLS slot below 64 in TlsSlots and a slot at 64 or above through the TlsExpansionSlots pointer (Mono's two forms); PEB+0x10 = the image base.
- Fault: a first vectored handler saw ExceptionCode 0xC0000005, two parameters (read, address 0), RIP at the faulting instruction, RBX, RSI, RDI, R12-R15 and XMM6 as set, MxCsr 0x1F80 in both places, x87 control word 0x27F, IF set; setting RIP past the instruction and RCX to a marker resumed the thread with the marker in RCX.
- Suspend: SuspendThread returned prior count 0; GetThreadContext(CONTEXT_FULL) returned the spinning thread's RBX and R12 and a RIP inside the spin loop; SetThreadContext changed RBX and the resumed thread left the loop with the new value.
- `specs/guest_state.json` 0.1.0: state block 0x380 bytes (GPRs in encoding order, RIP, materialized RFLAGS, undefined-flag bits, MXCSR, x87 control/status/tag, FS and GS bases, TEB, thread state, suspend count, XMM0-15, YMM upper halves, ST0-7); host registers (GPR i in x_i, flags x19-x23 and x24, XMM i in v_i, YMM i upper half in v(16+i)); reconstruction-table format; the Windows fields above with sources; CONTEXT-from-state map. SegCs 0x33 and SegGs 0x2B are written from memory and marked unobserved.
- `scripts/check_guest_state.py` against the run: 62 fields, 0 differences; record `store/runs/gueststate-1791579238609115000` (SESSION_INIT, COMPARISON_COMPLETED conforms, SESSION_COMPLETED). Controls: a planted CONTEXT.Rip 0xF0, a planted LastErrorValue 0x6C, a planted MXCSR default and a planted failed run are each caught (`tests/test_guest_state.py`).
- Gates: `scripts/gates.sh` 0.

