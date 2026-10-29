# Sprint 205 — A DLL loaded after the start

```yaml
---
id: 205
status: closed-partial
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

## result

Closed partial: the code did not land. It stands on branch sprint-205-wip (worktree base cb013d8e8, the card
ccac604c4, the work uncommitted there) and lands after Sprint 207 (the Decision "Sequence after Sprint 205").

- **The load, written:** `modules::load_dll` (the bridge's resolution, `place_image`, `map_image`, imports bound to a
  loaded DLL's export or the import thunk, `snapshot_image`, `dll_paths`, PEB.Ldr written again through
  `process::write_loader_data` over a `LoaderList`, `trace_ranges`); `Guest::init_stub` and `init_stub_code`
  (vm_backend 4002-4021); the module request's second answer is `pending_init`. mono-2.0-bdwgc.dll (4,974,728 bytes,
  no TLS directory, entry RVA 0x31953c) maps and its DllMain runs.
- **The run:** the stream equals the Python's rows 0-923, 925 calls traced, and the run stops at row 925, MAP_VIEW 29
  (MapViewOfFile of mscorlib.dll, 0x3b9a00 bytes). Row 924, CreateFileMappingW, differs in its first stack argument:
  0x165e00000000 for 0x13fc00000000, the value in all four stored Python runs.
- **The cause, measured:** the word is a residue of mono's /GS cookie, 0x165e8e8cc734 on the Rust host (guest memory at
  0x7ffe00003050 and the heap from 0x200028c60; stack canaries cookie ^ RSP) and 0x13fc8e8cff34 on the Python's (row
  939). At mono's load the Rust game process had run 2,648,351 instructions with a clock warp of 609; the cookie's
  bits 32-47, ((E + c) >> 32 ^ c) & 0xffff with E = 134362624920000000, give c = 19,526 for Rust's and exactly
  18,916 = 2,648,351 / 140 for the Python's, and the low words agree, 0x3da2 ^ 0x05a2 = 0x3800. The instruction
  counts are equal; the Rust warp comes from six catch-ups after the child's slices, which the Python host does not
  make. The judge comes first: the Decision's Sprint 206 exit ledger.
- **Plant 1** (an import bound to the thunk): not run; the code did not land.
- **Plant 2** (DllMain's reason 0): not run; the code did not land.
- **Plant 3** (the trace left open): not run; the code did not land.
- **Departures:** one `sed` with an address range in `process.rs`, which edit_guard classes as multi-line (it renamed
  `areas.process` on the 10 lines of `write_loader_data` that held it, adding none), on the branch; it is quoted in
  the tail and redone with the Edit tool when the code lands. The temporary prints and dumps used for the measurement
  stay off the branch's commit.
