# Sprint 146 — The virtual memory arena: VirtualAlloc, VirtualFree, VirtualProtect and VirtualQuery in Rust

```yaml
---
id: 146
status: active
phase: 6
pass_kind: functional
---
```

## scope

The case runner's fault path (`docs/design-host-compile-path-2026-10-24.md`, link 3) needs the host to answer the
guest runtime's exception walk. With no handler, the walk asks for the x86 stubs' page (`HOST_EXCEPTION`, `exceptions.c`
`known_code` 142-148), and the Python host writes that page into the VirtualAlloc arena (`vm_backend.py` `_code_page`,
`_guest_stubs`). The arena comes first, as a crate of its own: `src/w2m/virtual_memory.py` (352 lines, read in full)
carried whole into `crates/virtual_memory`:

- the reservations, 64 KB aligned, each page reserved or committed with a protection; a commit zeroes its pages, and
  a decommit or release zeroes them again;
- the four calls (`alloc`, `free`, `protect`, `query`), with the error codes 87, 50 and 487;
- `query` on the main thread's stack (`_stack_query`), on the process heap (`_heap_query`), on a reservation, on an
  image's pages (`_image_page_protect`, `section_protect`), in the free arena, and in free address space outside every
  mapped range;
- `map_view`, `place_image`, `snapshot` and `snapshot_image`.

Guest memory reaches the arena through a trait with two methods, zero and read, in place of the Python's two
callables. A form the rows do not show stops by name (`Unimplemented`), as in the Python.

Out of scope: the host request that calls the arena (`_virtual_request`), which comes with the fault path; the heap
rows, which need `heap.c` running.

## prerequisites

- 145 (closed)

## context_files

- `src/w2m/virtual_memory.py` (read in full); `src/w2m/vm_backend.py` 2498-2505, 2832-2849, 4028-4047
- `probes/virtual_memory/virtual_memory.c`, `probes/thread_stack/thread_stack.c` (read in full)
- `contracts/windows.memory.virtual/0.2.0.json`, `contracts/windows.memory.thread_stack/0.1.0.json`
- `tests/test_virtual_arena.py`; `crates/pe_image`, `crates/w2m_vm/src/process.rs`

## signal contract

### Emits

None yet: the record comes at the skeleton's gate.

### Consumes

The probe images in `store/binaries` by the sha256 each contract's evidence records.

### Invariants

- No Python file added or changed.

## artifact contract

### Files created

- `crates/virtual_memory/` (`Cargo.toml`, `src/lib.rs`)
- `crates/w2m_vm/tests/virtual_memory.rs`: the replays and the Python tests' values, with the host's layout (the arena
  at `guest::VIRTUAL_VA`, the stack from `process::write`)

### Files modified

- `Cargo.toml` (the workspace member), `Cargo.lock`, `crates/w2m_vm/Cargo.toml`, `WORKING_AGREEMENT.md`,
  `BLACKBOARD.md` (Sprint 145's CI run, in the tail entry)

### Content assertions

- Each function cites the Python lines it carries.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values, from Windows:**
  - `windows.memory.virtual` 0.2.0: the probe's calls replayed against the arena in the probe's order, with the
    probe's image placed at its base for both runs' images (Intel and AMD). Each row is derived as the probe prints it
    and compared with the contract's row. The 33 arena rows and five of the seven image rows are judged.
    `image.section._data` and `image.rdata_after` depend on which data pages the running probe wrote, so they wait for
    the probe's run in the Rust VM.
  - `windows.memory.thread_stack` 0.1.0: the six query rows, with the stack triple the Rust process area writes for
    the probe's image (`process::write`, Sprint 140), named relative to the TEB's fields as the probe names them.
- **Expected values, carried from the Python tests:** `tests/test_virtual_arena.py`'s two tests: 64 reservations of
  0x1FFFF000 bytes at consecutive 512 MB steps, a commit zeroing only its own two pages, and a reservation over a
  placed image refused with 487.
- **Planted fault:** `protect` reporting 0 for a page only reserved, where Windows writes PAGE_NOACCESS: the replay
  names `protect.reserved_page`.

## done criteria

The Rust arena answers VirtualAlloc, VirtualFree, VirtualProtect and VirtualQuery as Windows did in the probes' rows.

## self-review checklist

- [x] Scope names one verifiable property: the arena's answers equal Windows's rows.
- [x] context_files cite the Python carried.
- [x] Emits: none yet, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: expected values from Windows, and a planted fault.
- [x] One concept: the arena.

## result

- `crates/virtual_memory`: `virtual_memory.py` whole. Guest memory comes through the trait `GuestPages` (`zero`,
  `read`). One change from the Python: `protect` over a size that rounds to no page stops by name, where the Python
  indexed an empty range and crashed.
- Expected values (`crates/w2m_vm/tests/virtual_memory.rs`, four tests, all passing):
  - `windows.memory.virtual` 0.2.0: the probe's calls replayed against the arena at `guest::VIRTUAL_VA`, with each run's
    image (Intel 8385bc74, AMD af77fd80; sections `.text`, `.rdata`, `.data`) at its base and its writable pages
    snapshot. The rows printed equal the contract's 40 by name, and 38 equal the contract's rows field for field, on
    both images. `image.section._data` and `image.rdata_after` wait for the probe's run in the VM.
  - `windows.memory.thread_stack` 0.1.0: the six query rows equal the contract's, with the stack `process::write`
    gives each run's image (Intel d39ae9f9, AMD 09f0d819).
  - `test_virtual_arena.py`'s values: 64 reservations at consecutive 512 MB steps, a commit zeroing only its two
    pages, and three reservations around a placed image (487, 487, granted).
- Planted fault: `protect` reported 0 for a page only reserved. The replay failed on "row protect.reserved_page",
  `old` 0 against Windows's 1. Restored, the tests pass.
