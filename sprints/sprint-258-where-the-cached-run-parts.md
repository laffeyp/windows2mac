# Sprint 258 — Where the cached run parts from the uncached

```yaml
---
id: 258
status: active
phase: 6
pass_kind: observation
---
```

## scope

Deferred (Sprint 257), reviewer-3's Low 1 at 257, card 5's first item by name: the batch run with the translation
cache retires 80,307,872,496 instructions and without it 80,306,813,574, each deterministic, and its in-VM compiles in
mono-2.0-bdwgc.dll differ. This card finds the first exit where the two runs part and names, by measurement, what makes
them part there, before card 5 changes the same path.

- **Where it stands, by lookup (2026-10-30):**
  - The exit ledger (`exit_ledger.rs`, `--exit-ledger`) writes a row a served exit: its sequence, kind, x86 site, the
    instructions executed and the clock's warp at the exit, a request's words, and the sha256 of the process area.
  - With the cache, the miss stub sends UnityPlayer.dll's misses to the host, so the cached run has about 149,000 more
    `hvc 1` rows; the two ledgers line up only on the other kinds.
  - The C compiler and the cache cut blocks by different rules: the cache's blocks run to a branch or 64 instructions
    (`cache_file::lift_block`); the C's also end at a page it may not read (`cut by guarded`, `page needs a split`) and
    at a decode it refuses (`cut by decode`).
  - Searched for `exit_ledger.rs`, `run.rs`, "diverge", "path" in Deferred, Surfaced and
    `docs/surfaced-archive-2026-10-30.md`: Deferred (Sprint 257), this item; nothing else on two runs of one guest
    parting.
- **The readers:** card 5 (the install inside the VM), whose records are read against 257's; the design's card 5 gate.
- **The work:**
  - Two untimed batch runs from one tree, without and with the cache, each with `--exit-ledger`; their rows of every
    kind but `hvc 1` compared in order; the first row that differs in kind, site, instructions executed or the process
    area's hash named, with the thread it ran on.
  - From that row back, the cause measured, not inferred: what the two runs did differently before it, by whatever the
    ledger or a temporary line shows; each temporary line removed by `cp` and checked by `cmp`.
  - The finding on the record: in the card's result, and, when it is a defect, a test that holds the fix; when it is a
    conforming schedule a cached run may take, the design's card 5 gate says so.
- **Not here:** the install inside the VM (card 5).

## prerequisites

- 257 (closed)

## context_files

- `crates/w2m_vm/src/exit_ledger.rs`; `src/run.rs` 600-760; `src/compile.rs` (`install`, `place`); `src/cache_file.rs`
  (`lift_block`); `tools/guest_runtime/block_compiler.c` (`extent`)
- `docs/design-translation-cache-2026-10-30.md` (card 5); BLACKBOARD Deferred (Sprint 257)

## signal contract

### Emits

- None of its own: two exit ledgers, compared; the batch run's record unchanged.

### Invariants

- No run's behaviour changed unless the finding is a defect, and then only by its fix and its test.

## artifact contract

### Files modified

- `sprints/sprint-258-where-the-cached-run-parts.md` (the result); a fix and its test if the finding is a defect;
  `docs/design-translation-cache-2026-10-30.md` if the finding bears on card 5's gate; `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the first parting row's sequence in each ledger, its kind, site, thread and instructions
  executed; the cause, measured.
- **Planted faults:**
  - (1) The comparison given two ledgers that differ at one planted row: it names that row and no earlier one.

## done criteria

The first exit where the cached run parts from the uncached is named, and what makes it part is measured and on the
record, with a fix and its test if it is a defect.

## self-review checklist

- [x] Scope names one verifiable property: the first parting exit and its measured cause.
- [x] context_files cite the ledger, the run loop, the install and the two block-cutting rules.
- [x] Emits: none of its own.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and one plant.
- [x] One concept: where and why the two runs part.

## result
