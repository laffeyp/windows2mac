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
   - *GetConsoleCP, GetConsoleOutputCP (2146, 2238):* a trace difference, not a call w2m skips. call_trace.h's
     log_get_proc_address answers a traced image's GetProcAddress with a traced thunk and records every call
     through it; w2m recorded a call through an unowned thunk only from code inside UnityPlayer or mono, and
     mono's p/invoke wrappers are JIT code. Fixed in 4f81e018: a traced image's GetProcAddress with the trace
     open gets a thunk of its own, owned by the GetProcAddress label.
   - *The message-pump turn (3487-3489):* message 0x60, which Windows posts to a system window after activation
     (family windows' open question). Posted by w2m as SetFocus's activation completes, Server 2025's wParam
     0x80000001, in d943c971.
   - *HeapReAlloc 3268:* an interleaving divergence. Thread 7868 (BatchDeleteObjects) grows one delete buffer
     0x817, 0x1017, 0x2017, then a second to 0x817; UnityPlayer's 8 HeapReAlloc records are under the capture's
     32, so Windows made exactly four. w2m's worker (thread 88) makes the first three and no second buffer at
     slices of 200,000, 20,000 and 2,000: on Windows the worker deletes while the main thread still queues.
4. **Plan row 066's own work:** stacks laid out as Windows lays them out (reserve, guard page, commit on demand);
   SuspendThread, GetThreadContext and ResumeThread on a running thread, as mono's collector does.
   - *What the held rows answer (2026-10-13):* windows.thread.threads 0.2.0 rows suspend, suspend.frozen,
     context and resume are a spinning thread suspended while it runs (RIP in the image, RSP in its stack, CS
     0x33, SS 0x2B); the family conforms on tree f419f145, and the frontier makes mono's SuspendThread,
     GetThreadContext and ResumeThread 40 times each (frontier 643a6283510c-1791846403001347000), none
     of them among the 14 divergences above. windows.memory.thread_stack 0.1.0 holds the main thread's layout (reserve,
     3 PAGE_GUARD pages below StackLimit, committed above), which w2m answers; it conforms. No held row shows a
     created thread's stack, StackLimit moving as a stack grows, or a CONTEXT's other registers.
   - *Probes for the rest, not yet run:* `probes/thread_stacks` (each created thread's TEB relations and
     VirtualQuery pages; StackLimit after touching 1 to 8 pages below it; a touch 16 pages below under a vectored
     handler) and `probes/running_thread_context` (every register planted in a spinning thread, CONTEXT_ALL
     after SuspendThread; a thread suspended in WaitForSingleObject). A local mingw build in the VM: the planted
     GPRs, XMM0-15 and MXCSR 0x9FC0 read back; a waiting thread's RIP lies in no module (the bridge thunk); the
     fifth touch below a created thread's StackLimit faults (w2m keeps 4 reserved pages there and never moves
     StackLimit). Commit on demand is built when the rows give the initial commit, the guard and the step.

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

### Windows rows for item 4 (laptop, 2026-10-13)

Probes `thread_stacks` and `running_thread_context` (5cb60b6d), built and run on the laptop, two executions each
(runs lap-20261013T234430841Z-5cb60b6d and lap-20261013T234642777Z-5cb60b6d, 64206f06):

- A created thread reserves 256 pages (64 with STACK_SIZE_PARAM_IS_A_RESERVATION at 0x40000), commits 2 (32 when
  0x20000 is asked for), and keeps three PAGE_GUARD pages below StackLimit (protect 260). Each page touched moves
  StackLimit down one page (`default.grow` moved_1..moved_8 = 1..8). A touch 16 pages below the limit is
  0xC0000005, and the stack does not move.
- `reserve.teb` committed_pages read 2 in one execution and 1 in the other: when the second page is touched is
  timing; the family takes either.
- A thread suspended in a wait has RIP in ntdll (`context.waiting`); w2m parks it at its bridge thunk, so this row
  will diverge until the parked context reports an ntdll address.
- A running thread's context returns every planted register, XMM0-15 and MXCSR 0x9FC0.

Next for item 4: commit on demand (three guard pages, one-page growth) and the waiting thread's RIP, judged by
these rows once each probe has a family entry.
