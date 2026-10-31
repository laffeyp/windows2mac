# Sprint 261 — The cache file in the guest

```yaml
---
id: 261
status: done
phase: 6
pass_kind: functional
---
```

## scope

The data half of card 5 of `docs/design-translation-cache-2026-10-30.md` (choices I and II, Sprint 260): the cache file
at version 2, each block's fix-ups in it, mapped read-only into the guest at one region when a run uses a cache, its
bytes there the file's. The installer that reads it is card 262. The design's review, which Sprint 260 asked for
before card 5, has not come; this half rests on choices I and II alone, and the review's dispositions, when it comes,
go in the next card's commit.

- **Where it stands, by lookup (2026-10-30):**
  - The file (`cache_file::to_bytes`, `from_bytes`, version 1): the magic, the version, the key, the base, four
    counts, then the blocks (eight `u32`s each, in start order), the words, the marks, the page index.
  - `CACHE_CODE_VA` is the address the file's words are assembled at (`cache_file::make`) and `relocate` moves them
    from (`HostCompiler::install`); it is never mapped, and `tests/cache_layout.rs`' second test holds every exit within
    a branch's reach of it. `CACHE_DATA_VA` has no reader since Sprint 256.
  - The guest's memory is one allocation of `Layout::size` and its maps (`guest.rs` 229-262, 775), a little over 513
    GiB of the 40-bit IPA's 1 TiB.
  - `relocate` re-encodes a `B` or `BL` whose target leaves the block (Sprint 256).
  - Searched for `cache_file.rs`, `layout.rs`, `guest.rs` (`Layout`) in Deferred, Surfaced and
    `docs/surfaced-archive-2026-10-30.md`: Deferred (Sprint 259), the ledger's share, card 262's records; census row 2
    (the loader's tests), which a new map bears on.
- **The readers:** card 262's installer; `HostCompiler::install`; `w2m-vm --make-cache` and `--cache`.
- **The work:**
  - Version 2: each block's fix-ups, the word indices `relocate` re-encodes, after the page index; `make` computes
    them by `relocate`'s own rule (one rule, one home); `from_bytes` refuses version 1 by name.
  - `CACHE_DATA_VA` and its size removed; `CACHE_FILE_VA` and its size (256 MiB) below the assembly range; the layout
    test rewritten for it, its IPA bound held.
  - `Layout::new` given the file's size when a run uses a cache, mapping `CACHE_FILE_VA` read-only to its own part of
    the guest's memory; `w2m-vm --cache` copies the file's bytes there; with no cache, the layout as before.
  - Tests: the fix-ups equal the words `relocate` changes, block for block, over the fixtures; a guest built with a
    cache holds the file's bytes at `CACHE_FILE_VA`; with none, `Layout` unchanged.
- **Not here:** the installer (card 262).

## prerequisites

- 260 (closed)

## context_files

- `docs/design-translation-cache-2026-10-30.md` (card 5, I and II); `crates/w2m_vm/src/cache_file.rs`
- `crates/guest_runtime/src/layout.rs` 30-60; `crates/w2m_vm/src/guest.rs` 219-262, 760-800; `tests/cache_layout.rs`

## signal contract

### Emits

- None; a file format and a mapping, held by tests.

### Invariants

- Without `--cache`, the guest's layout and the run as at Sprint 260; the gates' batch run unchanged.

## artifact contract

### Files modified

- `crates/guest_runtime/src/layout.rs`, `crates/w2m_vm/src/` (`cache_file.rs`, `guest.rs`, `compile.rs`,
  `bin/w2m-vm.rs`), tests, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the fix-ups' count over UnityPlayer.dll's cache; the file's version-2 size against 256 MiB; a
  cached guest's bytes at `CACHE_FILE_VA` equal to the file's.
- **Planted faults:**
  - (1) A block's fix-up list missing one branch: the fixture test names the block.
  - (2) The map left out with a cache: the guest test names the region.

## done criteria

The cache file carries each block's fix-ups, and a guest run with a cache holds the file's bytes read-only at
`CACHE_FILE_VA`, each held by a test; without a cache nothing changes.

## self-review checklist

- [x] Scope names one verifiable property: the file's fix-ups and its map in the guest.
- [x] context_files cite the design, the file, the layout and its test, the guest's memory.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: the cache file in the guest.

## result

- **Version 2** (c6b91f41a): each block carries `fixup` and `nfixups` into the file's `fixups`, the word indices
  `relocate` re-encodes; `branch_leaving` is the one rule both use; `from_bytes` refuses another version by number.
  Over UnityPlayer.dll: 2,448,701 fix-ups for 998,222 blocks, the file 228,971,368 bytes of the region's 268,435,456,
  all blocks relocating equal (`store/translation-cache/cache-unityplayer-2026-10-30.json`, bde425323).
- **The region:** `CACHE_DATA_VA` removed; `CACHE_FILE_VA`, 256 MiB, ends at the assembly range `CACHE_CODE_VA`, which
  stays an address the words are assembled at and is never mapped; `tests/cache_layout.rs` holds both against every
  mapped area. `Layout::new(loaded, cache)` places the file's part, 2 MB aligned, after the heap and maps it in blocks;
  `Guest::new` sets `AP_READ_ONLY` on each of its 128 block descriptors (`tables::block_descriptor`, new);
  `Guest::place_cache_file` copies the bytes; `w2m-vm --cache` places the file before `use_cache`. The maker refuses a
  file larger than the region; its figures now name the file's region and the fix-ups in place of the data region's.
- **The tests:** the fixture test holds each block's fix-ups to the words `relocate` changes, over the 280 images;
  `a_cached_guest_holds_the_file_read_only_at_its_region` holds the bytes, the read-only blocks, the guest's memory
  within the 40-bit IPA, and no region without a cache; `qpc` runs in a VM with the file mapped.
- **The batch run with the map:** one untimed run with `--cache`, the reference's end, 128,456 installs and 125,515
  compiles, as at Sprint 257.
- **Files beyond the artifact contract:** `crates/w2m_vm/src/tables.rs` (`block_descriptor`) and
  `store/translation-cache/cache-unityplayer-2026-10-30.json` (the figures at version 2); `compile.rs`, named in the
  contract, did not change.
- **The review:** not come; this card rests on choices I and II alone.
- **The gates:** `scripts/gates.sh` exit 0 in 882 s on bde425323, copied from `store/gates/passes.tsv`; the timing stage 650 ns an exit at load 3.90.
- **Plants,** run on bde425323, each restored by `cp` and checked by `cmp`.
- **Plant 1,** `every_fixture_cache_covers_its_reads_indexes_its_pages_and_reads_back` failed, "block 0x1000's fix-ups
  [33], where the move changes [32, 33]": each block's first fix-up dropped (`.skip(1)` in `fixups`). A first try,
  dropping a fix-up at a block's last word, changed nothing: a block's last word is the budget stop's `b body`, inside
  it.
- **Plant 2,** `a_cached_guest_holds_the_file_read_only_at_its_region` failed, "called `Result::unwrap()` on an `Err`
  value: \"the guest has no cache file region: its start set no host range\"": the region left out of the layout with a
  cache.
