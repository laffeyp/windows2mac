# Sprint 259 — Where the VM's time goes with the cache

```yaml
---
id: 259
status: done
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

- **The variants:** three temporary lines (`W2M_FLUSH` in the dispatcher after an in-VM compile, `W2M_ONLY` turning
  installs off, a print of the code cursor), each file backed up by `cp` first; a cache made under the edited tree.
  Twelve timed runs from fd840f005's tree with the lines, judge and ledger off, alternating, run alone, each ending at
  the reference's stop. The machine's load at each start, 2.48 to 3.79: WindowServer near 48% of a CPU and "State
  Transfer.app" near 15%, outside this project.
- **A first batch, void:** the script handed `--cache` to `env` as a variable the command's text never saw, so the
  runs named (a) and (b) ran without it, 1,091 `hvc 1` exits and the uncached cursor each; stopped after six runs and
  made again with the flag as an argument. Its (d) runs stand with the second batch's.

  | | "vm" run seconds | the VM's compile time | host `hvc 1` | code placed |
  |---|---|---|---|---|
  | (a) the cache installed | 23.06, 23.67, 23.33 | 606 to 613 ms | 0.22 s, 150,065 exits | 28,172,400 bytes |
  | (b) the host compiles, nothing installed | 23.23, 23.63, 23.56 | 600 to 619 ms | 1.49 to 1.52 s | 28,203,708 |
  | (c) no cache | 23.90, 24.30, 24.34 | 1,278 to 1,329 ms | 0.04 s, 1,091 | 29,867,536 |
  | (d) no cache, the global invalidation after each in-VM compile | 24.24, 24.53, 24.34 | 1,475 to 1,496 ms | 0.04 s | 29,867,536 |

- **What they show:** (a) against (b), −0.12 s on the means, inside the spread: the cached words and cuts cost no VM
  time against the host's own compiles of the same starts (reviewer-3's Low 1 at 259: nothing to divide). (d) against
  (c), +0.19 s over 274,837 invalidations, about 0.7 µs of VM time a use, so about 0.1 s over the cached run's
  150,065. (b) against (c), −0.70 s, about the VM's compile time the host takes over. So without the ledger and the
  judge the VM's run seconds fall with the cache, about 0.8 s on the means.
- **A departure, the ledger:** Sprint 257's like records, whose harness command writes the exit ledger, showed the VM's
  run seconds flat. Six runs with `--exit-ledger`, alternating: with the cache 24.92 (load 2.73), 30.78 (4.11), 25.96
  (3.73); without 25.49 (3.20), 24.62 (5.14), 25.52 (4.82). The ledger adds 1 to 2 s each way, and under this load the
  pairs disagree (−0.57, +6.16, +0.44): not settled. Deferred (Sprint 259) holds it for card 5's records.
- **The design's card 5:** amended in a work commit (796590103): the figures, the install invalidating by range and
  page as the in-VM compiler does, a miss-stub change an option off by default (reviewer-3's Low 2 at 259).
- **The temporary lines:** the three files restored by `cp` and checked by `cmp`, `monitor_code.rs` first for the plant;
  the tree clean before any commit.
- **The gates:** `scripts/gates.sh` exit 0 in 284 s on 796590103, copied from `store/gates/passes.tsv`; the timing stage 696 ns an exit at load 9.33.
- **Plant 1,** run with `monitor_code.rs` restored (the added invalidation left out) and `W2M_FLUSH=1` still set, load
  3.13: the compile time 1,325 ms, inside (c)'s 1,278 to 1,329 and under (d)'s 1,475 to 1,496, so the (d) figure names
  the invalidation and nothing else; the run seconds, 24.36, lie inside both (c)'s and (d)'s spread under this load,
  and the compile time is the figure that names it.
