# Sprint 053 — structured exceptions

```yaml
---
id: 053
status: open
phase: 5
pass_kind: functional
---
```

## scope

Sprint 052 left the frontier at 20,785,191 instructions, at `KERNEL32.dll!RaiseException` on mono's thread 8 (code 0x406D1388, three arguments: MSVC's thread-naming convention, caught by a `__try`/`__except` in mono). The game's Windows run (`store/game-runs/38113342549`, imports table) counts one RaiseException and one RtlUnwindEx, both from mono: the frame's `__except` takes the exception, and mono's statically linked `__C_specific_handler` unwinds to it. The three modules link the C runtime statically: every language-specific handler is the module's own code, and the only exception functions the game imports are KERNEL32's. On build 26100 kernel32's Rtl* exception functions are its own exports, not forwarders (`specs/windows_exports.json` `forwarded`); AddVectoredExceptionHandler and RemoveVectoredExceptionHandler forward to ntdll.

1. **Dispatch in the guest runtime.** RaiseException builds the EXCEPTION_RECORD (flags 0x80, EXCEPTION_SOFTWARE_ORIGINATE, as Sprint 049's dispatch rows show) and the CONTEXT of its return point, then dispatches: vectored handlers front to back, then the frames, found by each image's `.pdata` (and the growable tables of Sprint 049), unwound by their UNWIND_INFO (versions 1 and 2, chained entries, epilogues), each language-specific handler called with a DISPATCHER_CONTEXT; with no taker, the top-level filter (SetUnhandledExceptionFilter), then the process ends with the code. Handlers are x86 code: the runtime calls each through an x86 stub and resumes when it returns, so a handler may itself call RtlUnwindEx or raise.
2. **RtlUnwindEx** walks from its caller to the target frame, calling each frame's handler with EXCEPTION_UNWINDING (EXCEPTION_TARGET_UNWIND at the target), and resumes the target frame's context with RAX the return value and RIP the target; an x86 stub restores the full context (GPRs, RFLAGS, XMM0-15, MXCSR).
3. **Faults dispatch too.** Sprint 040's access violations and SIMD exceptions, which end the process today, enter the same dispatcher, so mono's vectored handler and a module's `__except` see them.
4. **RtlCaptureContext, RtlLookupFunctionEntry (images' `.pdata`), RtlVirtualUnwind, RtlPcToFileHeader** as services, since the dispatcher holds them: the game imports each (no call in the 180 s Windows run).
5. **The rows.** An exceptions probe on Server 2025 and 2022, its own `__C_specific_handler` as mono's static CRT has: `__except` with a filter, CONTINUE_EXECUTION, CONTINUE_SEARCH to an outer frame, `__finally` run during the unwind (the order of filter, finally, except), a noncontinuable exception continued (STATUS_NONCONTINUABLE_EXCEPTION and its chained record), a vectored handler before the frames, the raise three calls deep through frames with a frame pointer, a large allocation and saved XMM registers, an access violation caught, the Rtl* queries of item 4 on the probe's own functions, and last the top-level filter. Sprint 049's two dispatch rows of the vectored_handlers family are included.

Not in this sprint, each with its reason: `STATUS_UNWIND_CONSOLIDATE` (C++ catch blocks: UnityPlayer raises no exception in the Windows run), collided unwinds, RtlInstallFunctionTableCallback (mono uses the growable tables, trace seq 671 on), UnhandledExceptionFilter called directly (no call in the run); each stops the run by name when reached.

## prerequisites

- 052 (closed partial: CI run 38128014124 on 6d522115; main 803475f7)

## context_files

- `tools/guest_runtime/runtime.c` (Sprint 040's fault path), `vectored_handlers.c`, `exception_filter.c`, `function_tables.c`, `runtime.h`; `src/w2m/vm_backend.py` (the bridge, the exception entry); `specs/windows_exports.json`; Microsoft Learn: x64 exception handling (UNWIND_INFO, UNWIND_CODE, the language-specific handler and DISPATCHER_CONTEXT), RtlUnwindEx, RtlVirtualUnwind, RtlLookupFunctionEntry, RaiseException; winnt.h SCOPE_TABLE_AMD64

## signal contract

### Emits

- Per family: the item run. Per frontier run: EXECUTION_STARTED, then SERVICE_UNIMPLEMENTED or EXECUTION_COMPLETED.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- created: `probes/exceptions/`, the exceptions contract, `tools/guest_runtime/exceptions.c`
- modified: `tools/guest_runtime/runtime.c`, `function_tables.c`, `vectored_handlers.c`, `runtime.h`; `src/w2m/vm_backend.py` (the stubs, the bridge's KERNEL32-to-ntdll service, the exception entry's return to the guest); `scripts/family_run.py` (the family; vectored_handlers' dispatch rows included)
- gates 0 and CI `success` on the closing commit

## observation contract

- **The exceptions family equals Windows** on its probe's rows on Server 2025 and 2022, with plants of named reach; the vectored_handlers family equals Windows with its dispatch rows.
- **mono's RaiseException is caught by its own `__except`**: the frontier passes 20,785,191 instructions and UnityPlayer's six CoCreateInstance calls (Sprint 052's criterion); the next stop is recorded.
- **The fault oracle holds**: Sprint 040's fault cases still build Windows's EXCEPTION_RECORD and CONTEXT, now reached through dispatch.

## done criteria

The exceptions family accepted, vectored_handlers re-accepted with dispatch; the frontier past the six CoCreateInstance calls.

## observed
