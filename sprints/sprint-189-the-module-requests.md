# Sprint 189 — The module requests

```yaml
---
id: 189
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 185": each card serves the stop the last batch run named. Sprint 186's run stops
first at MODULE 15, UnityPlayer.dll's LoadLibrary of "api-ms-win-core-synch-l1-2-0" (`store/batch_run/first-stop-
2026-10-28.tsv`). Row 5a counts the module ops at 714 in the Python's batch run (15-17), MODULE_AT 18 already served.

- **The requests:** `src/w2m/modules.py` (190 lines) in Rust, over the state the guest already holds (the program's
  range and exports, the DLLs' ranges and exports, `ProcessArea`'s kernel32, kernelbase, ntdll and system images, the
  loader's import thunks, `Guest::import_thunk`): MODULE 15 (`module`: the canonical name; kernel32, kernelbase, the
  program; `API_SET_HOSTS`; any other `api-ms-win-core-*` set as kernelbase; another API set a stop by name; ntdll; the
  DLLs; the system images; GetModuleHandle's 0 for a name not loaded), PROC 16 (`proc`: the program's exports, a
  DLL's, a system image's thunk or the program's own import thunk, kernel32's and kernelbase's thunks from
  `specs/windows_exports.json`, 0 for a name not exported) and MODULE_PATH 17 (`file_name`, the path written as
  vm_backend 3897-3906 writes it), each served as `_serve_host_request` 3870-3874 serves them, the DllMain stub word
  at the transport's 48 left 0. A LoadLibrary that would map a DLL file or a system DLL after the start (`load_dll`,
  `load_system`) stays a stop by name, a later card's.
- **The run:** the batch run on the Rust host to its next stop, held in the stops file as the card that moves it.

## prerequisites

- 188 (closed)

## context_files

- `src/w2m/modules.py` (whole), `src/w2m/vm_backend.py` 3866-3910 (`_module_request`), `tools/guest_runtime/module.c`
  (the requests' arguments), `crates/w2m_vm/src/host_request.rs`, `crates/w2m_vm/src/guest.rs` (`import_thunk`, the
  image and DLL ranges), `crates/w2m_vm/src/process.rs` (`ProcessArea`), `specs/windows_exports.json`

## signal contract

### Emits

- None: no producer runs; the judges are the batch-run test and unit tests.

### Invariants

- No Python file added or changed; the 15 guests' stops as before, and the batch run's stop moved past MODULE 15.

## artifact contract

### Files modified

- `crates/w2m_vm/src/modules.rs` (new), `crates/w2m_vm/src/host_request.rs`, `crates/w2m_vm/src/lib.rs`,
  `store/batch_run/first-stop-2026-10-28.tsv`, `docs/step-4-census-2026-10-28.md`, `WORKING_AGREEMENT.md`

### Command exit codes

- the module unit tests, `batch_run.rs` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the batch run passes UnityPlayer.dll's LoadLibrary of api-ms-win-core-synch-l1-2-0 (kernelbase's
  handle) and stops at a later request, named in the held file; each rule of `modules.py`'s docstring held by a unit
  test.
- **Planted faults:** (1) an `api-ms-win-core-*` set answered as kernel32's handle: the unit test of that row fails; (2)
  `_canonical` without its ".dll" suffix rule: the unit test of a name with no extension fails.

## done criteria

The Rust host answers the module requests as `modules.py` does, and the batch run's held stop is past MODULE 15.

## self-review checklist

- [x] Scope names one verifiable property: the module requests served as the Python serves them.
- [x] context_files cite the Python to read in full first.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: the module requests.
