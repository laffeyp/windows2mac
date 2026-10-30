# Sprint 251 — The translation cache's design

```yaml
---
id: 251
status: done
phase: 6
pass_kind: architecture
---
```

## scope

Plan step 5, "the PE images translated ahead of time … UnityPlayer.dll first" (`docs/rust-port-plan-2026-10-24.md`
158-160), as the Decision "Sequence for step 5" orders it after Sprint 250: the cache's design,
`docs/design-translation-cache-2026-10-30.md`, passing `design_check` and reviewed, before its code. The design is
held to the gate in counts the Decision states: the batch run conforming, "compiled in UnityPlayer.dll" falling from
149,308 by the blocks the cache holds, and "compiled" (274,837) by as many.

- **Where it stands, by lookup (2026-10-30; a research agent's read of the code, each fact it gave checked by
  path and line before use):**
  - Lookup: a two-way hashed table of (target, code) pairs, 1<<20 entries, 16 MiB, at `MONITOR_VA + DISPATCH_TABLE`
    (`guest_runtime::layout` 48-50); written by the C compiler (`dispatch_insert`, `block_compiler.c` 166-183) and the
    host (`run.rs` 373-388).
  - Code: one 64 MiB area at `MONITOR_VA + 0x60000` (`layout` 36-37), one cursor shared by both compilers; full, the
    C declines and the host stops the run (census row 15c); nothing is freed.
  - Position: exits are PC-relative `B`/`BL` to the dispatcher's `exit_va` and `slice_va`; runtime symbols and the
    state block are absolute (`vm_compiler.c` 323, 500-514; `stencil_table.h` 348-357; `lower.rs` 2178-2182); blocks
    are not chained.
  - The words of a block depend on more than its bytes: the successor scan reads only pages already in the code-page
    set, so flag liveness depends on what was compiled before (`block_compiler.c` 131-140, `compile.rs` 169-172).
  - Equivalence: `crates/translator/tests/c_words.rs` holds the Rust translator's words to the C's instruction by
    instruction over 1,186,511 calls; no judge compares whole blocks.
  - Placement: UnityPlayer.dll's preferred base is 0x180000000, size 0x19db000; mono-2.0-bdwgc.dll asks the same base
    and, loaded later, is placed elsewhere (`pe_loader` 6-10, 367-372; `virtual_memory` 492-507; the images' PE
    headers, read 2026-10-30). No base is on the record.
  - `pe_image` parses `.pdata`'s function starts (133-135, 536-546); the static census of 2026-10-15 counts 4,996,593
    reachable instructions and 649 jump tables in UnityPlayer.dll (`store/census/static/instructions-hail-to-the-
    rainbow.json`), by the frozen Python tools; the Rust tree has no walker.
  - The frozen host had a translation cache, keyed by the compiler's sources, the runtime image and the image, its
    entries relocated at load (`vm_backend.py` at `python-final` 419-428, 4662-4752); the Rust port left it out
    (`compile.rs` 3-5). The Decision of 2026-10-11 declined a persistent cache then, on a profile; the Decision of
    2026-10-14 set the cache's cover, the PE images, UnityPlayer.dll first.
  - Searched for `compile.rs`, `run.rs`, `monitor_code.rs`, `block_compiler.c`, `translator`, `pe_image`, `cache` in
    Deferred, Surfaced and `docs/surfaced-archive-2026-10-30.md`: census row 15c (a full code cache), row 13 (flag
    liveness on composed blocks, re-visit with step 5), row 2 (the loader's tests, re-visit with step 5), Deferred
    (Sprint 247) the counters on several vCPUs; each is named in the design where it bears.
- **The readers:** the reviewer; the cards that build the cache, which take their order and gates from the design.
- **The work:**
  - The facts the design needs and the tree can give, measured in Rust and stated with their source: UnityPlayer.dll's
    base on the batch run; its blocks the batch run compiles, by start, and their guest bytes; the code bytes a block
    takes in the code area.
  - The document: the cover (which blocks, found how), where the code lives and how it stays valid at a fixed place,
    the lookup, flag liveness, invalidation on a guest write, the key and where the cache is kept, the judges, the
    order of the building cards with a gate each; "Against the goal" and "Against the systems that solved it" as
    `design_check` requires.
  - A Decision marked "Architecture:" naming the document; reviewer-3's review of it, its findings disposed.
- **Not here:** any of the cache's code.

## prerequisites

- 250 (closed)

## context_files

- `docs/rust-port-plan-2026-10-24.md` 81-87, 158-160; `docs/design-host-compile-path-2026-10-24.md`;
  `docs/design-rust-host-2026-10-24.md`; `crates/design_check/src/main.rs`
- `crates/guest_runtime/src/layout.rs`, `crates/w2m_vm/src/monitor_code.rs`, `src/compile.rs`, `src/run.rs`,
  `tools/guest_runtime/block_compiler.c`, `crates/translator/tests/c_words.rs`, `crates/pe_image/src/lib.rs`
- `vm_backend.py` at `python-final` 419-428, 4662-4752; BLACKBOARD Decisions 2026-10-11 and 2026-10-14

## signal contract

### Emits

- None; a design.

### Invariants

- No code of the cache; the gates as at Sprint 250.

## artifact contract

### Files modified

- `docs/design-translation-cache-2026-10-30.md` new, `BLACKBOARD.md` (the Decision), any measuring code in Rust

### Command exit codes

- `cargo run -q -p design_check` returns 0 with the new document counted; `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the design's facts, each with its source; `design_check` counting four plans and designs.
- **Planted faults:**
  - (1) A system's source removed from the document: `design_check` names it.
  - (2) The Decision's pointer to the document removed: `design_check` names the Decision.

## done criteria

The design passes `design_check`, its Decision names it, and reviewer-3's review of it is disposed.

## self-review checklist

- [x] Scope names one verifiable property: the cache's design, checked and reviewed.
- [x] context_files cite the plan, the designs, the code it builds on and the frozen host's cache.
- [x] Emits: none, a design.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: how the cache is built.

## result

- **The facts:** one batch run from 9622bb579 with temporary lines at the stop, removed and checked by `cmp`:
  UnityPlayer.dll at 0x180000000, size 0x19db000; mono-2.0-bdwgc.dll at 0x1180010000, steam_api64.dll at
  0x1222ff0000; in UnityPlayer.dll 132,110 distinct starts for 149,308 compiles, 3,098,064 guest bytes, 14,340,616
  code bytes; the code area 29.3 MB full for 275,559 blocks.
- **The design:** `docs/design-translation-cache-2026-10-30.md` (43ea83c64), revised by reviewer-3's review at 251
  (dfcf29cd8): the region `[MONITOR_VA − 0x780_0000, MONITOR_VA)`, within a `B`'s reach of the dispatcher and outside
  every judged area; the cached pages read-only at load, a write faulting to `hvc 13`; four cards, each with a gate.
  `design_check` counts four plans and designs. The Decision "The translation cache's shape" names it.
- **A slip:** the card was first marked `pass_kind: design`, not among the kit's kinds (TECHNIQUES.md #14); 9622bb579
  made it `architecture`, the first `sed` having named the wrong line and changed nothing, which the empty commit
  showed.
- **The gates:** `scripts/gates.sh` exit 0 in 219 s on dfcf29cd8, copied from `store/gates/passes.tsv`; the timing stage 699 ns an exit at load 5.01.
- **Plants,** run on dfcf29cd8 after the design's last edit, as on 43ea83c64 before it, each restored by `cp` and checked
  by `cmp`.
- **Plant 1,** `design_check` failed, "docs/design-translation-cache-2026-10-30.md: "## Against the systems that solved
  it" names FEX with no source": FEX's link removed.
- **Plant 2,** `design_check` failed, "BLACKBOARD.md: the Decision of 2026-10-30 marked "Architecture:" names no
  checked plan or design": the Decision's pointer removed.
