# Sprint 204 — The file bridge

```yaml
---
id: 204
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 198", the batch run's stops in row order: the stream equals the Python's rows
0-373 and stops at row 374, FILE_STAT 24, the first of the game's file calls (rows 374-395: GetFileAttributesExW,
CreateFileW, GetFileType, SetFilePointerEx, ReadFile). `src/w2m/file_bridge.py` (368 lines) serves the file requests
6-13 (LOOKUP, OPEN, CLOSE, READ, WRITE, SIZE, TRUNCATE, UNLINK), 24-28 (STAT, MKDIR, RMDIR, FSTAT, DUP) and 32 (LIST)
over a host directory that is C:\ with C:\w2m\app mounted on the program's directory, the recorded profile folders
(`specs/recorded_folders.json`) and a read-only mode. The same port ends the `statsize` guest's stop (FILE_STAT 24) and
windows.module.search's (FILE_MKDIR 25).

- **The bridge:** `crates/w2m_vm/src/file_bridge.rs`, `FileBridge` as the Python has it: the drive and its folders, the
  app mount, `_parts`, `_resolve` (case-folded), the descriptors, the recorded folders, and `request` for every op.
- **The host:** `w2m-vm` gives a run its bridge (the batch run's depot mounted; a guest or probe run a temporary drive
  holding probe.exe, as `run_in_vm` makes it) and `host_request.rs` serves the ops through it; `modules.rs`'s
  `host_file` uses the bridge's resolution.
- **The runs:** the batch run, the guests and the module families run again; each stop moved is named.

## prerequisites

- 203 (closed)

## context_files

- `src/w2m/file_bridge.py` (whole); `src/w2m/vm_backend.py` the file ops' dispatch; `specs/recorded_folders.json`;
  `tests/test_file_bridge*.py` (the values to carry); `crates/w2m_vm/src/{host_request.rs, modules.rs}`

## signal contract

### Emits

- None: the judges are `batch_run.rs`, `family_runs.rs` and unit tests.

### Invariants

- No Python file added or changed; the stream's rows 0-373 still equal; the guests and module families as before or
  further, each move named.

## artifact contract

### Files modified

- `crates/w2m_vm/src/file_bridge.rs` (new), `crates/w2m_vm/src/{host_request.rs, modules.rs, guest.rs, lib.rs}`,
  `src/bin/w2m-vm.rs`, `store/batch_run/first-stop-2026-10-28.tsv`, `store/family_runs/module-stops-2026-10-28.tsv`,
  `docs/step-4-census-2026-10-28.md`

### Command exit codes

- `batch_run.rs`, `family_runs.rs`, the bridge's unit tests and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the stream passes rows 374-395 and equals the Python's to its next stop, named; `statsize` ends
  with the Python's output or a later stop; windows.module.search's runs end or stop later, named.
- **Planted faults:** (1) the case fold dropped: a stream row or a unit test names the path not found; (2) READ's
  count off by one: the stream names ReadFile's row; (3) the read-only mode ignored: a unit test names the write.

## done criteria

The Rust host serves the file requests as the Python's file bridge does, and the batch run passes its first file
calls.

## self-review checklist

- [x] Scope names one verifiable property: rows 374-395 and on equal the Python's.
- [x] context_files cite the Python to read first.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the file bridge.
