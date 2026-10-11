# Sprint 053 — structured exceptions

```yaml
---
id: 053
status: closed (partial)
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

- **Closed partial:** the exceptions half of the done criteria is met; the frontier half is not. mono's thread-naming RaiseException is caught by its own `__except` as on Windows (one RaiseException, one handler call, one RtlUnwindEx, the counts of the game's Windows run), and the frontier runs 41,315,682 instructions, twice 052's 20,785,191, with 20 threads created; it stops at `KERNEL32.dll!VerSetConditionMask` (`store/runs/frontier-643a6283510c-1791745663100836000`, the same stop on the closing code as on the first run), Windows trace seq 1524, before the six CoCreateInstance calls at seq 1705.
- **Why the criterion moves again:** between seq 1524 and 1705 the held trace lists 20 functions w2m does not serve: the version check (VerSetConditionMask 5, ntdll RtlVerifyVersionInfo), the display (GetSystemMetrics 4, EnumDisplayDevicesA 3, EnumDisplayMonitors, GetMonitorInfoA, MonitorFromWindow, shcore GetDpiForMonitor) and the game window (RegisterClassExW, RegisterClassW, CreateWindowExW, DefWindowProcW 3, SetWindowLongPtrW, GetClientRect, LoadIconA, LoadCursorA), with GetSystemTimePreciseAsFileTime, RegOpenKeyExW, SHELL32 DllGetVersion and FindFirstFileExW; WINMM timeGetTime follows at 1707. Sprint 052 moved its criterion here without listing them; the listing, made now from the trace, is the next sprint's scope (KIT_DIARY #43).
- **The exceptions family equals Windows** (contract windows.exception.exceptions 0.1.0, 10 rows, runs 38165931122 and 38165933249, Server 2025 and 2022 equal) with seven plants of named reach: the software-origin flag, vectored handlers skipped, termination handlers skipped, the unwind's return value, a noncontinuable exception continued, UWOP_SET_FPREG ignored (it breaks every row after the frame-pointer frame), no top-level filter.
- **vectored_handlers 0.2.0 equals Windows** with Sprint 049's two dispatch rows included (8 rows) and a plant the dispatch rows now catch (First handlers put at the back), which 0.1.0's open question said no row could.
- **Faults dispatch:** the exception entry returns to the guest after w2m_exception; the fault oracle of run 38011862284 judges 39,708 cases (5,485 faults) with no class differing, and 436 of 436 classes stay accepted in model and VM.
- **Windows facts the rows fixed:** STATUS_NONCONTINUABLE_EXCEPTION carries flags 0x81 and no chained record; an access violation's flags are 0; clang's `__try` takes exceptions only from the calls in its body (a fault in its own body needs `-fasync-exceptions`) and gives `.pdata` only to functions with a `__try` unless built with unwind tables, without which Windows's RtlUnwindEx raised STATUS_BAD_STACK walking out of the probe's handler (run 38165571800).
- **Built:** `tools/guest_runtime/exceptions.c` (dispatch, the unwinder for UNWIND_INFO versions 1 and 2 with chained entries and epilogues, RtlUnwindEx, RaiseException, RtlCaptureContext, RtlVirtualUnwind, RtlPcToFileHeader), RtlLookupFunctionEntry over images' `.pdata`, `src/w2m/exception_stubs.py` (return, call and restore stubs, written at the first exception), host request 31, the bridge serving a KERNEL32 Rtl* import with the ntdll service, the runtime's memcpy; the exception area marks only an exception that ends the process.
- **Defects found and fixed on the way:** the reference traversal took an address the probe's code computes (`queries + 0x400`, one LEA) for a function entry and decoded from inside an instruction; an address taken inside a `.pdata` function but not at its start is no longer a root (`src/w2m/pe.py` keeps EndAddress). Three binary tests told a CRT-linked probe by having `.pdata` at all; they now use its size (448 and 451 entries against the exceptions probe's 23). The table search in function_tables.c had been copied for the dispatcher; RtlLookupFunctionEntry now calls the one search. Ten families re-recorded on the changed host files, all conforming.
- **Closed on CI run 38166790366** (success on 539c238c).
