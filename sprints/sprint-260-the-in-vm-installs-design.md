# Sprint 260 — The in-VM install's design

```yaml
---
id: 260
status: done
phase: 6
pass_kind: architecture
---
```

## scope

Card 5 of `docs/design-translation-cache-2026-10-30.md`, the install inside the VM, cannot be written from the design
as it stands: it names what the install does (the copy and the re-encoding in the miss stub's path, no exit; by range
and by page, Sprint 259) and not where the cached words live in guest memory, what code copies them, how the host
learns of an install, or how the run judges one. This card writes those choices into the design, `design_check`
passing and a review before code, as Sprints 251 and 256 did for cards 1 to 4.

- **Where it stands, by lookup (2026-10-30):**
  - `layout.rs` 41-45 keeps `CACHE_CODE_VA` (120 MiB below `MONITOR_VA`) and `CACHE_DATA_VA` (64 MiB below that),
    held by `tests/cache_layout.rs`; no run maps them since the revision at 256.
  - The host learns of an in-VM compile from the compiler's log, replayed after each exit (`vm_compiled`, `run.rs`
    416-446): the block's start, end and code, its marks' first index and count, its pages; the replay notes the
    tails, the pages and the start.
  - A change to the C runtime moves the image's linked addresses and with them the frozen-image and call-stream tests
    (Sprint 247, Observation 1); monitor code is written by Rust (`monitor_code.rs`) and judged by its own word tests.
  - UnityPlayer.dll's cache: 998,222 blocks, 112,565,508 bytes of words, 7,828,283 marks, its page index
    (Sprint 254); the batch run installs 128,456 of them.
  - Card 4's install (`HostCompiler::install`): `cache_file::relocate` re-encodes a `B` or `BL` whose target leaves the
    block and keeps every other word (Sprint 256).
  - Searched for `layout.rs`, `run.rs` (`vm_compiled`), `monitor_code.rs`, "install" in Deferred, Surfaced and
    `docs/surfaced-archive-2026-10-30.md`: Deferred (Sprint 259), the ledger's share, which card 5's records take;
    Deferred (Sprint 247), the counters on several vCPUs.
- **The readers:** card 5's builder; the design's gates; the Decision "The translation cache's shape".
- **The work:**
  - The design's card 5 extended with its choices, each with its ground and the alternative weighed: where the words,
    the marks and the start index live in guest memory, and their size against the guest's memory; the code that looks
    a start up, copies and re-encodes, appends the marks, protects the pages and invalidates by range (monitor code or
    the C runtime); the log entry that tells the host of an install, apart from a compile; what a write to the image
    does inside the VM; the judges (card 4's counts and the cached count exact, Sprint 258).
  - `design_check` passing; the Decision annotated "Architecture:"; a review asked before card 5.
- **Not here:** any code.

## prerequisites

- 259 (closed); its close's CI failed (run 41158222063), named in its entry, its fix in this card's commit.

## context_files

- `docs/design-translation-cache-2026-10-30.md` (G, H, card 5); `crates/guest_runtime/src/layout.rs` 30-60
- `crates/w2m_vm/src/run.rs` 416-470; `src/compile.rs` (`install`, `place`); `src/monitor_code.rs` 320-410;
  `tools/guest_runtime/block_compiler.c` 110-180, 220-310

## signal contract

### Emits

- None; a design.

### Invariants

- No run changed.

## artifact contract

### Files modified

- `docs/design-translation-cache-2026-10-30.md` (card 5), `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0; `design_check` counts the extended design

## observation contract

- **Expected values:** each choice with its ground, its alternative and the figure it rests on.
- **Planted faults:**
  - (1) A card-5 choice that names no ground: `design_check` refuses the design.

## done criteria

Card 5's design names where the cached words live in the guest, what installs them, how the host and the judges learn
of an install, and what a write does, each with its ground, and `design_check` counts it.

## self-review checklist

- [x] Scope names one verifiable property: card 5's design complete enough to build.
- [x] context_files cite the design, the layout, the log's replay, the monitor code and the C compiler.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and one plant.
- [x] One concept: the in-VM install's design.

## result

- **The design:** card 5's build, choices I to VII, in `docs/design-translation-cache-2026-10-30.md` (3056a992c): the
  file mapped read-only in the guest (211,190,784 bytes; copying only the run's blocks weighed and refused); each
  block's fix-ups in the file (decoding in the VM refused, one rule in one home); the installer in monitor code (the C
  runtime refused, its image's addresses held by the frozen tests); no page protected in the VM (card 4 guards them at
  load); the log's `pad` word marking an install; a write stopping the installer; the judges, with three plants.
- **A scope item first missed:** choice I gave the file's size and not the guest's memory, which the card's work names;
  found while the gates ran on 3056a992c, which were stopped (no row written), and amended (d60e3250e): the guest maps
  a little over 513 GiB of the 40-bit IPA's 1 TiB, the arena 512 GiB of it, and the file is one more map, with
  `tests/cache_layout.rs` rewritten for the one region.
- **A hazard found in the writing:** the C compiler never writes the log entry's `pad`, and the slots are reused, so
  the replay clears `pad` after each entry; written into choice V.
- **Facts checked before they were written:** the blocks in start order (`image_walk`'s `block_starts`, a
  `BTreeSet`); 20 probes for 998,222 blocks; about 14.5 MB for the run's 128,456 blocks at 112.8 bytes a block.
- **A departure in the plant:** the card planted a choice with no ground for `design_check` to refuse; `design_check`
  checks the goal's figures and each named system's source, not a choice's ground (its module head). The plant was
  replaced by one it judges: QEMU's source taken from the systems section.
- **The review:** the card asked for a review before card 5; review requests here carry the user's words alone, so
  card 5 begins after the next one.
- **The decision annotated** "Architecture:", naming the design.
- **Sprint 259's close fixed in this card's commit** (6ec632e34): its plant's run line quoted, its close's failed CI run
  41158222063 named, WORKING_AGREEMENT's check-before-push rule made to gate the push in the command.
- **The gates:** `scripts/gates.sh` exit 0 in 14 s on d60e3250e, copied from `store/gates/passes.tsv`, "rust=none": no crate changed, so no Rust or timing stage ran.
- **Plant 1,** run on 3056a992c, restored by `cp` and checked by `cmp`: QEMU's URL taken from the systems section,
  `design_check` exited 1 with "docs/design-translation-cache-2026-10-30.md: \"## Against the systems that solved it\"
  names QEMU with no source".
