# Sprint 049 — the work list, and loading at run time

```yaml
---
id: 049
status: closed
phase: 5
pass_kind: functional
---
```

## scope

Sprint 048 left the frontier at 2,668,647 instructions, at `MessageBoxA("Failed to load mono")`: w2m loads no DLL after the start. Sprints 046–048 found each need by crashing into it, while the game's own Windows run (`store/game-runs/38006665419`) already listed every function it calls: 168 statically imported functions called, 79 of them unserved; 331 names through GetProcAddress, 6 served, about 260 of them `mono_*` exports that resolve inside mono once it loads (user, 2026-10-10: "we shouldn't be searching for answers if we already have them"). This sprint works from that list.

1. **The reference trace.** `tools/jit_capture` records, for the first 32 calls of each function, the call's sequence number, thread, the four register arguments and four stack arguments, the return value and the last error after it; functions reached through GetProcAddress in a Windows DLL are traced too. A reader turns the capture into committed rows.
2. **The work list.** `scripts/work_list.py` writes `specs/work_list.json` from the capture and the service tables: each unserved function, its DLL, callers, call count, first call's sequence number, and its family; families ordered by first call.
3. **Services declared where they are written.** Each guest C file declares the services it implements; `vm_backend.py` and the API ids are built from those declarations, so families written on separate branches do not collide.
4. **Loading at run time:** LoadLibraryW of a DLL by path maps it, relocates it, binds its imports, runs its TLS callbacks and its DllMain from inside the service; GetProcAddress answers its exports. Calling guest code from the runtime serves FlsFree's callbacks too.
5. **Families from the list**, several at once, each in its own worktree, in the work list's order.
6. **Probe skeletons from microsoft/win32metadata**, if the reader is done before the families need it.

## prerequisites

- 048 (closed: CI run 38099562363 on 401deef)

## context_files

- `tools/jit_capture/jit_capture.c`, `.github/workflows/windows-game.yml`, `store/game-runs/38006665419/`, `src/w2m/loader.py`, `src/w2m/modules.py`, `src/w2m/vm_backend.py`, `tools/guest_runtime/runtime.h`, `scripts/family_run.py`; Microsoft Learn: LoadLibraryExW, DllMain; PE/COFF specification

## signal contract

### Emits

- Per family: the item run. Per frontier run: EXECUTION_STARTED, then SERVICE_UNIMPLEMENTED or EXECUTION_COMPLETED. The game run on the runner: its record in `store/game-runs/<run>/`.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- created: `scripts/work_list.py`, `specs/work_list.json`, the trace reader, the run-time loader, families' probes and contracts
- modified: `tools/jit_capture/jit_capture.c`, `src/w2m/vm_backend.py`, `src/w2m/modules.py`, `tools/guest_runtime/runtime.h`
- gates 0 and CI `success` on the closing commit

## observation contract

- **The trace holds the game's calls in order** with arguments and returns on Server 2025, and the work list is built from it, not from frontier stops.
- **mono loads:** LoadLibraryW of `mono-2.0-bdwgc.dll` returns its base after its DllMain returns TRUE, and GetProcAddress answers its exports.
- **Each family equals Windows** on its probe's rows on Server 2025 and 2022, with planted faults of named reach.
- **The frontier passes "Failed to load mono"**, every service before its stop judged.

## done criteria

mono-2.0-bdwgc.dll loads at run time inside w2m and the frontier passes "Failed to load mono"; the work list and the trace are committed, and every family this sprint adds comes from them.

## observed

- **The frontier moved** from 2,668,647 instructions (`MessageBoxA("Failed to load mono")`) to 5,376,528: mono-2.0-bdwgc.dll loads at run time (mapped into the arena, relocated, bound, DllMain run from inside LoadLibraryW), starts, maps mscorlib and its other assemblies, and stops on legacy CVTDQ2PS (0f 5b) in its own code at 0x1180023e52, a class the mono census already lists (`store/census/class-census-mono-2026-10-11b.json`).
- **The trace and the work list:** `tools/jit_capture` records each function's first 32 calls with arguments, return and last error (game runs 38113342549, 38113343972); `scripts/work_list.py` writes `specs/work_list.json` (164 unserved functions in 19 families at the start, 148 in 16 at the close). Every family this sprint added came from it, and probes took the game's own arguments from the trace.
- **Services declared where they are written:** the API ids and service tables come from the guest C definitions (`guest_runtime.service_table()`, `api_ids.h`).
- **The guest CPU is x86-64-v3** (user decision 2026-10-11). Case sets 049 (51 classes, 24,334 cases) and 049b (80 classes, 46,970 cases) judge the model and the VM equal to the Xeon 8573C (runs 38117654018, 38118493713; releases semantics-049, semantics-049b).
- **Families accepted, each on Server 2025 and 2022 with plants of named reach,** among them known folders, temp path, integrity level, critical-section flags, module names, function tables, vectored handlers, thread stack, thread handles, heap query, files more, locale, file mapping and format message.
- **Not done, deferred:** families ran one after another in this session, not in parallel worktrees (card item 5); the win32metadata reader (item 6) was not needed before the families (BLACKBOARD Deferred).
- **Scope beyond the card:** the AVX profile and its two case sets came from the user's decision mid-sprint, and the families after mono loaded (locale, file mapping, format message) followed the frontier past the card's done criteria. The card was not amended when they began (KIT_DIARY #40).
- **Defects found and fixed:** two invented citations in commits and docs (a run id and two page dates), corrected the same day (KIT_DIARY #39); a family candidate or plant that failed to build left the substrate run quiescent and never finalised, now rejected on its record; CI failed from 59d2405d on because it did not build the 049 case files; BLACKBOARD kept the superseded v2 CPU decision as the last word until d470850a.

