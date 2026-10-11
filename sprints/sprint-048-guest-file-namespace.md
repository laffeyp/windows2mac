# Sprint 048 — the guest's file namespace

```yaml
---
id: 048
status: closed
phase: 5
pass_kind: functional
---
```

## scope

Sprint 047 left the frontier at 2,522,889 instructions, at the game's first file call: `GetFileAttributesExW("C:\w2m\app\Data\boot.config")`. The frontier gives the VM no file-system root, and the file bridge refuses absolute paths and drive letters. Unity looks for its data folder under several roots (`Data`, `<exe>_Data`, `DataSource`, beside the executable and at `C:\`). UnityPlayer.dll imports 28 file-system functions w2m does not serve (GetFullPathNameW, GetCurrentDirectoryW, CreateDirectoryW, FindFirstFileW, FindFirstFileExW, GetTempPathW and more).

1. **The review's F1 and F3 first.** F1: measure a VM exit apart from the monitor's text pipe (`spin` in `vm_monitor.c`, `VM.measure_exits`, `scripts/measure_exit.py`) and restate the plan's transport figures from it. F3: the gate's replay line says what level 2 checks (record integrity), not that w2m still behaves as recorded.
2. **The drive namespace:** `C:\` is a host directory per run; the program's directory, `C:\w2m\app`, is the game's depot directory; the working directory is the program's directory, as Steam starts a game. Absolute and drive-relative names resolve in it; a name that leaves a drive's root stops by name.
3. **Paths and directories as a family:** GetFullPathNameW, GetCurrentDirectoryW, SetCurrentDirectoryW, CreateDirectoryW, RemoveDirectoryW, and absolute names through the file functions already served, probed on the runners.
4. **Finding files as a family:** FindFirstFileW, FindFirstFileExW, FindNextFileW, FindClose, if the frontier reaches them.
5. **The frontier with the game's directory** mounted, and the families it meets next.

## prerequisites

- 047 (closing; CI run 38087819989 on 5ac2a1e)

## context_files

- `src/w2m/file_bridge.py`, `tools/guest_runtime/file.c`, `tools/guest_runtime/file_attributes.c`, `scripts/run_frontier.py`, `scripts/family_run.py`, `tools/vm_monitor/vm_monitor.c`; Microsoft Learn: Naming Files, Paths, and Namespaces; GetFullPathNameW; FindFirstFileExW

## signal contract

### Emits

- Per family: the item run. Per frontier run: EXECUTION_STARTED, then SERVICE_UNIMPLEMENTED or EXECUTION_COMPLETED.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- created: `scripts/measure_exit.py`; probes and contracts for the families; the namespace in `file_bridge.py`
- modified: `tools/vm_monitor/vm_monitor.c`, `src/w2m/vm_backend.py`, `scripts/check_replay.py`, `docs/plan.md`, `scripts/run_frontier.py`
- gates 0 and CI `success` on the closing commit

## observation contract

- **F1:** the raw exit and the pipe's round trip, each measured in three repeats on this Mac, replace "≈27 µs" in the plan.
- **Each family equals Windows** on its probe's rows on Server 2025 and 2022, with planted faults of named reach.
- **The frontier reads the game's files:** its stop lies past the first `GetFileAttributesExW` on `C:\w2m\app\...`, every service before it judged.

## done criteria

The frontier runs with the game's directory as `C:\w2m\app` and stops past the game's first file reads, every service before the stop judged against Windows.

## observed

- **The frontier moved** from 2,522,889 instructions (`GetFileAttributesExW` on `C:\w2m\app\Data\boot.config`, a fault) to 2,668,647 (`store/runs/frontier-643a6283510c-1791679311047487000`). It stops at `USER32.dll!MessageBoxA` with "Fatal error: Failed to load mono": `LoadLibraryW("C:\w2m\app\MonoBleedingEdge\EmbedRuntime\mono-2.0-bdwgc.dll")` answers NULL, because w2m loads no DLL after the start. Before the stop the game reads boot.config and app.info, creates `C:\Users\w2m\AppData\LocalLow\NSS\Hail to the Rainbow` and writes its Player.log there ("Unable to load mono library from C:/w2m/app/..."). Every service before the stop has a family judged against Windows.
- **F1:** a raw exit 685–769 ns, one through the pipe 10.3–10.5 µs (three repeats, `scripts/measure_exit.py`); the plan's "≈27 µs" was lockstep's whole step. **F3:** the gate's replay line says "intact" and names what level 2 checks.
- **The drive namespace:** `C:\` is a host directory per run holding `w2m\app` (the game's directory, mounted read-only for the frontier) and the profile `Users\w2m` with `AppData\Local\Temp`, `LocalLow` and `Roaming`; the guest makes every name full against the working directory (`paths.c`).
- **Families accepted, each on Server 2025 and 2022 with plants of named reach:** paths (GetFullPathNameW over 26 names, the working directory, directories), file security (SECURITY_ATTRIBUTES, GetHandleInformation), local free (LocalFree on argv + 1 fails harmlessly; 16 bytes in ends the process), file access (13 access masks and sharing), system modules (statically imported system DLLs as modules; LoadLibraryW), known folders (SHGetKnownFolderPath, CoTaskMemFree), temp path (GetTempPathW, Get/SetEnvironmentVariableW, a mutable block), integrity level (OpenProcessToken, GetTokenInformation(TokenIntegrityLevel), GetSidSubAuthority, LocalAlloc). 33 families in all; every one reruns conforming after this sprint's changes.
- **Instruction classes, set 048:** PREFETCHW and PREFETCHT0–T2 as hints; the Intel Xeon 8573C agrees on all 675 cases and never faults, even at a page's edge (run 38093695090).
- **Exports:** `specs/windows_exports.json` 0.2.0 lists the exports of the 19 DLLs UnityPlayer imports (Server 2025 run 38094046782; `scripts/exports_spec.py`).
- **Windows against expectation:** GetProcAddress(shell32, "SHGetKnownFolderPath") is not the import's address though SHELL32 exports the name directly; the last error after SHGetKnownFolderPath is 1008, 0 or 183 by folder; an empty TMP makes GetTempPathW answer 1 and write nothing; a 303-character TMP is skipped; new environment variables go in upper-cased name order; runners run elevated (RID 0x3000).
- **w2m's choices, recorded in BLACKBOARD:** the user is `w2m` (`C:\Users\w2m`, fresh each run); the game starts with TEMP, TMP and USERPROFILE; the token's RID is medium (0x2000), a standard user's; a known-folder GUID outside w2m's four stops the run.
- **Defects found and fixed:** the temp-path probe faulted four times inside its own row writer (more than 64 variables on a runner overflowed its buffer); one commit blamed GetEnvironmentStringsW before the cause was found, and the next corrected it (KIT_DIARY #37). The known-folders probe printed nothing under w2m because its rows were buffered past the expected stop. A plant's named rows included one whose printed value the fault cannot change (KIT_DIARY #38).

