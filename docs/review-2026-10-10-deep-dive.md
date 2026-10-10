# Review 2026-10-10 — deep dive, back to basics

Scope: the whole repository at c657bda (Sprint 043 card, open), with Sprints 034–042 read in full and every headline claim below checked on this machine today. No earlier review's conclusions are assumed.

## Method

Ran `scripts/gates.sh`, the game's frontier (`scripts/run_frontier.py` on `Hail to the Rainbow.exe`), and an import census of the game's three main images against `vm_backend.SERVICES`. Read the CI history (`gh run list`), the substrate clone's branches, the guest runtime source, `docs/plan.md`, BLACKBOARD Decisions of 2026-10-09 and 2026-10-10, KIT_DIARY #19–#27, and the Sprint 039–043 cards.

## 1. Verdict

w2m has built, in three days, a translator core whose correctness rests on evidence and not on review: a decoder confirmed against XED on the game's code, 141 instruction classes each accepted on its own record against Intel CPUs, Windows's CONTEXT reproduced at faults, and one Windows function family (CreateFileW and five companions) matching Windows on 188 recorded rows. The method is sound and the records are honest.

The game has not moved. It runs 13 instructions under w2m and stops at `KERNEL32.dll!GetSystemTimeAsFileTime`, exactly where it stopped on 2026-10-09. The pipeline built in Sprint 042 is the machine for moving it, and the roadmap sends the next sprint (043) to rewrite record plumbing instead. Two defects need fixing first: CI has failed on every push since 3710f99, and the roadmap's milestone-A judgment for Sprint 045 depends on Sprint 046.

## 2. What w2m is today (measured 2026-10-10)

| Part | State |
|---|---|
| Gates (local) | `scripts/gates.sh` 0: 306 tests in 31.7 s; 441 records replay at level 2 |
| Gates (CI) | failed on the last four pushes to main (38033959901 → 38034552789): `uv sync` cannot find `./substrate` |
| Decoder | exit criterion (d) met on the game: 718 of 718 rows XED-confirmed (Sprint 036) |
| Instruction classes | 141 accepted, each on its own causal record (`store/runs/class-runs.json`); hand-drafted rules in `src/w2m/rules/` (92 modules) |
| Semantics evidence | sets 039 (39,708 cases, 5,485 faults), 041 (13,243), 042 (618): model and VM equal Intel Xeon 8573C and 8370C on every case |
| Guest state | CONTEXT and EXCEPTION_RECORD at a fault equal Windows's on 1,916 cases; commit and forward rules; lazy AF/DF; CPUID profile x86-64-v2 |
| Guest runtime | 728 lines of C in `tools/guest_runtime/` (runtime.c, last_error.c, file.c, runtime.h) |
| Windows services | 12: CloseHandle, CreateFileW, DeleteFileW, ExitProcess, GetCurrentThreadId, GetLastError, GetStdHandle, QueryPerformanceCounter, ReadFile, SetFilePointerEx, SetLastError, WriteFile |
| Images | one at a time; no relocation, no DllMain, no imports between images |
| Threads, exception dispatch, JIT region, graphics | none |
| Frontier | 13 instructions, stop `SERVICE_UNIMPLEMENTED KERNEL32.dll!GetSystemTimeAsFileTime` (rerun today, record in scratch) |
| Python | 17,740 lines across `src`, `scripts`, `tools`, `tests` |
| Pace | 97 commits from 10-07 19:35 to 10-10 00:28; Sprints 034–042 closed between 10-09 13:55 and 10-10 00:28 |

The game's demand, from its own import tables:

| Image | Imports | Served by w2m |
|---|---|---|
| `Hail to the Rainbow.exe` | 66 (65 KERNEL32, 1 `UnityPlayer.dll!UnityMain`) | 10 |
| `UnityPlayer.dll` | 490 from 18 DLLs | 12 |
| `mono-2.0-bdwgc.dll` | 313 from 10 DLLs | 12 |

The launcher's 55 missing KERNEL32 functions are the statically linked C runtime's start-up and support surface: time and process identity (`GetSystemTimeAsFileTime`, `GetCurrentProcessId`), the heap (`GetProcessHeap`, `HeapAlloc`, `HeapFree`, `HeapSize`, `HeapReAlloc`), TLS (`TlsAlloc`, `TlsGetValue`, `TlsSetValue`, `TlsFree`), critical sections (four), code pages and strings (`GetACP`, `GetOEMCP`, `GetCPInfo`, `IsValidCodePage`, `MultiByteToWideChar`, `WideCharToMultiByte`, `GetStringTypeW`, `LCMapStringW`), the command line and environment (four), modules (`GetModuleHandleW`, `GetModuleHandleExW`, `GetModuleFileNameW`, `LoadLibraryExW`, `GetProcAddress`, `FreeLibrary`), unwinding and failure paths (`RtlCaptureContext`, `RtlLookupFunctionEntry`, `RtlVirtualUnwind`, `RtlUnwindEx`, `RaiseException`, `UnhandledExceptionFilter`, `SetUnhandledExceptionFilter`, `IsDebuggerPresent`, `TerminateProcess`), the console and files (`WriteConsoleW`, `GetConsoleCP`, `GetConsoleMode`, `SetStdHandle`, `GetFileType`, `FlushFileBuffers`, `FindFirstFileExW`, `FindNextFileW`, `FindClose`), and `InitializeSListHead`, `IsProcessorFeaturePresent`, `GetStartupInfoW`. Which of them start-up reaches is a frontier question, not a static one.

