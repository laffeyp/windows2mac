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

## result

- **The bridge:** `crates/w2m_vm/src/file_bridge.rs`, `src/w2m/file_bridge.py` ported: ops 6-13, 24-28 and 32, the
  drive with C:\w2m\app mounted, case-folded resolution, the recorded folders from `specs/recorded_folders.json`, the
  read-only mount, FILE_STAT's times from the host file's metadata, and the owned drive removed on drop. Unit tests:
  `a_file_is_found_case_folded_stated_opened_and_read`, `the_read_only_mount_refuses_a_change_and_the_profile_takes_one`
  and `paths_split_as_the_python_splits_them`.
- **The host:** `host_request.rs` serves the file ops through `Guest::files`. The game runs with its depot mounted
  read-only and no time limit; a guest with a temporary C:\w2m\app holding probe.exe and the 60 s limit, now the
  explicit argument `limited` of `on_one_vcpu` (it had keyed on a missing app root); a child with its parent's drive,
  read-only setting and limit, carried in the child's start.
- **The runs:** the stream equals the Python's rows 0-499, row 500 in flight, and stops at row 500, MODULE 15, a
  LoadLibrary of MonoBleedingEdge\EmbedRuntime\mono-2.0-bdwgc.dll, a DLL file after the start (`_load_dll`). 14 of the
  16 guest runs end: `statsize` now ends with the Python host's output; `mutual` and `mutual_context` stop at THREAD 30
  request 5 as before. All 12 module-family binaries ran; windows.module.search's two now copy their DLL and stop at the
  LoadLibrary of w2m_search\w2m_found.dll, `_load_dll`.
- **Plant 1** (the case fold dropped): `a_file_is_found_case_folded_stated_opened_and_read` failed, FILE_STAT's kind
  "left: 3, right: 1"; the batch run's stream test passed under it. Restored byte for byte.
- **Plant 2** (READ's count one more): the unit test failed, "left: (5, [51, 52, 53, 54])", and the held stop test
  failed on the run's counts ("compiled\t13894" for 14018). The stream test passed under it, so the stream did not name
  ReadFile's row as the contract expected. Restored byte for byte.
- **Plant 3** (the read-only check off): `the_read_only_mount_refuses_a_change_and_the_profile_takes_one` failed,
  "called `Result::unwrap_err()` on an `Ok` value: (0, 0)". Restored byte for byte.
- **Departures:** `modules.rs`'s `host_file` still resolves through its own `AppDirectory`, not the bridge's
  resolution, as scope promised; `_load_dll`, the next stop for both the game and the search family, will need one
  resolution for both. The search family's first run on the bridge printed `"copied": false` against the contract's
  true: the runner had module_search_dll.dll beside probe.exe, which `run_in_vm` writes when a family sets
  `dll_on_disk`; `w2m-vm` gained `--beside <name> <file>` and `store/family_runs/beside-2026-10-28.tsv` (transcribed
  from `scripts/family_run.py` 643-646), read by `scripts/gates.sh`, two files the artifact contract did not name.
  Plant 2's judge was the held stop and the unit test, not the stream.
