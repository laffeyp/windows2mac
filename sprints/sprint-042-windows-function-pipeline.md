# Sprint 042 — the Windows-function pipeline, on CreateFileW

```yaml
---
id: 042
status: open
phase: 4
pass_kind: functional
---
```

## scope

Milestone A needs every Windows function the game calls to have a contract built from Windows's behavior, a judge, and an implementation that passes. So far w2m has this for SetLastError/GetLastError (Sprints 031–035) and no file system at all. In play run 38006665419 (`store/game-runs/38006665419/jit_capture.bin.imports.tsv`), UnityPlayer called CreateFileW 683 times and mono 261, with ReadFile 43,114, SetFilePointerEx 38,932, CloseHandle 683, WriteFile 109 and DeleteFileW 2. This sprint builds the pipeline once, end to end, on CreateFileW and those five companions. GetFileAttributesExW (985 calls) and GetFileType (196) are next in line for the same pipeline. User decision 2026-10-10 reframed the sprint from "the generation bet". The model measurement comes last, uses a cloud model (user decision 2026-10-10: local models cannot do it), and nothing depends on it.

1. **The probe** (`probes/create_file/`). It is no-CRT, imports only from kernel32, and runs on the windows-2025 runner in mode stdout. It prints one JSON row per scenario: the scenario name, whether the handle is valid, the return value, GetLastError (read after a SetLastError(0x0000DEAD) sentinel before the call, so a success that leaves the code untouched is visible), and the file's size (SetFilePointerEx to FILE_END) and first bytes afterwards. All paths are relative to the working directory. The probe deletes its files before and after.
   - **Dispositions.** Each of CREATE_NEW, CREATE_ALWAYS, OPEN_EXISTING, OPEN_ALWAYS and TRUNCATE_EXISTING, on an absent file and on an existing 5-byte file. TRUNCATE_EXISTING also runs without GENERIC_WRITE.
   - **Sharing.** The full matrix: first open {R, W, RW} × share {0, R, W, RW}, then a second open with the same choices. That is 144 pairs.
   - **Delete.** DeleteFileW on an absent file, a closed file and an open file (with and without FILE_SHARE_DELETE). After a delete while open, a CreateFileW of the same name.
   - **Names.** A missing directory component; the directory "." without FILE_FLAG_BACKUP_SEMANTICS; a different-case name; a trailing dot; the empty name.
   - **Handles.** WriteFile, SetFilePointerEx to FILE_BEGIN, FILE_CURRENT and FILE_END (and before the start), then ReadFile on the same file; CloseHandle on a closed handle.
2. **The contract** `windows.file.create_file.dispositions_sharing@0.1.0`. Its rows are the runner's output, as observed evidence with the run's execution id. Microsoft Learn's CreateFileW, "Creating and Opening Files" and DeleteFileW pages are documented evidence. It has a new acceptance relation, `rows_equal`: the candidate's rows equal the recorded rows, key for key. The first unequal row is the divergence.
3. **The file-system bridge.** Host operations on the transport page (a widened `host_call`) are POSIX primitives under a per-run root directory on the Mac: look up a name (ASCII case folded, as NTFS compares ASCII names), create exclusively, open, truncate, read, write, size, unlink. A path outside the root, or an absolute one, is `Unimplemented`: honest, not guessed.
4. **The implementation** in the guest runtime (`tools/guest_runtime/file.c`), in C. It holds the dispositions, the sharing table (open files tracked in the runtime, as Windows tracks them per file object), the error codes, the trailing-dot rule and delete-pending. Its services join `SERVICES`.
5. **The judge.** The probe's own binary from the runner runs in the VM. Its output is compared with the contract by `rows_equal`, on a substrate record.
6. **The measurement** (after 1–5 pass). The contract, the runtime header and the bridge operations go to a cloud model. Its `file.c` goes into the same judge. The attempts and outcome go on a record. No gate depends on it.

## prerequisites

- 035 (guest runtime, import bridges), 040–041 (the VM runs clang -O1 code with faults and SSE), 032 (contract format, judged probes), 036 (import census)

## context_files

- `tools/guest_runtime/runtime.{h,c}`, `src/w2m/guest_runtime.py`, `src/w2m/vm_backend.py` (`SERVICES`, `_host_request`), `src/w2m/contract.py`, `src/w2m/compare.py`, `.github/workflows/windows-probe.yml`, `probes/last_error_values/`
- Microsoft Learn: CreateFileW, Creating and Opening Files (the sharing table), DeleteFileW, CloseHandle, SetFilePointerEx, ReadFile, WriteFile, Naming Files (trailing dot and space); System Error Codes 0–499

## signal contract

### Emits

- The runner run: the probe's rows, as Sprints 032–035 record them.
- The judge: SESSION_INIT, CANDIDATE_GENERATED (the implementation's sha256), COMPARISON_COMPLETED (relation `rows_equal`), then CANDIDATE_ACCEPTED or DISCREPANCY_REPORTED + DISCREPANCY_ROUTED, then SESSION_COMPLETED.

### Invariants

- Vocabulary 0.7. `rows_equal` is a relation value inside COMPARISON_COMPLETED's existing field, not a new tag.

## artifact contract

- created: `probes/create_file/{create_file.c,probe.json}`, `contracts/windows.file.create_file.dispositions_sharing/0.1.0.json`, `tools/guest_runtime/file.c`, `src/w2m/file_bridge.py`, `scripts/file_run.py`, `tests/test_create_file.py`, the runner output and binary in `store/`
- modified: `tools/guest_runtime/runtime.{h,c}`, `src/w2m/guest_runtime.py`, `src/w2m/vm_backend.py`, `src/w2m/contract.py`, `src/w2m/compare.py`
- `scripts/gates.sh` → 0

## observation contract

- **Windows recorded:** every scenario has a row from the runner, the binary's sha256 is on the record, and two runs on different runner machines give equal rows (any row that differs is named and excluded with a reason).
- **The VM equals Windows:** the probe's runner binary, run in the VM, prints rows equal to the contract's on every scenario.
- **Planted faults**, reach counted first:
  - The sharing check without its reverse half (the first opener's access against the second's share) changes the second-open row on 35 of the 144 pairs, by the documented rule. If Windows's rows disagree with the documented rule, the count is redone from the record before the plant runs.
  - ERROR_ALREADY_EXISTS dropped changes the error field on two rows: CREATE_ALWAYS and OPEN_ALWAYS on an existing file.
  - Case-sensitive lookup changes the different-case row.
  - Each is caught by `rows_equal` alone.
- **The measurement:** the model, the prompt's sha256, the number of attempts, and each attempt's first divergence or acceptance are on a record.

## done criteria

CreateFileW and its companions run in the VM as Windows runs them on every recorded scenario, judged by a contract made from Windows's output. The pipeline (probe, contract, bridge, runtime implementation, judge) is the template for the next Windows function.
