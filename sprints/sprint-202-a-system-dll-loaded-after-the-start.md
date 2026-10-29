# Sprint 202 — A system DLL loaded after the start

```yaml
---
id: 202
status: done
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 198": the batch run's stops in row order. The stream equals the Python's rows 0-48
and stops at row 49, UnityPlayer's LoadLibrary of api-ms-win-core-winrt-l1-1-0.dll, whose host is combase.dll
(`API_SET_HOSTS`), a System32 DLL the program did not import. `modules.py` 124-129 answers it with a synthetic system
image when `specs/windows_exports.json` lists the DLL's exports (`load_system`, vm_backend's `system_image`: one page
with DOS and PE headers in the next free slot of the process area), and GetProcAddress on it answers a thunk for each
export the spec lists.

- **The load:** `modules.rs` serves a LoadLibrary of a System32 name whose exports the spec lists by writing a synthetic
  page in the next system slot, as the start writes the imported ones, and records it under System32's spelling;
  `SYSTEM_IMAGES` full stops by name.
- **The run:** the batch run run again to its next stop, the stream held past row 49.

## prerequisites

- 201 (closed)

## context_files

- `src/w2m/modules.py` 94-132; `src/w2m/vm_backend.py` `system_image` (in `_modules`); `crates/w2m_vm/src/modules.rs`;
  `crates/w2m_vm/src/process.rs` (the system pages at the start)

## signal contract

### Emits

- None: the judges are `batch_run.rs` and unit tests.

### Invariants

- No Python file added or changed; the stream's rows 0-48 still equal; the guests as before.

## artifact contract

### Files modified

- `crates/w2m_vm/src/modules.rs`, `crates/w2m_vm/src/process.rs`, `store/batch_run/first-stop-2026-10-28.tsv`,
  `docs/step-4-census-2026-10-28.md`

### Command exit codes

- `batch_run.rs`, `modules.rs`'s unit tests and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** row 49 returns the Python's handle and the stream equals the Python's to the next stop, named.
- **Planted faults:** (1) the synthetic page written one slot off: the stream names the row whose handle differs; (2)
  a spelling other than System32's recorded: a GetModuleFileName row or a unit test names it; (3) a name the spec lacks
  loaded anyway: the unit test names it.

## done criteria

A LoadLibrary of a System32 DLL the program did not import answers as the Python's, and the batch run passes row 49.

## self-review checklist

- [x] Scope names one verifiable property: row 49 and on equal the Python's.
- [x] context_files cite the Python to read first.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: a system DLL loaded after the start.

## result

- **The load:** `module_in` answers a LoadLibrary of a System32 DLL whose exports the spec lists with the next free
  system slot's address (`Handles::next_system`), a stop by name when the slots are full; `module` writes the synthetic
  page there (`process::synthetic_image`) and records the DLL, an API set's host when one was asked, under System32's
  spelling. Unit cases: dbghelp.dll and the winrt API set load in the next slot, aphostclient.dll (not in the spec) is
  a DLL w2m does not model, full slots stop.
- **The run:** the batch run's stream equals the Python's rows 0-256, row 257 in flight, and stops at row 257, RANDOM
  23.
- **Plant 1** (the page one slot off): the stream test failed, "row 49 of the call stream differs from the Python batch
  run's". Restored byte for byte.
- **Plant 2** (the canonical name recorded) did not compile and reached no judge; its replacement 2b (the spelling
  uppercased) failed the stream test, "row 52 of the call stream differs from the Python batch run's". Restored byte for
  byte.
- **Plant 3** (a DLL the spec lacks loaded anyway): `a_load_of_a_name_found_nowhere_is_0` failed, "called
  `Result::unwrap_err()` on an `Ok` value: 20480". Restored byte for byte.
- **Departures:** plant 2 replaced as above.
