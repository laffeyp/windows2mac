# Sprint 252 — The whole-block judge

```yaml
---
id: 252
status: active
phase: 6
pass_kind: functional
---
```

## scope

Card 1 of `docs/design-translation-cache-2026-10-30.md` (choice F): the cache's words will be the Rust translator's,
and no judge compares whole blocks of the two compilers; `crates/translator/tests/c_words.rs` holds them equal
instruction by instruction over 1,186,511 calls. This card judges every block the batch run's in-VM compiler makes,
compiled again by the Rust translator from the same bytes, at the same code address and with the same flags live.

- **Where it stands, by lookup (2026-10-30):**
  - Each in-VM compile leaves a log entry (`block_compiler.c` 31-42, 295-311): `start`, `end`, `code`, its marks'
    first index and count in the reconstruction table, its instruction `count`, `cut` (the status it stopped before),
    and `live_used`, "the flags live at its end as compiled"; the cursor moves by its words (`v[VMC_CURSOR] = cursor +
    4 * words`), so a block's words run from its `code` to the next entry's, the last to the cursor.
  - `run.rs`' `vm_compiled` drains the log at each exit and empties it (395-447); it reads `count` and `npages` and not
    `live_used`.
  - The host compiles with `translator::lower::assemble_block(insns, base, exit_va, slice_va, state, runtime, live)`
    (`compile.rs` 95-111), its instructions lifted from guest memory by `lift_at`.
  - Searched for `run.rs`, `compile.rs`, `bin/w2m-vm.rs`, `block_compiler.c` in Deferred, Surfaced and
    `docs/surfaced-archive-2026-10-30.md`: Deferred (Sprint 251), the cache's details, cards 3 and 4, not this one;
    census row 15b names `run.rs` and `compile.rs` for string errors, not changed; census row 13, flag liveness on
    composed blocks, re-visited: this judge holds the C's liveness as compiled, not the liveness itself.
- **The readers:** the run loop, which calls the judge before the log is emptied when asked; `w2m-vm`; the gates.
- **The work:**
  - `w2m-vm --judge-blocks <file>`: before each drain, every logged block lifted from `start` for `count`
    instructions, assembled at its `code` with `live_used`, its words and marks compared with the C's; one line a
    block that differs (start, end, the first differing word's index, the C's word, the Rust's, the instruction
    there), and a last line of totals: blocks judged, equal, differing.
  - Run on the batch run, alone; the file kept as `store/translation-cache/whole-blocks-2026-10-30.tsv`, each
    differing block's class named in its line by the card's work.
  - A test holds the stored file: its totals add up, blocks judged equal the record's "compiled", and every
    differing line names a class from a closed list the test holds, with its reason.
- **Not here:** the walk, the cache.

## prerequisites

- 251 (closed)

## context_files

- `tools/guest_runtime/block_compiler.c` 31-42, 221-317; `crates/w2m_vm/src/run.rs` 395-447, 650-665
- `crates/w2m_vm/src/compile.rs` 60-140; `crates/translator/src/lower.rs` (`assemble_block`); `src/bin/w2m-vm.rs`
- `docs/design-translation-cache-2026-10-30.md` (F, card 1)

## signal contract

### Emits

- None of its own; the gates' batch run, which must still conform, and the judge's file from one run.

### Invariants

- Without `--judge-blocks`, the run, its record and its stop file unchanged.

## artifact contract

### Files modified

- `crates/w2m_vm/src/run.rs` (the judge's call), a new module for the judge, `src/bin/w2m-vm.rs`, a test,
  `store/translation-cache/whole-blocks-2026-10-30.tsv`, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** blocks judged equal to the run's compiled count, 274,837; the equal and differing counts; each
  differing class with its count.
- **Planted faults:**
  - (1) The Rust words assembled with every flag live: the judge names blocks the C compiled with fewer.
  - (2) A differing line with no class: the test names it.
  - (3) The totals off by one block: the test names them.

## done criteria

Every block the batch run's in-VM compiler made is judged against the Rust translator's, and each difference has a
named class, or there is none.

## self-review checklist

- [x] Scope names one verifiable property: whole blocks of the two compilers compared.
- [x] context_files cite the log, its drain, the host's assembly and the design.
- [x] Emits: the judge's file from one run.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the whole-block judge.

## result
