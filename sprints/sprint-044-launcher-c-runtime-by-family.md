# Sprint 044 — the launcher's C runtime surface, by family

```yaml
---
id: 044
status: closed
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

## observed

- **The frontier moved** from 13 instructions (stop `GetSystemTimeAsFileTime`) to 19,084: the launcher's C runtime start-up completes in w2m and the run stops at the import `UnityPlayer.dll!UnityMain` (SERVICE_UNIMPLEMENTED, guest address 0x1400011F2). On the way it stopped at, in order: GetSystemTimeAsFileTime, the module calls, the critical sections, TLS and FLS, the heap, the standard handles, GetACP, MultiByteToWideChar's MB_PRECOMPOSED form, LCMapStringEx (reached through GetProcAddress, not the imported LCMapStringW), RET imm16, GetEnvironmentStringsW, SetUnhandledExceptionFilter.
- **Families accepted, each on its own record** (runner rows on Server 2025 and 2022, AMD EPYC 7763, 9V45, 9V74 and Intel Xeon 8370C, 8573C; every run's two executions and every runner agreed): process queries; modules; critical sections; thread storage; heap; console 0.2.0 (GUI start without handles; GetCommandLineA added at the close); code pages 0.2.0 (3,827 rows: code page 1252 per byte, 452 best-fit units, 3,072 malformed UTF-8 inputs, the launcher's batched forms); environment, as two families over one probe (a set block, an empty block); exception filter. Every service the launcher calls before UnityMain has rows in a family (checked against the call counts of the frontier run).
- **Windows against its pages:** malformed UTF-8 follows the Unicode Standard's Table 3-7 except that after E0, ED, F0 or F4 an out-of-range continuation byte joins the U+FFFD (WHATWG leaves it; the plant of WHATWG's rule changes the 37 rows named); HeapAlloc's failure sets ERROR_NOT_ENOUGH_MEMORY though its page says it does not; IsValidCodePage(CP_ACP) is FALSE; an empty environment block comes back as two NULs.
- **Planted faults:** one or more per family, reach named first. Three named reaches were wrong and are recorded here: a critical-section plant (LockCount kept on the last Leave) named 3 rows and stopped the run; a heap plant (unaligned blocks) stopped the run on heap.c's own check; the heap's size-is-capacity plant named 2 rows and changed 4. A heap plant that ignored HEAP_ZERO_MEMORY changed nothing on a fresh, zero arena: a blind spot, closed by a probe row that zeroes a dirty, freed block. Reaches may be fnmatch patterns (452 best-fit rows).
- **Instruction classes:** sets 044 (11 integer classes), 044b (REP STOS/MOVS), 044c (ADC, BSF, BSR), 044d (the SSE2 integer block, MXCSR access, SFENCE, PREFETCHNTA; 5,149 cases) and 044e (RET imm16, 256 cases), each judged on an Intel runner by the model and the VM with no difference; releases semantics-044 to semantics-044e; 183 of 183 classes accepted (163 at the sprint's middle). XGETBV is judged by the guest profile (#UD; the runners set OSXSAVE). A class may span sets (RET_NEAR: 039's C3, 044e's C2).
- **Hot services:** over the process-queries probe, 100,005 QueryPerformanceCounter calls make no host exit (four exits in all: the wall clock once, the module path, the rows, the exit); over the thread-storage probe, TlsGetValue makes none (`tests/test_hot_services.py`). Over the frontier run the host serves 17 requests, 1 system time and 16 module lookups.
- **The model family (D7):** kimi-k2.7-code:cloud in substrate's session topology wrote environment.c; both environment item runs accepted it (16 model calls, 316,810 prompt and 1,288 completion tokens, 2 judgments; `store/runs/model-family-environment-kimi-k2.7-code-cloud-1791629810134921000.json`). Where the rows are silent it differs from the agent's file (an empty block's second unit unwritten; NULL from a full arena and NULL arguments pass).
- **Census:** launcher 124 classes reached, 101 lifted, 23 missing (VEX forms behind AVX, INT3, INT); UnityPlayer.dll 460 reached, 163 lifted, 6 partly, 291 missing, SHUFPS (23,568), PXOR (9,439) and INT3 (6,196) first (`store/census/class-census-2026-10-10.json`).
- **Defects found and fixed:** a timing row of process queries failed 4 of 30 runs (a translation stall inside the measured window; the window now runs twice); the ring's per-API counters silently skipped ids 64 and above; a refused argument form ended the frontier as a fault, not SERVICE_UNIMPLEMENTED; runner binaries were not reproducible (16 runs, up to 16 binaries; `/Brepro` now gives one per image); a committed lint error (E741) left CI red until found; one dispatch built an uncompiled probe edit's predecessor and was cancelled. At the close the heap family's acceptance was stale (heap.c changed after its item run); every family reran on the final runtime, all accepted, and `tests/test_family_runs.py` now holds each acceptance to its file's sha256.
- Gates and CI: see the closing commit.
