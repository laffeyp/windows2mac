# Sprint 265 — The cache key over its own inputs

```yaml
---
id: 265
status: active
phase: 6
pass_kind: functional
---
```

## scope

The translation cache's key hashes the whole build's inputs (`built_from::crates_tree`: `crates/`, the manifests, the
toolchain, `specs/`), so any edit under `crates/`, a test's included, names a cache no file holds: each sprint remakes
it, and in Sprint 263 an edit to `run_loop.rs` while records ran voided a batch (Observation 4). This card makes the key
hash what the cached words are made from and nothing else, held by a test against the crates' own manifests.

- **Where it stands, by lookup (2026-10-30):**
  - `cache_file::key(image, base, translator_tree, exits)`: the image's bytes, its base, a tree id, and the addresses
    the words embed (the dispatcher, the slice stub, the state block, the runtime's functions); `w2m-vm`'s
    `cached_image` passes `built_from::crates_tree(root)`, the tree of `built_from::INPUTS`.
  - The maker (`cache_file.rs`) uses `guest_runtime`, `pe_image`, `translator` and `image_walk`; by their manifests'
    `path =` lines, `translator` needs `a64_encoder`, `x86_decoder` and `guest_runtime`, `image_walk` needs `pe_image`
    and `x86_decoder`, `x86_decoder` needs `guest_runtime` and `pe_image`; `x86_decoder`'s build script reads `specs/`.
  - The monitor code, the installer, the run loop and the tests in `w2m_vm` change no cached word; the addresses they
    place, which the words embed, are hashed apart as the exits.
  - Searched for `cache_file.rs`, `built_from.rs`, "key" in Deferred, Surfaced and `docs/surfaced-archive-2026-10-30.md`:
    nothing on the key's inputs.
- **The readers:** `w2m-vm --make-cache` and `--cache`; the record script; every card that edits a crate while a cache is
  in use.
- **The work:**
  - `built_from::tree_of(root, inputs)`, the tree id of any list of paths, `crates_tree` its use for `INPUTS`.
  - `cache_file::KEY_INPUTS`: the maker's crates and their path dependencies, `cache_file.rs` itself, `specs/`, the
    lockfile and the toolchain; `cached_image` hashes `tree_of(root, &KEY_INPUTS)`.
  - A test: every crate reachable by `path =` from the maker's four is in `KEY_INPUTS`, and `cache_file.rs` is.
  - A check by run: the cache made, a line of `w2m_vm/tests/run_loop.rs` changed, and the key unchanged; a line of the
    translator changed, and the key changed; each change restored by `cp` and checked by `cmp`.
- **Not here:** the burst statistic (card 266).

## prerequisites

- 264 (closed)

## context_files

- `crates/w2m_vm/src/built_from.rs`; `src/cache_file.rs` (`key`); `src/bin/w2m-vm.rs` (`cached_image`)
- `crates/{translator,image_walk,x86_decoder,pe_image,guest_runtime,a64_encoder}/Cargo.toml`

## signal contract

### Emits

- None; a key's inputs, held by a test.

### Invariants

- The cached words as before; only the key's tree input narrows.

## artifact contract

### Files modified

- `crates/w2m_vm/src/` (`built_from.rs`, `cache_file.rs`, `bin/w2m-vm.rs`), a test, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the key before and after a `run_loop.rs` edit (equal) and a translator edit (different).
- **Planted faults:**
  - (1) `x86_decoder` left out of `KEY_INPUTS`: the test names it.

## done criteria

The cache's key hashes the inputs its words are made from, held by a test against the manifests, and an edit outside
them leaves the key as it was.

## self-review checklist

- [x] Scope names one verifiable property: the key over its own inputs.
- [x] context_files cite the key, the tree, the maker and the manifests.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and one plant.
- [x] One concept: the key's inputs.

## result
