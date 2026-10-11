# Sprint 055 — the crash handler: a second process

```yaml
---
id: 055
status: open
phase: 5
pass_kind: functional
---
```

## scope

Sprint 054 left the frontier, given the batch run's arguments, at `KERNEL32.dll!CreateEventW` with a name (batch trace seq 6) after 421,498 instructions: UnityPlayer starting UnityCrashHandler64.exe. The batch trace (`store/game-runs/38113342549`) holds the start: three CreateEventW with names (handles 0x26c, 0x270, 0x274), CreatePipe, SetHandleInformation(0x280, HANDLE_FLAG_INHERIT, 0), GetModuleFileNameW, three GetCommandLineW, CreateProcessW(NULL, a command line, ..., bInheritHandles TRUE, 0x20 NORMAL_PRIORITY_CLASS), CloseHandle(0x284); then framed writes to the pipe (an 8-byte header, a key, a value: four records by seq 71) and zero-timeout WaitForSingleObject on the process (0x288, WAIT_TIMEOUT each time). RoInitialize (seq 75, combase through GetProcAddress) and SetConsoleCtrlHandler (seq 419) follow on the batch path.

The handler is a full program: UnityCrashHandler64.exe imports 195 functions from 10 DLLs (KERNEL32 158, USER32 17, WININET 7, dbghelp, SHELL32, ole32, VERSION, PSAPI, ADVAPI32, GDI32), carries its own manifest and dialog resources, and no held capture records what it calls while the game runs. The captures also hold the names only as pointers. Planning the process from guesses is ruled out (memory "use answers already held"), so the sprint captures first.

1. **Strings in the game capture.** `tools/jit_capture` records the input strings of name-taking functions (event and file names, the command line, module and registry names, window titles and classes) in `<capture>.strings.tsv`, by call sequence number; the trace format is unchanged.
2. **The handler's own calls.** jit_capture's CreateProcessW hook starts UnityCrashHandler64.exe suspended and queues LoadLibraryW of `handler_trace.dll` as an APC on its main thread; the APC runs after the loader resolves the handler's imports and before its entry point, and the DLL hooks the handler's import table with jit_capture's machinery (moved to a shared file), writing the same files under its own base name. The handler runs as its own exe, with its manifest and resources (KIT_DIARY #45).
3. **The batch capture again**, on windows-2025, the game's manifest linked into jit_capture (Sprint 054), held as a new game run; the old runs stay.
4. **From the new captures, the plan:** the handler's calls in a batch run, listed against w2m's service table; the card is amended with the families the second process needs and the address-space design (BLACKBOARD Decision 2026-10-11: a second guest address space in the VM, w2m holding the handles both processes share), before any of it is built.

## prerequisites

- 054 (closed: CI run 38175121068 on 2b02046a; main 90731e7a)

## context_files

- `tools/jit_capture/jit_capture.c`, `.github/workflows/windows-game.yml`, `scripts/work_list.py`, `store/game-runs/38113342549/`; Microsoft Learn: CreateProcessW (CREATE_SUSPENDED), QueueUserAPC, CreatePipe, SetHandleInformation

## signal contract

### Emits

- The game capture's run records (Sprint 038's DISTRIBUTION and EVIDENCE events as the workflow ingest emits them); per family: the item run; per frontier run: EXECUTION_STARTED, then SERVICE_UNIMPLEMENTED or EXECUTION_COMPLETED.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- created: `tools/jit_capture/call_trace.h` (the shared hooks), `tools/jit_capture/handler_trace.c`, a new game run under `store/game-runs/` with `strings.tsv` and the handler's files
- modified: `tools/jit_capture/jit_capture.c`, `.github/workflows/windows-game.yml`
- gates 0 and CI `success` on the closing commit

## observation contract

- **The new capture holds the names:** CreateEventW's three names and CreateProcessW's command line for the handler, and the registry key RegOpenKeyExW asks for at seq 1653's call.
- **The handler's capture holds its calls** in the batch run, with counts, and the work list lists the unserved ones.

## done criteria

Amended at item 4 from the captures.