## 3. What Sprints 034–042 proved

- **Reference discipline.** Every instruction verdict cites the CPU and run it came from; the AMD runs corrected the model three times before Intel agreed on all 39,708 cases (039). The CPU found a scratch-register clobber in 625 cases on its first run of set 041; the SDM's flag table found SHR CF over-defined since Sprint 029.
- **Planted faults with reach.** Plants now state the field they change and how many reference cases change it (KIT_DIARY #21, #22). Two plants that sat in the judge's blind spot (AF undefined; PUSH before store) were caught as blind spots and replaced.
- **Frozen references.** Lifting CPUID changed set 039 because the generator consulted the lifter; the inputs are now frozen by sha256 (#23).
- **Operating-system behavior from observation.** CreateFileW's contract takes its acceptance rows from Server 2025 and Server 2022 runs; three rows contradict or extend Microsoft Learn (POSIX delete while open, trailing dot and space stripped, ERROR_NEGATIVE_SEEK 131) and are recorded as such (#27).
- **Substrate as orchestration.** Instruction classes and the CreateFileW family are item runs with a causal chain from candidate to decision; a duplicated decision in every one of 139 class records was found by counting events and fixed with `Once()` (#26).
- **Generation measured once.** In substrate's session topology, kimi-k2.7-code:cloud wrote a CreateFileW `file.c` the item run accepted in one turn (46 model calls, 45 tool calls, 2,697,424 prompt and 35,069 completion tokens). The shipped `file.c` is the agent's.

## 4. Findings

### D1 — CI has been red for four pushes, and sprints closed on it

Commit 3710f99 made `substrate-kernel` an editable path source, `{ path = "substrate", editable = true }`, and `.gitignore` excludes `/substrate/`. A fresh checkout has no `./substrate`, so `uv sync --all-extras` fails ("Distribution not found at: file:///home/runner/work/w2m/w2m/substrate") on runs 38033959901, 38033989258, 38034472839 and 38034552789. Sprint 042 closed with "gates 0" from the local run while CI was red. A second checkout of w2m on any machine cannot install.

The clone's branch `w2m` sits at 3c0542a with no commits beyond it, and 3c0542a is origin/main of `laffeyp/substrate`. A git source pinned to that revision restores CI today with no push at all. When w2m needs kernel changes, branch `w2m` can be pushed to origin (the user's rule forbids pushing main, not branches) and the pin moved to its head. A sprint should close only on a green CI run of its closing commit.

### D2 — Records do not say which code produced them

Every SESSION_INIT carries `harness_version="0.0.1"`, a constant; manifests carry run id, segment and replay ceiling. No record names the w2m commit or the substrate commit. With substrate now an editable clone that changes under w2m, a record cannot be tied to the kernel that wrote it. `w2m.record` can fill `harness_version` with `git describe --always --dirty` for both repositories.

### D3 — Sprint 045's judgment needs Sprint 046

`docs/plan.md` judges 045 by "the frontier passes C runtime start-up and enters `UnityMain`". `UnityMain` is the launcher's only import from `UnityPlayer.dll`; reaching it means loading that 26 MB image with relocations, its own 490 imports and its DllMain, which is 046's scope. By Decision 112's rule (a test that cannot run means the sprints are out of order), 045's judgment should be "the launcher's C runtime start-up completes and the call to `UnityPlayer.dll!UnityMain` is the stop", and 046 judged by `UnityMain`'s entry.

### D4 — Decision 105 is not implemented

Decision 105 (2026-10-09) says Sprint 040 serves QueryPerformanceCounter inside the guest from CNTVCT_EL0, and that every service above about a thousand calls a second stays in the guest. `k32_QueryPerformanceCounter` in `runtime.c` calls `host_call(W2M_HOST_CLOCK)`, which exits the VM into Python's `_host_request`. The game called it 16,912,633 times in 180 s on the runner. Sprint 040's card does not mention it. TlsGetValue (23,291,261 calls) is not served yet and must start in the guest.

### D5 — The roadmap's next sprint does not move the game

Sprint 043 (open) moves six differentials onto one causal topology and a discrepancy ledger. That repairs S1 of the progress review and is good work, but no judgment waits on it and the six differentials already pass. Sprint 044 (shared-memory transport) also has no waiting judgment. The game's next need is the launcher's C runtime surface, and 042 built the pipeline (probe on the runner, contract from its rows, bridge, guest C, judge in the VM) for exactly that.

### D6 — The unit of work should be a family, not a function

Sprint 042 took one family (CreateFileW, CloseHandle, DeleteFileW, ReadFile, WriteFile, SetFilePointerEx) through one probe, one contract and one item run, because those functions share state (open files, sharing table, delete-pending). The launcher's surface falls into the same kind of families: heap (5), TLS (4), critical sections (4), code pages and strings (8), command line and environment (4), modules (6), unwinding and failure (9), console and files (9), and a handful of stateless queries. One probe and one item run per family keeps shared state inside one judge; the stateless queries fit one probe together. That is about ten item runs for the launcher, each independent of the others, so they can run concurrently as separate substrate runs, as the progress review's §6.3 proposed for single items.

### D7 — Generation is proven once and not used

The one cloud-model session passed. All 141 class rules and all six file functions shipped from the agent session, one at a time. For the 55 + 490 + 313 imports ahead (with overlap between the DLLs), concurrent model sessions in substrate's session topology, each owning one family's item run, are the lever that does not serialize on one agent. The measurement recorded the model calls and tokens; a second family run by a model, accepted by the same kind of judge, would show whether the first was luck.

### D8 — Smaller items

- `w2m_logged` increments a global ring count without atomics; correct on one vCPU, a race when Sprint 047 runs several.
- `scripts/check_guest_state.py` had an uncommitted rename of `_inp` to `input` (shadows the builtin) when last read.
- The substrate clone carries a side branch `fix/bash-tool-timeout` (418c027) in worktree `substrate-bash-fix`, based on b3874ac, 20 commits behind origin/main, unmerged.
- The local VM: SSH works (33 ms a call); the 3D display (local-VM review L2) is still `virtio-ramfb`; Decision 110 (the guest's lack of internet "not a defect") conflicts with Decision 106 (the Steam client logged in so `SteamAPI_Init` succeeds).
- The progress review's count of 323 instruction classes UnityPlayer reaches without a rule predates 041; the current gap is not on the record.

## 5. SDD and substrate

The kit is being used well where it matters most: observation contracts state expected values and planted faults with reach; resolve-and-record carries every user redirection (Decisions 112, 116–118); KIT_DIARY turns each surprise into a finding (#19–#27) instead of a silent fix. Two gaps: the dual contract's artifact pass reads local gates, which let D1 through four closes; and a Decision's "done in Sprint N" (D4) has no check that Sprint N did it.

Substrate is now used as an orchestrator for item runs (class runs, the file run, `Once()` decisions) and as a model harness (session topology). Sprint 043 finishes the conversion of the old one-producer records. The missing piece is provenance (D2).

## 6. Direction

1. **Fix CI now** (D1): pin `substrate-kernel` to `git+https://github.com/laffeyp/substrate@3c0542a`, or push branch `w2m` and pin its head; close sprints only on green CI.
2. **Stamp provenance** (D2): w2m and substrate commits in every SESSION_INIT.
3. **Next sprint: the launcher's C runtime surface, by family** (D5, D6): about ten family item runs through the 042 pipeline, run concurrently, judged by the frontier's stop moving to `UnityPlayer.dll!UnityMain`. QueryPerformanceCounter, TlsGetValue and GetCurrentThreadId in the guest (D4).
4. **Give one or more families to model sessions** in substrate's session topology alongside the agent's (D7).
5. **Fix the 045/046 judgments** (D3); then 046 (UnityPlayer and Mono images), whose frontier lists the next families.
6. **Run 043 and 044 when a judgment needs them**, or concurrently in a separate worktree; neither blocks the frontier.

## Sources

- Commands run 2026-10-10: `scripts/gates.sh`; `scripts/run_frontier.py "~/w2m-depots/hail-to-the-rainbow/Hail to the Rainbow.exe"`; `w2m.pe.parse` over the exe, `UnityPlayer.dll`, `mono-2.0-bdwgc.dll` against `vm_backend.SERVICES`; `gh run list`, `gh run view 38034552789 --log-failed`; `git -C substrate branch -vv`, `ls-remote`.
- Files: `docs/plan.md`; `sprints/sprint-039…043`; BLACKBOARD Decisions 105, 106, 110, 112, 115–118; KIT_DIARY #19–#27; `tools/guest_runtime/runtime.c` (lines 41–49, 95–98); `src/w2m/vm_backend.py` (lines 94, 816–840); `pyproject.toml` (line 52); `.gitignore`.
