# Sprint 066 — threads that run at once

```yaml
---
id: 066
status: open
phase: 5
pass_kind: functional
---
```

## scope

Sprint 065 closed partial (Decision 2026-10-12): on the instruction clock the frontier writes unity.log equal to
Windows' 168 lines but for timings, two runs of one tree write the same log and calls, and the comparison with game
run 38216081440 finds 14 divergences over 3,443 calls (frontiers 643a6283510c-1791846403001347000 and
-1791848771286087000):

| Seq | Call | Windows | w2m |
|---|---|---|---|
| 3044, 3050, 3053, 3093 | UnityPlayer InterlockedFlushSList | 0 (empty) | an entry |
| 3132 | UnityPlayer InterlockedPopEntrySList | 0 | an entry |
| 3144, 3146, 3157 | UnityPlayer SetEvent on the job event (0x40c) | made | missing |
| 3268 | UnityPlayer HeapReAlloc (a second block's growth) | made | missing |
| 3487-3489 | the second message-pump turn | made | missing |
| 2146, 2238 | GetConsoleCP, GetConsoleOutputCP through GetProcAddress | made | never |

On Windows the main thread (9024) and Job.Worker threads (5612, 9732, 6832) flush and pop the same lists, and every
flush finds them empty: a worker on another of the runner's four cores takes each item as it is pushed. w2m runs
every thread on one vCPU in 200,000-instruction slices (scheduler.py TIME_SLICE), so items wait for the next turn.

1. **The measurement first.** Scratch frontiers with TIME_SLICE at 20,000 and 2,000: the divergences above, the
   instructions a second, the switches. A finer slice brings w2m's interleaving nearer the runners' parallel
   cores; what it costs in host exits decides between a finer slice, a scheduler that runs a woken thread at once
   (an idle core on Windows), and several vCPUs (review 2026-10-12 R2).
2. **The design chosen on the measurement,** written here before it is built, judged by the comparison and by a
   probe of a producer and consumers on several cores (rows from the runners).
3. **The calls w2m never makes:** GetConsoleCP and GetConsoleOutputCP (the game looks them up and calls them on
   Windows), HeapReAlloc 3268 and the message-pump turn, each traced to its cause.
   - *Measured 2026-10-12* (scratch frontiers on tree ef8a3ad7, run together on one host, each stopped by the
     reach rule with the log at the reference's 168 lines):

     | TIME_SLICE | divergences | instructions | M instructions a second | schedule exits (hvc 12) |
     |---|---|---|---|---|
     | 200,000 (065 close) | 14 | — | — | — |
     | 20,000 | 6 | 100,399,409,889 | 30.94 over 3,245 s | 8,019,918 |
     | 2,000 | 14 | 101,371,639,527 | 16.70 over 6,069 s | 27,886,521 |

     At 20,000 the SList flushes and pop (3044-3132) and SetEvent 3144-3157 conform; the 6 left are item 3's.
     At 2,000 those also conform, but SetThreadPriority and GetThreadPriority 1483-1506 go missing (8 new) and
     the rate halves.
2. *Chosen:* TIME_SLICE 20,000 (`src/w2m/scheduler.py`). The finer slice clears every interleaving divergence
   the comparison shows; running a woken thread at once and several vCPUs stay unbuilt until a comparison or a
   probe asks for them.
4. **Plan row 066's own work:** stacks laid out as Windows lays them out (reserve, guard page, commit on demand);
   SuspendThread, GetThreadContext and ResumeThread on a running thread, as mono's collector does.

## prerequisites

- 065 (closed partial: CI run 38327467991; main bb7c899b)

## context_files

- `src/w2m/scheduler.py`, `src/w2m/vm_backend.py` (`_switch`, `_schedule`, the instruction clock),
  `src/w2m/kernel.py`, `tools/guest_runtime/sync_objects.c`, `tools/guest_runtime/threads.c`,
  `scripts/call_compare.py`, `scripts/run_frontier.py`; review 2026-10-12 R2; v0.2 §9

## signal contract

### Emits

- Each probe's PROBE_AUTHORED; the threads family's item run (SESSION_INIT, CANDIDATE_GENERATED,
  COMPARISON_COMPLETED, CANDIDATE_ACCEPTED, SESSION_COMPLETED); the frontier's records.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- created: this card's probes and their families
- modified: `src/w2m/scheduler.py`, `src/w2m/vm_backend.py`, `tools/guest_runtime/threads.c`
- gates 0 and CI `success` on the closing commit

## observation contract

- **The frontier's comparison conforms** with game run 38216081440 but for FSIN and FCOS (pinned).
- **The threads family's new rows** (stack pages, a running thread's context, producer and consumers) equal
  Windows.

## done criteria

The frontier's comparison conforming but for FSIN and FCOS; the threads family accepted with its new rows.
