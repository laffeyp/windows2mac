# SIGNAL_REPORT — Sprint 048

## 1. Observed

### scope_confirmation

> The frontier runs with the game's directory as `C:\w2m\app`, reads boot.config and app.info, writes its Player.log under `C:\Users\w2m\AppData\LocalLow`, and stops at 2,668,647 instructions at `USER32.dll!MessageBoxA` ("Failed to load mono"): LoadLibraryW of a DLL by path after the start, which w2m does not do. Every service before the stop is judged against Windows.

### work_performed

- F1: `spin` in `tools/vm_monitor/vm_monitor.c`, `VM.measure_exits`, `scripts/measure_exit.py`. F3: `scripts/check_replay.py` prints "intact" and names level 2's check.
- The drive namespace: `src/w2m/file_bridge.py` (drive C:, the mount at `C:\w2m\app`, read-only for the frontier, the profile folders, FILE_MKDIR and FILE_RMDIR), `tools/guest_runtime/paths.c` (full paths, the working directory, directories).
- System DLLs: `specs/windows_exports.json` 0.2.0 and `scripts/exports_spec.py`; a synthetic module per statically imported system DLL (`src/w2m/modules.py`, `SYSTEM_IMAGES_OFF`); SHELL32, OLE32 and ADVAPI32 services through `DLL_SERVICES`.
- Families: paths, file security, local free, file access, system modules, known folders (`known_folders.c`), temp path (`environment.c`: a mutable block), integrity level (`token.c`, `local_memory.c`).
- Classes: PREFETCHW, PREFETCHT0–T2; set 048 judged on the Intel Xeon 8573C (run 38093695090).
- The frontier and look-ahead start the game with TEMP, TMP and USERPROFILE (`USER_ENVIRONMENT`).

### signal_trace

```
store/runs/frontier-643a6283510c-1791679311047487000: SESSION_INIT, EXECUTION_STARTED, SERVICE_UNIMPLEMENTED (USER32.dll!MessageBoxA, 2,668,647 instructions), SESSION_COMPLETED
store/runs/family-runs.json: 33 families, each SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED x runner binaries, CANDIDATE_ACCEPTED, SESSION_COMPLETED
store/runs/class-runs.json: classes accepted with set 048 (Intel Xeon 8573C run 38093695090, release semantics-048)
```

## 2. Expected

> The card expected F1's two figures to replace "≈27 µs", each family to equal Windows on Server 2025 and 2022 with plants of named reach, and the frontier to stop past the first `GetFileAttributesExW` on `C:\w2m\app\...` with every service judged. All held.

## 3. Delta

### dual_contract_self_grade

**Signal contract:** pass (vocabulary 0.7, no new tags). **Artifact contract:** gates 0; CI run 38099562363 success on 401deef. **Observation contract:** F1 pass (685–769 ns raw, 10.3–10.5 µs through the pipe); families pass (eight new, every plant as named after one corrected prediction); frontier pass (2,668,647, past boot.config and app.info).

**overall:** pass.

## 4. Hypothesis

### rubber_duck_observations

**Sequence narration:** run the frontier; record the stopped call's arguments past it; read the caller's code when the arguments do not say why; probe Windows the way the caller calls; contract, guest C, item run, plants; run every family again when shared code moves; run the frontier.

- *Observation 1 (a probe's own fault read as Windows's):* the temp-path probe died with 0xC0000005 four times; one commit blamed GetEnvironmentStringsW on an emptied environment before the cause, the probe's 64-name pass and a buffer overflow, was found. Disposition: rows are written as they end; the commit after corrected the comment; KIT_DIARY #37.
- *Observation 2 (a plant's reach predicted wrong):* temp_before_tmp named temp.size_zero, which prints a length, and TEMP's path is as long as TMP's. Disposition: the named list corrected with the reason beside it; KIT_DIARY #38.
- *Observation 3 (runner state is not a user's):* runners run elevated (RID 0x3000) and hold more than 64 environment variables. Disposition: the family compares whether the RID is low; the probe deletes the runner's variables before it sets its own.

### status_and_blockers

`status: complete`
