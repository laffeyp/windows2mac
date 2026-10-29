# Sprint 205 — A DLL loaded after the start

```yaml
---
id: 205
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 198", item 4: the batch run's stream equals the Python's rows 0-499 and stops at
row 500, MODULE 15, UnityPlayer.dll's LoadLibrary of MonoBleedingEdge\EmbedRuntime\mono-2.0-bdwgc.dll (4,974,728
bytes; no TLS directory, entry point RVA 0x31953c), a DLL file after the start. windows.module.search's two binaries stop
at the same request for w2m_search\w2m_found.dll. `_load_dll` (`src/w2m/vm_backend.py` 3906-3951) maps such a file
and `_init_stub` (3988-4023) runs its DllMain as the caller's next code.

- **The load:** `modules.rs` resolves the name through the process's file bridge (`FileBridge::host_file`, replacing
  `AppDirectory`, Sprint 204's departure), parses the file, places it in the VirtualAlloc arena (`place_image`), maps
  it (`pe_loader::map_image`), binds its imports (a loaded DLL's export, else `import_thunk`), snapshots it, and records
  it in the module table, the PEB loader lists and the trace ranges. A DLL with TLS callbacks or a TLS directory stops
  by name, as the Python's does for callbacks; no row loads one.
- **DllMain:** the init stub as the Python writes it, byte for byte: the trace closed around a traced image's DllMain,
  DllMain(base, DLL_PROCESS_ATTACH, NULL), a FALSE return to the DLL_INIT_FAILED thunk, RAX = base and a jump through
  the caller's return slot; its address is the MODULE request's second answer at TRANSPORT+48 (vm_backend 3873).
- **The runs:** the batch run, the guests and the module families run again; each stop moved is named.

## prerequisites

- 204 (closed)

## context_files

- `src/w2m/vm_backend.py` 3870-3874 (the second answer), 3889-3951 (`_module_request`, `_load_dll`), 3988-4023
  (`_init_stub`), 2970 (`_write_loader_data`); `src/w2m/loader.py` 40 (`DLL_INIT_FAILED`);
  `crates/w2m_vm/src/{modules.rs, host_request.rs, process.rs, guest.rs, file_bridge.rs}`; `crates/virtual_memory`
  (`place_image`, `snapshot_image`); `crates/pe_loader` (`map_image`)

## signal contract

### Emits

- None: the judges are `batch_run.rs`, `family_runs.rs` and unit tests.

### Invariants

- No Python file added or changed; the stream's rows 0-499 still equal; the guests and module families as before or
  further, each move named.

## artifact contract

### Files modified

- `crates/w2m_vm/src/{modules.rs, host_request.rs, guest.rs}`, `store/batch_run/first-stop-2026-10-28.tsv`,
  `store/family_runs/module-stops-2026-10-28.tsv`, `docs/step-4-census-2026-10-28.md`

### Command exit codes

- `batch_run.rs`, `family_runs.rs`, the unit tests and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the init stub's bytes equal the Python's for a fixed base, entry, trace header and slot; the
  stream passes row 500 and equals the Python's to its next stop, named; windows.module.search's runs conform or stop
  later, named.
- **Planted faults:** (1) an import bound to the thunk though a loaded DLL exports it: the stream or a family row names
  it; (2) DllMain's reason 0 for 1: the stub's bytes test names the byte; (3) the trace left open around a traced
  DllMain: the stream names the first extra row.

## done criteria

The Rust host maps a DLL file loaded after the start and runs its DllMain as the Python host does, and the batch run
passes row 500.

## self-review checklist

- [x] Scope names one verifiable property: row 500 and on equal the Python's.
- [x] context_files cite the Python to read first.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: a DLL loaded after the start.
