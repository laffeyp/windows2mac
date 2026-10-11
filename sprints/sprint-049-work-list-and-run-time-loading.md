# Sprint 049 — the work list, and loading at run time

```yaml
---
id: 049
status: open
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
