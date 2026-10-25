# Sprint 138 — The loader in Rust

```yaml
---
id: 138
status: done
phase: 6
pass_kind: functional
---
```

## scope

The walking skeleton's seventh link: the guest program placed in memory as Windows places it. A crate, `pe_loader`,
carries `src/w2m/loader.py` item for item on top of `pe_image` (Sprint 137): `Memory` and its regions, `map_image`
(headers, sections, DIR64 and HIGHLOW relocations, any other type refused), the placement of each DLL the program
imports (its preferred base when free, else from `DLL_BASE` up, 64 KB aligned), import binding (a supplying DLL's
export, else a host thunk 16 bytes apart from `THUNK_BASE`; a missing export refuses the load), the loader stub that
runs TLS callbacks and each DllMain deepest dependency first, and the static TLS array. Its judges:

- `tests/test_loader_images.py`, carried: static_tls_dll.dll mapped at its preferred base and at `DLL_BASE` differs
  at exactly its 780 DIR64 sites, each by the delta; loader.exe binds loader_dll.dll's two exports, starts the DLL
  first through the stub, and makes no thunk for the DLL's names; a renamed export refuses the load naming it.
- The Windows rows the loader decides (`contracts/windows.loader.static_import` row `dll.base`: the DLL at its
  preferred base, 64 KB aligned, above 4 GB; `contracts/windows.loader.static_tls` rows `tls.index` and `tls.array`:
  the DLL's index written at its AddressOfIndex is 1, the array's slot 1 holds the template's bytes followed by the
  zero fill, and slot 0, the program's, is NULL).
- The loader stub's bytes equal clang's assembly (`--target=x86_64-pc-windows-msvc`) of the instructions `_stub`'s
  comments name, with the addresses of the loader.exe load.

## prerequisites

- 137 (closed)

## context_files

- `src/w2m/loader.py` (307 lines, read in full), `tests/test_loader_images.py` (64 lines, read in full)
- `contracts/windows.loader.static_import/0.1.0.json`, `contracts/windows.loader.static_tls/0.1.0.json`

## signal contract

### Emits

None yet: the skeleton's record comes with the guest program.

### Consumes

Nothing at run time.

### Invariants

- No Python file is added or changed.

## artifact contract

### Files created

- `crates/pe_loader/` (`src/lib.rs`, `tests/images.rs`)

### Files modified

- `Cargo.toml`, `Cargo.lock`, `WORKING_AGREEMENT.md` (canonical home), `BLACKBOARD.md` (Sprint 137's CI run)

### Content assertions

- Each item in `crates/pe_loader/src/lib.rs` cites the `loader.py` lines it carries.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the three Python tests' values; the contracts' rows above; the stub equal to clang's bytes.
- **Planted fault:** the relocation delta applied as `delta + 1`: the 780-site test fails naming the first site.

## done criteria

The program and the DLLs it imports are placed, relocated, bound and started from Rust as `loader.py` placed them, and
the rows Windows showed for placement and static TLS hold.

## self-review checklist

- [x] Scope names one verifiable property: the loader's placement, binding, stub and TLS against the Python's tests,
  the Windows rows and clang.
- [x] context_files cite every Python line carried.
- [x] Emits: none yet, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: expected values and a planted fault.
- [x] One concept: the loader.

## result

- `crates/pe_loader`: `Memory` and its regions, `map_image`, the placement of each supplied DLL, import binding, the
  loader stub and the static TLS array, each citing `loader.py`'s lines. `init_order` and `dependency_order` index
  `images`, where the Python held the images themselves.
- Deviations: where the Python's bytearray slice assignment would have grown a region (a section, a relocation, an
  IAT slot or the TLS index past SizeOfImage), the load is refused; the Python's `on_write` hook belongs to the VM and
  is not carried.
- Expected values (`tests/images.rs`, 6 tests): static_tls_dll.dll at its preferred base and at `DLL_BASE` differs at
  exactly its 780 DIR64 sites, each by the delta; loader.exe binds `loader_dll_call` and `loader_dll_count` to the
  DLL's exports, starts at `LOADER_STUB` with the DLL in `init_order`, and makes no thunk for the DLL's names; a
  renamed export refuses the load naming `loader_dll_call`; the DLL sits at its preferred base, above 4 GB and 64 KB
  aligned (`dll.base`); the DLL's TLS index is 1, slot 0 is NULL and slot 1 holds the template with its zero fill
  (`tls.index`, `tls.array`); the stub's 73 bytes equal clang's assembly.
- Planted fault: the DIR64 delta applied as `delta + 1`: "the site at RVA 0xe258", 140696686166017 against
  140696686166016; restored, the tests pass.
- Tests: pe_loader 6.
