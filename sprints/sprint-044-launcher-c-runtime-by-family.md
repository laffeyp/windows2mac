# Sprint 044 — the launcher's C runtime surface, by family

```yaml
---
id: 044
status: open
phase: 5
pass_kind: functional
---
```

## scope

The launcher `Hail to the Rainbow.exe` runs 13 instructions under w2m and stops at `KERNEL32.dll!GetSystemTimeAsFileTime` (frontier, 2026-10-10, 0.29 s). It imports 66 functions: 65 from KERNEL32, of which w2m serves 10, and `UnityPlayer.dll!UnityMain`. The 55 missing are the statically linked C runtime's start-up and support surface (review 2026-10-10 §2). Sprint 042 built the pipeline for one Windows function family; this sprint runs it per family until the launcher's start-up completes (plan reordered 2026-10-10, review D3/D5/D6).

1. **Families.** Functions that share state are judged together, as 042 judged the file family: the heap (GetProcessHeap, HeapAlloc, HeapFree, HeapSize, HeapReAlloc); TLS (TlsAlloc, TlsGetValue, TlsSetValue, TlsFree); critical sections (four); code pages and strings (GetACP, GetOEMCP, GetCPInfo, IsValidCodePage, MultiByteToWideChar, WideCharToMultiByte, GetStringTypeW, LCMapStringW); command line and environment (four); modules (GetModuleHandleW, GetModuleHandleExW, GetModuleFileNameW, LoadLibraryExW, GetProcAddress, FreeLibrary); unwinding and failure (nine); console and files (nine); and one probe for the stateless queries (GetSystemTimeAsFileTime, GetCurrentProcessId, IsProcessorFeaturePresent, GetStartupInfoW, InitializeSListHead, IsDebuggerPresent, …). Each family is one no-CRT probe on the runner, one contract from its rows, guest C in `tools/guest_runtime/`, and one item run. The order follows the frontier: a family is taken when start-up reaches one of its functions.
2. **Hot services stay in the guest** (Decision 2026-10-09 "hot imports stay in the guest", quoted per the 2026-10-10 rule; review D4): QueryPerformanceCounter from CNTVCT_EL0 scaled to 10 MHz, TlsGetValue and GetCurrentThreadId with no host exit. The lockstep comparison, which fixes the clock today, keeps a fixed counter.
3. **A family by a cloud model** (review D7): at least one family is written by a model in substrate's session topology, judged by the same item run, beside the agent's.
4. **The class census:** the instruction classes the launcher and UnityPlayer reach without a rule, recounted (the 323 of the progress review predate 041).

## prerequisites

- 042 (the pipeline), 043 (closed; CI 38037257526 success on b5f5bb1), Decision 2026-10-09 hot imports, review 2026-10-10

## context_files

- `scripts/file_run.py`, `probes/create_file/`, `tools/guest_runtime/`, `src/w2m/file_bridge.py`, `scripts/run_frontier.py`, `scripts/measure_file_generation.py`, Microsoft Learn pages of each family's functions

## signal contract

### Emits

- Per family: the 042 item run (SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED per runner binary, one decision, SESSION_COMPLETED); per frontier run: EXECUTION_STARTED, SERVICE_UNIMPLEMENTED at the stop.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- created: a probe, a contract and a guest C file per family; the family item runs; frontier records
- modified: `tools/guest_runtime/runtime.{h,c}`, `src/w2m/vm_backend.py` (SERVICES), `scripts/file_run.py` generalized to any family
- gates 0 and CI `success` on the closing commit

## observation contract

- **The frontier moves:** the launcher's stop is the call to `UnityPlayer.dll!UnityMain`; every function start-up calls before it is served.
- **Each family equals Windows** on every recorded row, in the VM, on its own record, with one planted fault per family whose reach is named first.
- **Hot services:** QueryPerformanceCounter, TlsGetValue and GetCurrentThreadId make no host exit (the transport's request count is 0 for them over a frontier run).
- **The model family:** its session record (model calls, tokens, tool calls) and its item run's decision.

## done criteria

The launcher's C runtime start-up completes under w2m, and every function it called is judged against Windows on its own family's record.
