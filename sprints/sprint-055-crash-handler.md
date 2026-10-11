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

5. **Amended 2026-10-11 from the captures, before the build.** Game run 38176899195 (batch, windows-2025, the game's manifest in jit_capture, the handler traced from inside; unity.log equal to run 38113342549's 168 lines but for timings) holds:
   - **The game's side:** CreateEventW names `UnityCrashHandler_<pid>`, `UnityCrashHandlerResp_<pid>`, `UnityCrashHandlerTerm_<pid>`; the pipe's read end inheritable and given to the child as its standard input; the command line `"<game dir>\UnityCrashHandler64.exe" --attach <pid> <address>`; the read end closed after the start; 12 framed records written (an 8-byte header, then a key and a value); zero-timeout waits on the handler's process answering WAIT_TIMEOUT throughout. The first registry key is `Software\NSS\Hail to the Rainbow` (RegOpenKeyExW, seq 1656).
   - **The handler's side (411 traced calls, 65 functions):** the C runtime's start; OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, the game's id); CreateEventW on the three names (ERROR_ALREADY_EXISTS: the game's events); CoInitializeEx; CommandLineToArgvW; GetStdHandle(STD_INPUT_HANDLE) answering the pipe; CreateThread for a reader that reads the 12 records (8 bytes, then the body); WaitForMultipleObjectsEx on four handles, which returns only when the game ends. When the workflow kills the game the reader sees ERROR_BROKEN_PIPE (109) and the handler starts a second, post-mortem copy of itself and exits: that path is outside a run that ends normally.
   - **Unserved:** for the game CreatePipe, CreateProcessW, named CreateEventW, RoInitialize (seq 75), SetConsoleCtrlHandler (seq 419); for the handler's normal run OpenProcess and WaitForMultipleObjectsEx (59 of its 65 functions are served; CreateProcessW, CancelSynchronousIo, LoadLibraryA and AppPolicyGetProcessTerminationMethod are the post-mortem path's).

   The build, in order:
   - **The processes family**, judged by a probe that starts itself as the child (`probe.exe child ...`): CreatePipe, SetHandleInformation on a pipe, CreateProcessW with inherited handles and the pipe as the child's standard input, the child's GetStdHandle and command line, framed pipe writes and reads, ERROR_BROKEN_PIPE when the writer closes, named events opened by the child (ERROR_ALREADY_EXISTS, one object), OpenProcess by id, WaitForSingleObject and WaitForMultipleObjectsEx on a process, GetExitCodeProcess; and SetConsoleCtrlHandler.
   - **RoInitialize** (with RoUninitialize) from the winrt family, judged by its rows.
   - **The second process** (amended before the build; BLACKBOARD Decision 2026-10-11, the second one on processes): a second VM with its own monitor, driven by the same host; named events, pipes and process records held by the host (`src/w2m/kernel.py`); VM.run as a generator a driver resumes; CreateProcessW building the child VM with its inherited handles at the same values and its standard input from STARTUPINFO.

   **Done criteria (amended):** the processes family and RoInitialize equal Windows; the batch frontier starts UnityCrashHandler64.exe as a second guest process that reads the game's 12 records and waits, and the frontier passes SetConsoleCtrlHandler (seq 419); the next stop is recorded with the unserved functions up to seq 1712 listed.

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

As amended in item 5.
