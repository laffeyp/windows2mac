# SIGNAL_REPORT — Sprint 204 — The file bridge

Made by `signal_report 204` from 0 records under store/runs/ whose baseline names one of the sprint's 4 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 204.

## 1. Observed

### scope_confirmation

> The Decision "Sequence after Sprint 198", the batch run's stops in row order: the stream equals the Python's rows
> 0-373 and stops at row 374, FILE_STAT 24, the first of the game's file calls (rows 374-395: GetFileAttributesExW,
> CreateFileW, GetFileType, SetFilePointerEx, ReadFile). `src/w2m/file_bridge.py` (368 lines) serves the file requests
> 6-13 (LOOKUP, OPEN, CLOSE, READ, WRITE, SIZE, TRUNCATE, UNLINK), 24-28 (STAT, MKDIR, RMDIR, FSTAT, DUP) and 32 (LIST)
> over a host directory that is C:\ with C:\w2m\app mounted on the program's directory, the recorded profile folders
> (`specs/recorded_folders.json`) and a read-only mode. The same port ends the `statsize` guest's stop (FILE_STAT 24) and
> windows.module.search's (FILE_MKDIR 25).

### work_performed

- a05ef408f Sprint 204: the file bridge
- cb013d8e8 Sprint 204 code (worktree base)
- 497156990 Sprint 204 card: the file bridge
- 8ce885b24 Sprint 204 card: the file bridge

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None: the judges are `batch_run.rs`, `family_runs.rs` and unit tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 204: its Rubber Duck Pass and close.
