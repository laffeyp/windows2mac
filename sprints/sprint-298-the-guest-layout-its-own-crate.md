# Sprint 298 — The guest layout its own crate

```yaml
---
id: 298
status: active
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Low 1 at 297: `pe_loader` and `virtual_memory` depend on `guest_runtime`, the crate that builds the C
runtime's image (`lib.rs`: `build`, the clang call, `frozen_root`), for its `layout` module alone (3 uses and 1).
`layout.rs` (241 lines) imports nothing.

- **The work:** a crate `guest_layout`, no dependencies, holding `layout.rs` whole as its `lib.rs`; `guest_runtime`
  depending on it and re-exporting it as `layout`, so every `guest_runtime::layout` path stays; `pe_loader` and
  `virtual_memory` depending on `guest_layout` in place of `guest_runtime`; `KEY_INPUTS` (`w2m_vm/src/cache_file.rs`
  74) gaining `crates/guest_layout` beside `crates/guest_runtime`, as `tests/cache_file.rs`' walk of the maker's path
  dependencies requires once a maker crate reaches it; a test that `pe_loader` and `virtual_memory` name no
  `guest_runtime` in their manifests.
- **The key:** a layout change remakes the cache, as the words embed its addresses; a builder change still does,
  `guest_runtime` staying in the key. Narrowing the key to `guest_layout` alone waits on a measurement of what the
  maker reads.
- **Not here, the chain after it (kit rule 6, ≤2 files / one concept),** after 299, the record's roll (Deferred,
  Sprint 290, before 420 KB; 409,031 bytes at this card): 300, the monitor data's slots one home: the translator
  depending on `guest_layout` and its 11 private copies (`lower.rs` 40-61, 2747) and the case runner's four
  (`cases.rs` 41-45) derived from it; 301, the state block's fields held to `struct w2m_state` by the image's compile,
  through `W2M_STATE_*` defines and `guest_runtime::current_image` for the 14 builds of the current sources; 302, every
  `const` at any visibility one definition. `cpu_cases`' and `x86_decoder`'s edges reach `sha256`, not the layout.

## prerequisites

- 297 (closed)

## context_files

- `crates/guest_runtime/` (`Cargo.toml`, `src/lib.rs`, `src/layout.rs`); `crates/{pe_loader,virtual_memory}/Cargo.toml`
- `crates/w2m_vm/src/cache_file.rs` 74 and `tests/cache_file.rs`
- BLACKBOARD Surfaced, the review at 297

## signal contract

### Emits

- None; a crate's boundary, held by tests.

### Invariants

- Every run as held, the runtime image byte for byte (Sprint 133's judge), and every crate's tests.

## artifact contract

### Files modified

- `Cargo.toml` (members), `crates/guest_layout/` new (`Cargo.toml`, `src/lib.rs` moved from
  `guest_runtime/src/layout.rs`), `crates/guest_runtime/` (`Cargo.toml`, `src/lib.rs`), `crates/{pe_loader,virtual_memory}/Cargo.toml`,
  `crates/w2m_vm/src/cache_file.rs`, `crates/w2m_vm/tests/layout_constants.rs`, `Cargo.lock`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the image equal to the Python build's; no `guest_runtime` in the two manifests; `guest_layout`
  with no dependencies.
- **Planted faults**, each changing what its judge reads:
  - (1) `crates/guest_layout` struck from `KEY_INPUTS`: the cache key's test against the manifests must name it.
  - (2) `guest_runtime` added back to `virtual_memory`'s dependencies: the manifest test must name `virtual_memory`.

## done criteria

The guest's layout is a crate of facts; the loader and the arena reach it without the image's builder.

## self-review checklist

- [x] Scope names one verifiable property: the layout reached without the builder.
- [x] context_files cite the crate, the manifests and the tests.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what its judge reads.
- [x] One concept: one crate's boundary; the slots, the state fields and the wider class 300-302.

## result

