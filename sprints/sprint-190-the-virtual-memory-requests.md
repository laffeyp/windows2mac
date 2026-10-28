# Sprint 190 — The virtual-memory requests

```yaml
---
id: 190
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 185": Sprint 189's batch run on the Rust host stops at VIRTUAL_ALLOC 19 with
arguments 0x0 0x1ffff000 0x2000, a reserve with no base, after 60 host requests (`store/batch_run/first-stop-
2026-10-28.tsv`). The arena the four virtual-memory requests use is ported and judged (`crates/virtual_memory`,
Sprint 146's `crates/w2m_vm/tests/virtual_memory.rs` against the probes' rows); the host does not call it.

- **The requests:** `host_request::serve` answers VIRTUAL_ALLOC 19 (`alloc(a0, a1, a2, a3)`: the address, the error
  second), VIRTUAL_FREE 20 (`free(a0, a1, a2)`), VIRTUAL_PROTECT 21 (`protect(a0, a1, a2)`: the old protection, the
  error above it, in the second word) and VIRTUAL_QUERY 22 (`query(a0)`: MEMORY_BASIC_INFORMATION's 48 bytes written at
  a1, its length the result; 0 and 87 for no region), as `_virtual_request` (vm_backend 4028-4047) and its dispatch
  (3865-3869) do: the second answer at the transport's 48, the result at 56. The arena's refusal (`Unimplemented`) is a
  stop by name. MAP_VIEW 29 needs the file bridge and stays a later card's.
- **The run:** the batch run on the Rust host to its next stop, held as the card that moves it.

## prerequisites

- 189 (closed)

## context_files

- `src/w2m/vm_backend.py` 3857-3869, 4028-4047; `src/w2m/virtual_memory.py` 123-254;
  `crates/virtual_memory/src/lib.rs` (`alloc`, `free`, `protect`, `query`, `Info`); `crates/w2m_vm/src/guest.rs`
  (`GuestView`'s `GuestPages`); `crates/w2m_vm/src/host_request.rs`; `tools/guest_runtime/virtual_memory.c` (the
  requests' arguments)

## signal contract

### Emits

- None: no producer runs; the judges are the batch-run test and a unit test of the request's words.

### Invariants

- No Python file added or changed; the arena's judge (`virtual_memory.rs`) unchanged; the guests' stops as before.

## artifact contract

### Files modified

- `crates/w2m_vm/src/host_request.rs`, a test of the four requests' words, `store/batch_run/first-stop-2026-10-28.tsv`,
  `docs/step-4-census-2026-10-28.md`, `WORKING_AGREEMENT.md`

### Command exit codes

- the request test, `batch_run.rs`, `virtual_memory.rs` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the batch run passes its first VirtualAlloc and stops at a later request, named in the held file;
  VIRTUAL_QUERY's 48 bytes laid out as `struct.pack("<QQIHHQIIIi", ...)` lays them.
- **Planted faults:** (1) the result and the second answer swapped on the transport page: the batch run's held stop
  moves, or the request test names the word; (2) VIRTUAL_QUERY's RegionSize written at the wrong offset: the request
  test names the field.

## done criteria

The Rust host serves the four virtual-memory requests as the Python does, and the batch run's held stop is past
VIRTUAL_ALLOC 19.

## self-review checklist

- [x] Scope names one verifiable property: the four requests served as the Python serves them.
- [x] context_files cite the Python to read first.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: the virtual-memory requests.
