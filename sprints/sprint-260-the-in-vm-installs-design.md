# Sprint 260 — The in-VM install's design

```yaml
---
id: 260
status: active
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
