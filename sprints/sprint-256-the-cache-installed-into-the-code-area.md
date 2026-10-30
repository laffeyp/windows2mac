# Sprint 256 — The cache installed into the code area

```yaml
---
id: 256
status: done
phase: 6
pass_kind: architecture
---
```

## scope

The Decision "The translation cache's shape" as annotated at Sprint 255: card 4 of
`docs/design-translation-cache-2026-10-30.md` cannot be built as designed. The C runtime maps a host address outside
`W2M_CODE_START..W2M_CODE_END` to x86 address 0 at an exception (`runtime.c` 245) and at an in-VM slice switch
(`threads.c` 258); the cache's 7,828,283 marks fit no table it reads; the host's give-backs read the 64 MiB code area
alone. This card revises the design (KIT_DIARY #108: every reader of a code address listed first) and measures what
the revision rests on.

- **Where it stands, by lookup (2026-10-30):**
  - The research read for card 4 (Sprint 255, its findings in the Decision's annotation) listed every structure keyed
    to run-time compiles: the reconstruction table (4,194,304 entries, appended in cursor order and searched by
    bisection, `run.rs` 191, `runtime.c` 162-171), the tails (`run.rs` 41-72), `HostCompiler::blocks` and
    `code_pages` (`compile.rs`), the dispatch slots (`run.rs` 373-389), the give-backs (`run.rs` 95-101, 476), the
    code-write path (`hvc 13`, `code_written`).
  - A host compile already does all of that bookkeeping for a block the host makes (`compile.rs` 70-140): it writes
    the words at the cursor, appends the marks to the reconstruction table, notes the tails, protects the pages and
    records the block; the C runtime and every judge then see it as code in the code area.
  - The costs, from `store/runs/w2m-vm-game-1793387174000000000`: an in-VM compile 4.93 µs (1,355 ms over 274,834);
    a simple host service 4.1 µs a call (`hvc 13`, 56.1 ms over 13,639); a host compile 43.8 µs (`hvc 1`, 47.8 ms over
    1,091).
  - The cached words are assembled at `CACHE_CODE_VA` plus an offset; which of them depend on that address is not
    recorded.
  - Searched for `cache_file.rs`, `compile.rs`, `run.rs`, `monitor_code.rs`, the design in Deferred, Surfaced and
    `docs/surfaced-archive-2026-10-30.md`: Deferred (Sprint 251), card 4's half, which this revision rewrites;
    census row 15c (a full code area), which installs into the code area bear on.
- **The readers:** the cards that build the cache's use, which take their order and gates from the revised design.
- **The work:**
  - The relocation measured: every cached block of UnityPlayer.dll assembled at two bases, the words that differ
    counted by kind, and re-encoding the first assembly's differing words for the second base checked to give the
    second assembly exactly. Its figures on the record; a test holding the re-encoding on the fixtures.
  - The design revised: B, B', C and E give way to installing a cached block into the code area on its first use, with
    its marks, tails, pages and record as a host compile makes them; installs counted apart from compiles; the order
    two cards, the install by the host first (every judge sees cached words in the real run; the plan's gate in
    counts), then the install inside the VM (the time, from 4.1 µs a host exit to a copy). The regions' constants and
    their test stay until a card removes what nothing uses.
  - The Decision annotated "Architecture:" with the revision.
- **Not here:** any install.

## prerequisites

- 255 (closed)

## context_files

- `docs/design-translation-cache-2026-10-30.md`; the Decision "The translation cache's shape" and its annotations
- `crates/w2m_vm/src/cache_file.rs`, `src/compile.rs` 70-140, `src/run.rs` 41-101, 191, 373-389, 476
- `tools/guest_runtime/runtime.c` 162-171, 245; `tools/guest_runtime/threads.c` 258

## signal contract

### Emits

- None; a design and its measured relocation.

### Invariants

- No run changed; the gates as at Sprint 255.

## artifact contract

### Files modified

- `crates/w2m_vm/src/cache_file.rs` (the relocation), a test, `docs/design-translation-cache-2026-10-30.md`,
  `store/translation-cache/` (the figures), `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0; `design_check` counts the revised design

## observation contract

- **Expected values:** the relocated words by kind; the blocks whose re-encoding equals the second assembly, against
  the blocks made.
- **Planted faults:**
  - (1) A relocated branch's offset computed from the old base: the test names the block whose words differ.
  - (2) A kind of address-dependent word left out of the relocation: the test names it.

## done criteria

The design states how a cached block becomes a block the C runtime and the host already know, and the relocation it
needs is measured and held by a test.

## self-review checklist

- [x] Scope names one verifiable property: the design revised on measured relocation.
- [x] context_files cite the design, the research's sites, the host compile and the runtime's limits.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: how a cached block enters the run.

## result

- **The relocation:** `cache_file::relocate` and `relocation_census` (ab0f395f8): a `B` or `BL` whose target lies outside
  the block re-encoded, a branch inside kept, any other address-dependent word refused by kind (`Unmovable`). Measured
  on UnityPlayer.dll's cache, remade under its new key `24c34b0c…` in 16.8 s: all 998,222 blocks, moved from the cache
  region to `MONITOR_VA + CODE_CACHE + 0x10_0000`, equal their assembly there, none refused, none differing; every block
  of the 280 fixtures too (`tests/cache_file.rs`). The rule held by clang's words (1f388ea8e).
- **The design:** revised, choices G and H, B, B', C and E superseded and kept; the order from card 4: the install by
  the host, then the install inside the VM, each with its gate; `design_check` counts it. The Decision annotated.
- **A test corrected:** the unit test's first draft held `b .+4` at a two-word block's last word as inside; it targets
  the block's end, outside, and `relocate` re-encoded it rightly.
- **The gates:** `scripts/gates.sh` exit 0 in 675 s on 1f388ea8e, copied from `store/gates/passes.tsv`; the timing stage 653 ns an exit at load 3.80.
- **Plants,** run on 1f388ea8e, each restored by `cp` and checked by `cmp`.
- **Plant 1,** `a_branch_out_of_the_block_is_re_encoded_and_other_kinds_refused` failed, "assertion `left == right`
  failed … left: Ok([335544384, 402653183])": the offset re-encoded from the old base, the word left 14000040.
- **Plant 2,** the same test failed, "called `Result::unwrap_err()` on an `Ok` value: [268435520]": the `ADR` kind left
  out of the refusals.
