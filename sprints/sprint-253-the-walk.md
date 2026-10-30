# Sprint 253 — The walk

```yaml
---
id: 253
status: active
phase: 6
pass_kind: functional
---
```

## scope

Card 2 of `docs/design-translation-cache-2026-10-30.md` (choice A): UnityPlayer.dll's blocks found statically, in Rust,
as the frozen census found its instructions; the cover of the batch run's starts measured, which the design's first
revisit ground reads (under half of 132,110 starts reopens the choice).

- **Where it stands, by lookup (2026-10-30):**
  - The frozen census (`scripts/class_census.py` at `python-final`, 52-186; `src/w2m/x86ref.py` 29-147) counted
    4,996,593 instructions reachable in UnityPlayer.dll, 4,996,586 walked and 7 found by the sweep alone, 649 jump
    tables holding 14,885 entries (`store/census/static/instructions-hail-to-the-rainbow.json`), by:
    - a recursive walk from the code targets (entry, exports, TLS callbacks, `.pdata` starts) within the executable
      sections: direct branch and call targets followed; a RIP-relative `lea` of code not inside a `.pdata` function
      taken as a root; a stop at `ret`, `retf`, `jmp`, `int3`, `ud2`, `hlt`, `int 0x29`, and at a call through the
      import table to one of 14 imports that never return (`NORETURN`, x86ref 35-39);
    - MSVC switch tables found from `mov`/`movsxd r32, dword [base + index*4 + table]`, their entries read while each
      is an RVA inside the load's own `.pdata` function, walked as further roots until no new table appears (at most
      four rounds);
    - a linear sweep of each `.pdata` function that resumes after a stop only at an address the walk or a table
      reaches, and never decodes a table's words.
  - The census decoded with Capstone and named classes with w2m's decoder; the Rust tree has the decoder,
    `x86_decoder::decode` (length, iclass, ModRM, SIB, displacement, immediate), and `pe_image` (sections, the import
    table by slot, exports, TLS callbacks, `.pdata` starts and ends, `read_rva`), and no walker.
  - Searched for `pe_image`, `x86_decoder`, `census`, `walk` in Deferred, Surfaced and
    `docs/surfaced-archive-2026-10-30.md`: census row 11 (the DLLs and imports the batch run calls), whose imports'
    count this card does not take; Deferred (Sprint 251), cards 3 and 4.
- **The readers:** card 3, which translates the walk's blocks; a test on the depot's image; the census.
- **The work:**
  - A crate `image_walk` (library and binary): the walk, the tables and the sweep as the frozen census did them, with
    w2m's decoder; and the blocks: every root, every branch and table target, every fall-through after a conditional
    branch or a call.
  - Run on UnityPlayer.dll: its instruction count against 4,996,593, its tables against 649 and 14,885, each
    difference named by its cause; and the cover: of the batch run's 132,110 distinct starts in UnityPlayer.dll, how
    many are the walk's instructions and how many its block starts, from the exit ledger's blocks.
  - The figures kept in `store/translation-cache/walk-unityplayer-2026-10-30.json`, a test holding the stored file to
    the depot's image when the depot is present.
- **Not here:** the cache file; the walk of the other images.

## prerequisites

- 252 (closed)

## context_files

- `scripts/class_census.py` and `src/w2m/x86ref.py` at `python-final`; `store/census/static/instructions-hail-to-the-
  rainbow.json`
- `crates/x86_decoder/src/lib.rs`, `crates/pe_image/src/lib.rs`; `docs/design-translation-cache-2026-10-30.md` (A, card 2)

## signal contract

### Emits

- None; a static walk and its stored figures.

### Invariants

- No code of the run changed; the gates as at Sprint 252.

## artifact contract

### Files modified

- `crates/image_walk/` new, the workspace's member list, `store/translation-cache/walk-unityplayer-2026-10-30.json`,
  `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the instruction count, the tables and their entries, each against the census; the block
  starts; the cover of 132,110 starts, as instructions and as block starts.
- **Planted faults:**
  - (1) The jump tables left unwalked: the walk's count falls short of the census's, and the test names it.
  - (2) A call's fall-through not taken as a block start: the cover as block starts falls, and the test names it.
  - (3) The `NORETURN` stop removed: a unit test names the bytes after a call to `ExitProcess` walked.

## done criteria

The Rust walk counts UnityPlayer.dll's instructions as the census did, or names each difference, and its cover of the
batch run's starts is on the record.

## self-review checklist

- [x] Scope names one verifiable property: the image's blocks found statically, against the census and the run.
- [x] context_files cite the frozen walk, the census, the decoder, the image reader and the design.
- [x] Emits: the stored figures.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the walk.

## result
