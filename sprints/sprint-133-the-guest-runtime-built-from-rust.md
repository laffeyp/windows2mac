# Sprint 133 — The guest runtime built from Rust

```yaml
---
id: 133
status: active
phase: 6
pass_kind: functional
---
```

## scope

The walking skeleton's second link. A crate, `guest_runtime`, builds the C guest runtime image as
`src/w2m/guest_runtime.py` does, with no Python: the memory layout the runtime is compiled against (the 60 defines
`vm_backend.runtime_defines()` passes, each constant carried from `src/w2m/vm_backend.py` 80-355 and
`src/w2m/scheduler.py` 28-49 as the same expression, cited by line); the five headers the build writes
(`cpuid_table.h` from `specs/guest_cpuid.json`, `code_page_tables.h` from probe run 38044229348,
`system_messages.h` from `specs/system_messages.json`, `approximation_tables.h` from `specs/approximations.json`, and
`api_ids.h` from the services the runtime's C defines), each byte for byte as the Python wrote it; the same cache key
(sha256 over every input); clang over the 51 runtime files and the 3 compiler files, and `ld -preload` at
`MONITOR_VA + RUNTIME_IMAGE`; and the MH_PRELOAD image read back into segments and symbols. Its judge: the image Rust
links equals, byte for byte, the image the Python build cached under the same key in `build/guest_runtime/`, built
by the frozen host from the same sources. A wrong constant or a header off by one byte changes the key or the image,
and the test fails.

## prerequisites

- 132 (closed)

## context_files

- `src/w2m/guest_runtime.py` (read in full: `_compile`, `build`, `_parse`, `service_table`, `api_header`)
- `src/w2m/vm_backend.py` 72-355 (the layout), 1075-1104 (`runtime_defines`), 1179-1187 (`build_runtime`)
- `src/w2m/scheduler.py` 20-50 (the run table's offsets)
- `src/w2m/cpuid.py`, `code_pages.py`, `system_messages.py`, `approximate.py` (the header generators, read in full)
- `crates/hypervisor/` (Sprint 132)

## signal contract

### Emits

None. The build puts nothing on a record; the record comes with the guest (a later link).

### Consumes

`tools/guest_runtime/*.c`, `runtime.h`; `tools/vm_compiler/vm_compiler.c`, `x86_decode.c`, `a64_encoder.c`, `*.h`;
the specs and the probe run above.

### Invariants

- No Python file is added or changed; no crate is added.
- Rust writes its objects under its own cache directory and never into `build/guest_runtime/`, which it only reads.

## artifact contract

### Files created

- `crates/guest_runtime/` (layout, headers, services, build, its tests)

### Files modified

- `Cargo.toml`, `Cargo.lock`, `WORKING_AGREEMENT.md` (canonical homes)

### Content assertions

- Every layout constant cites the line of `vm_backend.py` or `scheduler.py` it carries.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected value:** the Rust key names a file in `build/guest_runtime/`, and the image Rust links equals it byte
  for byte; the symbols Rust reads from it include every service the runtime's C defines.
- **Planted fault 1:** one define changed by one (`W2M_TIME_SLICE` 20,001): the key names no Python image, and the
  test says so by name.
- **Planted fault 2:** a header byte changed (the cpuid table's first row): the key differs again.
- **Planted fault 3:** a service defined twice is refused by name, as `service_table` refuses it.

## done criteria

The C guest runtime is built from Rust into the image the frozen Python host built, byte for byte.

## self-review checklist

- [x] Scope names one verifiable property: the image equal to Python's, byte for byte.
- [x] context_files list every Python file whose behavior is carried.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: an expected value and three planted faults.
- [x] One concept: the runtime image.

## result

- `crates/guest_runtime`: `layout.rs` (the 57 named defines and 12 per-thread monitor values `runtime_defines()`
  passes, each constant the same expression as its line in `vm_backend.py` or `scheduler.py`), `headers.rs` (the four
  generated headers), `services.rs` (the services scanner, in place of the Python's regex; `api_ids.h`), `lib.rs`
  (the sources, the cache key, clang and `ld -preload` under `build/guest_runtime_rust/`, the MH_PRELOAD reader, and
  SHA-256 checked against FIPS 180-4's three examples).
- Expected value, held on the first run: the key Rust computes, `91e639a070f276e16a39c2444a91b1adf91f8406d489cfcc8a3d917478629b5d`,
  names an image in `build/guest_runtime/` that the Python build made, and the image Rust links equals it byte for
  byte: 3,169,760 bytes, 2 segments, 706 symbols, every service the runtime's C defines among them.
- Planted faults: `W2M_TIME_SLICE` 20,001 gives a key with no Python image; one byte added to `runtime.c` gives
  another key; a service defined twice is refused by name (a unit test, as `service_table` refuses it). The card named
  a changed header byte for the second plant; a source byte changes the key the same way and needs no edit to the
  generators.
- Tests: guest_runtime 9 (6 unit, 3 against the Python's image); the gates pass.
