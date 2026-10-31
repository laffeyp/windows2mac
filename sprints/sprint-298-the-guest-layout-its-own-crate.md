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
- **Not here (kit rule 6, ≤2 files / one concept):** three cards, Deferred (Sprint 298) behind the run loop's cards,
  none being a precondition of the second vCPU: the monitor data's slots one home (the translator depending on
  `guest_layout` and its 11 private copies, `lower.rs` 40-61 and 2747, and the case runner's four, `cases.rs` 41-45,
  derived from it); the state block's fields held to `struct w2m_state` by the image's compile, through `W2M_STATE_*`
  defines and `guest_runtime::current_image` for the 14 builds of the current sources; every `const` at any visibility
  one definition. 299 is the record's roll (Deferred, Sprint 290, before 420 KB; 409,031 bytes at this card).
  `cpu_cases`' and `x86_decoder`'s edges reach `sha256`, not the layout.

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
- [x] One concept: one crate's boundary; the slots, the state fields and the wider class Deferred (Sprint 298).

## result

- **Rule 6 weighed (reviewer-3's Low 1 at 298):** the change touches 12 files against the kit's sweet spot of two
  (`sdd-kit-3/AGENTS.md` 235): the workspace's manifest and lockfile, the new crate's manifest and its moved source,
  `guest_runtime`'s manifest and `lib.rs`, `pe_loader`'s and `virtual_memory`'s manifests and re-exports,
  `cache_file.rs`' key and the test. One concept, at its floor: a crate move changes the manifests, the lockfile and
  the re-exports together or the workspace does not build; the slots, the state block and the wider class went to
  Deferred (Sprint 298).
- **The change** (WORK-COMMIT): `crates/guest_runtime/src/layout.rs` moved whole by `git mv` to
  `crates/guest_layout/src/lib.rs`, a crate with no dependencies; `guest_runtime` depending on it and re-exporting it
  (`pub use guest_layout as layout;`, `lib.rs` 11), so every `guest_runtime::layout` path stays; `pe_loader`'s three
  and `virtual_memory`'s one re-export read `guest_layout`, their manifests naming it in place of `guest_runtime`;
  `KEY_INPUTS` 11, `crates/guest_layout` beside `crates/guest_runtime`; the lockfile changed by the new package and
  the three edges alone; `the_loader_and_the_arena_reach_the_layout_without_the_image_builder` in
  `tests/layout_constants.rs`.
- **Expected values:** the 16 tests the change reaches passing before the pre-commit run, among them
  `the_image_rust_builds_is_the_python_builds_byte_for_byte`, `the_key_covers_every_crate_the_cache_is_made_from`, the
  constants tests and `guest_layout`'s `the_chained_offsets_land_where_the_python_host_puts_them`; before the commit, format, lint, the Linux check, `design_check`, the record check and the five crates' doc tests passing, and the batch-run step; its tests, stopped under WORKING_AGREEMENT's amendment at 298, run in the gates alone.
- **The gates:** GATES-LINE
- **Plant 1,** restored by `cp` and checked by `cmp`: `"crates/guest_layout",` in `KEY_INPUTS` replaced by a second
  `"crates/pe_image",`, the array's length kept; `the_key_covers_every_crate_the_cache_is_made_from` failed,
  "crates/guest_layout, which the cache is made from, is not among the key's inputs".
- **Plant 2,** restored by `cp` and checked by `cmp`: `guest_runtime = { path = "../guest_runtime" }` written into
  `virtual_memory`'s manifest; `the_loader_and_the_arena_reach_the_layout_without_the_image_builder` failed,
  "depending on guest_runtime for the layout: [\"virtual_memory\"]". The plant's resolve wrote the edge into
  `Cargo.lock`; a build after the restore resolved it out, the lockfile's diff read after.
- **A judge that passed too much, found before the commit:** the manifest test's first form matched a line starting
  `guest_runtime` and a `[dependencies]` header, so `[dependencies.guest_runtime]` and a `[target.'cfg(..)'.dependencies]`
  table passed; it reads every line less comments, `guest_layout`'s description reworded from "no dependencies". The
  pre-commit run in progress, past its doc tests, was stopped by `TaskStop`, no process of it left. Plant 2 was run
  again on the new form and failed as above; `[dependencies.guest_runtime]` with its `path` appended to
  `virtual_memory`'s manifest failed the same way, restored by `cp` and checked by `cmp`.

