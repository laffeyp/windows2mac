# Sprint 048 — the guest's file namespace

```yaml
---
id: 048
status: open
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
