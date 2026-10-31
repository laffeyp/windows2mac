# Sprint 259 — Where the VM's time goes with the cache

```yaml
---
id: 259
status: active
phase: 6
pass_kind: observation
---
```

## scope

Deferred (Sprint 258), reviewer-3's Medium 1 at 258: on Sprint 257's like records the VM's compile time falls by about
700 ms with the cache (1,363 to 1,385 ms to 667 to 674) while the VM's run seconds do not (24.18 to 24.33 against
24.28 to 24.36). Card 5 installs inside the VM and can recover only what the host's path costs. This card measures,
by run variants that differ in one thing each, how much of the VM's time goes to the cached code's words, to the
`hvc 1` exits, and to the global cache and TLB invalidation each exit's return makes.

- **Where it stands, by lookup (2026-10-30):**
  - After every `hvc 1` the miss stub runs `IC IALLUIS; TLBI VMALLE1IS; DSB ISH; ISB` before it re-dispatches
    (`monitor_code.rs` 394-400). The cached run takes it 150,065 times, the uncached 1,091.
  - The in-VM compiler makes its code visible by range, `dc cvau` and `ic ivau` over the block's words
    (`block_compiler.c` 166-176), and invalidates by page, `tlbi vaae1is`, for a page it protects (113-118).
  - With the host range and no block installed, the host compiles each miss in UnityPlayer.dll at the in-VM
    compiler's cuts (Sprint 258's bisection: the 40 blocks it compared cut alike) through the same exits.
  - Searched for `monitor_code.rs`, `block_compiler.c`, "TLBI", "flush" in Deferred, Surfaced and
    `docs/surfaced-archive-2026-10-30.md`: Deferred (Sprint 258), this item; nothing on the miss path's invalidation.
- **The readers:** card 5 (the install inside the VM), its design and gate; the Decision "The translation cache's
  shape".
- **The work:**
  - Four variants from one tree, three timed runs each, judge off, run alone, alternating: (a) the cache installed;
    (b) the same host range with nothing installed, the host compiling; (c) no cache; (d) no cache, with the miss
    stub's global invalidation added after each in-VM compile. Each run's "vm" run seconds, the VM's compile time,
    the host's `hvc 1` seconds, the exits, and the code cursor at the end (the code bytes the run placed).
  - From them: (a) against (b), the cached words' cost; (d) against (c), the global invalidation's cost per use with
    no exit; (b) against (c), the exits' cost with the invalidation. Each temporary line removed by `cp` and checked
    by `cmp`.
  - The figures on the record, in the card's result and the Decision; the design's card 5 says what it must do about
    the invalidation if the figures name it.
- **Not here:** any change to a run (card 5 or a card the figures call for).

## prerequisites

- 258 (closed)

## context_files

- `crates/w2m_vm/src/monitor_code.rs` 320-410; `tools/guest_runtime/block_compiler.c` 110-180
- `crates/w2m_vm/src/run.rs` (`hvc 1`, `run_ns`); `docs/design-translation-cache-2026-10-30.md` (card 5)

## signal contract

### Emits

- None of its own: twelve measured runs, their figures in the card's result.

### Invariants

- No run's behaviour changed by a commit; the variants exist only as temporary lines.

## artifact contract

### Files modified

- `sprints/sprint-259-where-the-vm-time-goes-with-the-cache.md` (the result); `docs/design-translation-cache-2026-10-30.md`
  if the figures bear on card 5; `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** each variant's "vm" seconds, compile time, `hvc 1` seconds and exits, and code bytes, three runs
  each; the three differences named above with their per-use cost.
- **Planted faults:**
  - (1) Variant (d)'s added invalidation left out: its run seconds and (c)'s agree within their spread, and the
    result's (d) figure names no cost (the plant read as the check that (d) measures what it says).

## done criteria

The VM's run seconds with the cache are accounted to the cached words, the exits and the invalidation, each by a
measured difference, and card 5's design says what follows.

## self-review checklist

- [x] Scope names one verifiable property: the VM's time with the cache, accounted by measured differences.
- [x] context_files cite the miss stub, the C's invalidation, the run loop and card 5.
- [x] Emits: none of its own.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and one plant.
- [x] One concept: where the VM's time goes.

## result
