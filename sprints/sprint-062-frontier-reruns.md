# Sprint 062 — the frontier's reruns: where 131 seconds go, a snapshot at the stop, translations kept

```yaml
---
id: 062
status: closed
phase: 5
pass_kind: functional
---
```

## scope

Every frontier rerun replays 57 million instructions and takes about 131 s (Sprint 060's records). The review
(W3) and the plan's row 062 ask for a snapshot at the stop and a persistent translation cache. A profile of one
scratch run (cProfile, 2026-10-11) shows where the time goes. Of 154 s under the profiler:

| Cost | Where | Seconds |
|---|---|---|
| `list.sort`, 62,014 calls | `_compile`, `pc_map.sort()` | 98 |
| model steps | `_slow_sse` | 21 |
| the monitor's text pipe | `readline` | 15 |
| lifting | `lift` | 7 |

1. **The quadratic sort goes.** The code cache only grows, and each block's marks come in rising host addresses,
   so appending keeps `pc_map` sorted, as `_recon_append` already assumes for the same pairs. The frontier's time
   is measured before and after on the same commit's records.
2. **A snapshot at the stop.** The frontier saves its state where it stops: the guest memory file (an APFS clone),
   the threads, the host's tables, the shared objects and the second process. A run resumes from a snapshot and
   reaches the same stop with the same call stream as a full run.
3. **Translations kept across runs,** keyed by the bytes they were lifted from, if the measured time after items 1
   and 2 still has translation as a large share. If it does not, the card records that and the row's item closes on
   the measurement.

## prerequisites

- 061 (closed: CI run 38208477502 on aee379d6; main 84b153eb)

## context_files

- `src/w2m/vm_backend.py` (`_compile`, `pc_map`, `flush_translations`, the memory file), `src/w2m/kernel.py`,
  `src/w2m/scheduler.py`, `scripts/run_frontier.py`; the profile in the card's observed section

## signal contract

### Emits

- The frontier's records, before and after each change, with `elapsed_seconds`; a resumed run's EXECUTION_STARTED
  names the snapshot it resumed from in `unobserved`'s place only if the vocabulary has no field. The vocabulary is
  checked before the build.

### Invariants

- Vocabulary 0.7; no new tags without a proposal.

## artifact contract

- modified: `src/w2m/vm_backend.py`, `scripts/run_frontier.py`, and the files a snapshot needs; tests that fail
  before each change
- gates 0 and CI `success` on the closing commit

## observation contract

- **The frontier's run takes less time,** measured on its own records, with the same stop, call stream and
  comparison verdict (`conforms`).
- **A resumed run equals a full run** at the stop: the same stop, the same instruction count, the same call
  stream digest.

## done criteria

The sort gone and measured; a snapshot taken and resumed with equal results; the translation cache built or
declined on the measurement.

## observed

- **The sort is gone, and the frontier runs in 33.1 s, down from 138.8 s.** The record is
  `frontier-643a6283510c-1791783735115462000`, against 061's last frontier `…1791782586329441000`. It has the same
  stop at the appmodel API set, the same 57,445,097 instructions, and the comparison with game run 38206263976
  still conforms. The two runs make the same calls in the same order. The comparator, run between the two streams,
  finds 0 divergences over 2,685 calls; their digests differ only by the execution id in each trace's first line and
  by register leftovers past each function's parameters.
- **The second profile** runs 55.8 s under the profiler: about 20 s in the monitor's text pipe (1,659,004 exits),
  21 s in model steps (`_slow_sse`), and about 12 s in translation (`compile_insn`, decoding, lifting).
- **The snapshot and the translation cache are declined on the measurement** (Decision 2026-10-11,
  agent-resolved). Decision 2026-10-10's condition, "until a frontier run takes minutes", no longer holds. A snapshot
  holds the runtime image and the host's translations, so most sprints' changes invalidate it. A translation cache
  would save at most the 12 s. The larger shares belong to plan row 066 and the transport.
- **A guard test** checks that the map stays sorted after a probe's run. It would pass with the old sort too: it
  guards the invariant the change relies on.
