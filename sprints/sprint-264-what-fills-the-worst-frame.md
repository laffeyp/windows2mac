# Sprint 264 — What fills the worst frame

```yaml
---
id: 264
status: done
phase: 6
pass_kind: observation
---
```

## scope

Deferred (Sprint 263): with the installer inside the VM the compile time halves (678 to 694 ms against 1,285 to
1,296) and the most compiling in a 60 Hz frame does not fall (14.42 to 14.85 ms against 14.79 to 14.85), so step 5's
gate is not met. This card names the compiles that fill the worst frame, with and without the cache, by image and by
kind, before any change is made for them.

- **Where it stands, by lookup (2026-10-30):**
  - The probe (`monitor_code::compile_probe`) keeps sums only: the total, the longest call, the open frame's start and
    sum, and the most any closed frame held; a frame opens at a call's start and holds every call that ends within
    1/60 s of it.
  - Every call that returns code writes a compile log entry of 128 bytes (`block_compiler.c` 31-42; the installer
    likewise): its eight page slots end at byte 120, and bytes 120 to 127 are written by neither.
  - The host replays each entry at the next exit (`run.rs`, `vm_compiled`), the block's start and code in hand.
  - The cache covers UnityPlayer.dll; on Sprint 263's records 82,018 compiles lie outside any image and 41,830 in
    mono-2.0-bdwgc.dll.
  - Searched for `monitor_code.rs` (`compile_probe`), `run.rs` (`vm_compiled`), "frame" in Deferred, Surfaced and
    `docs/surfaced-archive-2026-10-30.md`: Deferred (Sprint 263), this item.
- **The readers:** the next card for step 5; the Decision "The translation cache's shape".
- **The work:**
  - A temporary probe: after a call that returned code, its ticks and the low 32 bits of its start counter written to
    the log entry's bytes 120 to 127; a temporary line in the replay printing each entry's start, kind (compile or
    install), ticks and start; each removed by `cp` and checked by `cmp`.
  - Two timed runs alone, judge and ledger off, without and with the cache; the frames rebuilt from the printed calls by
    the probe's own rule (`awk`), the rebuilt worst frame held to the record's figure.
  - The worst frame of each named: its calls, their ticks, their images (UnityPlayer.dll, mono-2.0-bdwgc.dll, outside
    any image, others), compiles against installs; and, for the cache's run, the ten worst frames likewise.
  - The findings on the record: in the card's result, the Decision, and Deferred (Sprint 263) with what the figures
    point to.
- **Not here:** any change for what the figures name.

## prerequisites

- 263 (closed)

## context_files

- `crates/w2m_vm/src/monitor_code.rs` (`compile_probe`, `call_path`); `src/run.rs` (`vm_compiled`)
- `tools/guest_runtime/block_compiler.c` 31-42; `docs/design-translation-cache-2026-10-30.md` (card 5's gate)

## signal contract

### Emits

- None of its own: two measured runs, their calls printed by temporary lines, the figures in the card's result.

### Invariants

- No run changed by a commit.

## artifact contract

### Files modified

- `sprints/sprint-264-what-fills-the-worst-frame.md` (the result); `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** each run's worst frame, rebuilt, against the record's "vm most in a 60 Hz frame"; its calls by
  image and kind; the ten worst frames' make-up with the cache.
- **Planted faults:**
  - (1) The rebuild given a frame span of 1/30 s: its worst frame no longer equals the record's figure, so the rebuild
    is the probe's rule and nothing looser.

## done criteria

The compiles that fill the worst 60 Hz frame are named by image and kind, with and without the cache, the rebuild
held to the record's figure.

## self-review checklist

- [x] Scope names one verifiable property: the worst frame's make-up.
- [x] context_files cite the probe, the replay and the log entry.
- [x] Emits: none of its own.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and one plant.
- [x] One concept: what fills the worst frame.

## result

- **The probe:** three temporary lines from e06d16415's tree, each file backed up by `cp` first and restored and checked
  by `cmp`: after a call that returned code, its ticks and its start counter's low 32 bits stored at bytes 120 and 124
  of the newest log entry (`monitor_code.rs`, after `compile_probe`); the replay printing each entry's start, ticks,
  start and kind (`run.rs`); the counter's frequency printed, 24,000,000 (`w2m-vm.rs`). A cache made under that tree.
- **The runs:** two timed runs alone, judge and ledger off, each to the reference's end: without the cache at load 1.75,
  276,560 calls printed, the record's most in a 60 Hz frame 11.87 ms; with it at load 2.28, 272,805 calls, 7.34 ms.
- **The rebuild:** `build/frame/frames.awk`, the probe's rule (a frame opens at a call's start and holds every call
  ending within 1/60 s of it; the first to end past it opens the next at its own start; the open frame folded at the
  end): 11.864 ms over 923 frames and 7.345 ms over 894, each the record's figure; the declined calls, which print no
  entry, change neither.
- **The worst frames:**

  | | without the cache | with the installer |
  |---|---|---|
  | the worst frame | 11.864 ms, 21.09 s in | 7.345 ms, 0.38 s in |
  | its calls | 1,633 compiles, 0 installs | 1,819 compiles, 0 installs |
  | their image | all in UnityPlayer.dll (0x181000000) | all in mono-2.0-bdwgc.dll (0x1180000000) |

  The cached run's ten worst frames, 5.00 to 7.35 ms: eight are mono-2.0-bdwgc.dll's in the run's first 1.04 s, and
  two, at 15.19 s and 16.82 s, code at 0x1222000000 outside any image, Mono's run-time code (Sprint 250 found its
  arena there).
- **What it shows:** with the installer, UnityPlayer.dll leaves the worst frame, and what remains is Mono's, which the
  cache does not cover; at this load the frame falls from 11.86 to 7.35 ms, below Sprint 248's 10.9 to 12.0. Sprint
  263's six records read 14.4 to 14.9 ms either way at load 3.3 to 4.2; these two runs are one each and not records, and
  what raised those figures is not measured, so step 5's gate is read again on six records alike at a load near 2
  (Deferred, Sprint 263, annotated).
- **The evidence kept** (f84997f57): the three rebuilds and their figures in `store/batch_run/worst-frames-2026-10-30/`.
  The first close, 51aac859d, was refused by `signal_report --check`, "no commit "Sprint 264: ..." to compare its gates
  run's trees with", the card having changed no file outside the bookkeeping; it was not pushed, undone by `git reset
  --soft`, and the gates run again on this commit, the first run's row kept in `store/gates/passes.tsv`.
- **The gates:** `scripts/gates.sh` exit 0 in 607 s on f84997f57, copied from `store/gates/passes.tsv`; the timing stage 669 ns an exit at load 3.66.
- **Plant 1,** the rebuild with a 1/30 s frame (`hz=30`): its worst frames, "worst 15.060 ms" without the cache and
  "worst 11.436 ms" with it, are not the record's 11.87 and 7.34; the rebuild at 1/60 s is the probe's rule.
