# Sprint 254 — The cache file

```yaml
---
id: 254
status: done
phase: 6
pass_kind: functional
---
```

## scope

Card 3 of `docs/design-translation-cache-2026-10-30.md` (choices B, B', D, F): the walk's blocks of UnityPlayer.dll
translated by the Rust translator into a file under its key, at the code region's addresses, each block's flag
liveness from the walk; nothing loads it yet (card 4).

- **Where it stands, by lookup (2026-10-30):**
  - The walk (`crates/image_walk`, Sprint 253): 1,004,561 block starts in UnityPlayer.dll.
  - The host's compile (`compile.rs` 70-140): a block lifted from its start until a branch, a thunk or 64
    instructions (`MAX_BLOCK_INSNS`), a lift error ending it before the failing instruction; its flags live from its
    successors by `flags_live_out`, scanning at most `LIVENESS_SCAN` instructions of each; `assemble_block(insns, base,
    exit_va, slice_va, state_va, runtime, live)` (`translator::lower` 3421) gives its words and marks.
  - `exit_va` is the dispatcher's, `slice_va` the slice stub's, `runtime` the runtime image's symbols, all fixed by a
    guest's setup (`guest.rs` 920-930), so the file is made by `w2m-vm` with a game's setup, not by a tool apart.
  - The regions: code `[MONITOR_VA − 0x780_0000, MONITOR_VA)` (B), data `[MONITOR_VA − 0xB80_0000, MONITOR_VA −
    0x780_0000)` (B', reviewer-3's Medium 1 at 253, in this card's commit).
  - Deferred (Sprint 251), card 3's half: the page-to-starts index keyed by every page a block touches; the layout test
    asserting the regions within the IPA size at the 4 KB granule and every branch target within 128 MiB of the code
    region's lowest byte. Deferred (Sprint 253): a fixture image walked on every target for CI; the 755 run starts
    inside walk blocks counted by cause.
  - Searched for `compile.rs`, `image_walk`, `layout.rs`, `w2m-vm.rs`, `translator` in Deferred, Surfaced and
    `docs/surfaced-archive-2026-10-30.md`: the two lines above; census row 13 (flag liveness on composed blocks): this
    card's liveness check is its evidence for the cache's blocks; census row 15c (a full code area): the cache's code
    lies outside it.
- **The readers:** card 4's loader and probe; the tests named below.
- **The work:**
  - `w2m-vm --make-cache <file>` with a game's setup: each walk block start lifted as the host compiles, its live
    flags from its walked successors over the whole image, assembled at the next address in the code region; the file
    holds its key (the image's sha256, its base, the translator's tree id, the monitor code's words' sha256, the
    runtime image's sha256), each block's start, end, code offset and marks, the code, and the page-to-starts index
    over every page a block's bytes touch.
  - Measured on the record: blocks made, starts refused and why, code bytes against 120 MiB, data bytes against 64
    MiB, the time to make it; the 755 inside-block run starts counted by cause.
  - Tests: the layout test of B and B' (`W2M_*` and `MONITOR_SIZE` unchanged, the regions overlapping nothing the
    loader or the arena maps, within the IPA size, every exit target within 128 MiB); every cached block's live flags
    covering every flag its walked successors read; a fixture image's walk held on every target.
- **Not here:** loading the file, the probe, the write bitmap (card 4).

## prerequisites

- 253 (closed)

## context_files

- `docs/design-translation-cache-2026-10-30.md` (B, B', D, F, card 3); `crates/image_walk/src/lib.rs`
- `crates/w2m_vm/src/compile.rs` 70-200, `src/guest.rs` 900-935, `crates/guest_runtime/src/layout.rs`,
  `crates/translator/src/lower.rs` 3421-3440; BLACKBOARD Deferred (Sprint 251) and (Sprint 253)

## signal contract

### Emits

- None; the cache file and its measured figures.

### Invariants

- No run loads the file; the gates' batch run, its judge and its stop as at Sprint 253.

## artifact contract

### Files modified

- `crates/w2m_vm/` (a module for the cache file, `bin/w2m-vm.rs`, tests), `crates/guest_runtime/src/layout.rs` (the
  regions' constants), a fixture walk test, `store/translation-cache/` (the figures), `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** blocks made against 1,004,561; code and data bytes against their regions; the time; the 755 by
  cause.
- **Planted faults:**
  - (1) A block's live flags dropping one its successor reads: the liveness check names the block.
  - (2) The code region moved 8 MiB lower: the layout test names an exit target past 128 MiB.
  - (3) The page index keyed by a block's start page alone: a test names a block over two pages missing from the second.

## done criteria

The cache file for UnityPlayer.dll is made under its key, its blocks' liveness checked against their successors, its
sizes within its regions on the record, and the regions held by the layout test.

## self-review checklist

- [x] Scope names one verifiable property: the cache file, made and checked.
- [x] context_files cite the design, the walk, the host compile, the layout and the translator.
- [x] Emits: the file and its figures.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the cache file.

## result

- **The pieces:** the regions' constants and `tests/cache_layout.rs` (e12ce43a7); the fixture walk on every target
  (201e2f726); the liveness scan moved to `translator::liveness`, one function for the host and the cache (ec6d594ae);
  `MAX_BLOCK_INSNS` moved to the translator; `cache_file.rs` and `w2m-vm --make-cache` (6bd1bc47f); the fixture test of
  the file (727b51262).
- **The file:** UnityPlayer.dll at 0x180000000, key `10625aa1…`, made in 16.7 s: 998,222 blocks of the walk's
  1,004,561, the other 6,339 starts' first instruction not lifting; code 112,565,508 bytes of the code region's
  125,829,120 (89.5%), data 37,611,068 of 67,108,864; 0 blocks whose live flags miss a successor's read; 211,190,784
  bytes under `build/translation-cache/`, not committed; figures in `store/translation-cache/cache-unityplayer-2026-10-30.json`.
- **The 755:** 351 lie inside a cached block, 101 after a block at the 64-instruction cap and 250 at no cached block's
  end; 404 inside none. The cache adds no starts beyond the walk's.
- **Departures:** the key hashes the addresses the words embed (the dispatcher, the slice stub, the state block, the
  runtime's calls) with the image, its base and the translator's tree, not the monitor code's words whole, which are
  more than the words depend on. The regions' bound within the 40-bit IPA moves to card 4, where the guest's memory is
  laid out. The liveness check reads each successor's first instruction, independent of the scan that set the flags.
- **Two faults on the way:** the first fixture run of the file's test found the refusals map inside `CacheFile` and not
  in the file, so a cache with refusals read back unequal; `make` returns them beside the file. And the maker's first
  commit message named a "9 commits' tree" that was not so; amended before any push.
- **The gates:** `scripts/gates.sh` exit 0 in 759 s on 727b51262, copied from `store/gates/passes.tsv`; the timing stage 655 ns an exit at load 3.29.
- **Plants,** run on 727b51262, each restored by `cp` and checked by `cmp`.
- **Plant 1,** `every_fixture_cache_covers_its_reads_indexes_its_pages_and_reads_back` failed, "live flags miss a
  successor's read at [1cff, 398d, …]": a block's stored live flags without ZF.
- **Plant 2,** `every_exit_is_within_a_branchs_reach_of_the_cache_code` failed, "the dispatcher at 0x7f0000001800": the
  code region moved 8 MiB lower and larger.
- **Plant 3,** the fixture test failed, "block 0x2fc8 not in page 0x3000's list": the page index keyed by a block's
  first page alone.
