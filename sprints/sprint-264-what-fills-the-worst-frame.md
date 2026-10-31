# Sprint 264 — What fills the worst frame

```yaml
---
id: 264
status: active
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
