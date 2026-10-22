# Postmortem: w2m, 2026-10-07 to 2026-10-21

**Date:** 2026-10-21. **Status:** draft, unreviewed. **Scope:** the whole project, to Sprint 122's close (f2d071af).
**Format:** a project postmortem (Washburn et al., ICSE 2016) written to the incident discipline of Google's SRE
book: impact quantified, causes kept apart from the trigger, blameless, each claim linked to its evidence.

## Summary

w2m set out to make Hail to the Rainbow, an x86-64 Windows game, "playable on this Mac, at speed", and then to run
any game (docs/plan-2026-10-09.md, user goal of 2026-10-16). In 15 days and 873 commits it reached milestone A: in batch mode,
with no graphics, the game starts under w2m and writes the same 168-line `unity.log` it writes on Windows. It never
reached milestone B, the first frame. The host side of w2m (the scheduler's hand-overs, the Windows services that
leave the VM, files, processes, and every host thread added for several vCPUs) is CPython 3.12.13 with its global
interpreter lock. Sprints 107 to 122 built concurrency for several vCPUs out of Python threads that cannot run Python
at once. The trigger for this review was the user's question on 2026-10-21, after Sprint 122's close, whether that
work was being done in Python.

## Impact

| Item | Value | Source |
|---|---|---|
| Duration | 2026-10-07 to 2026-10-21 | `git log` |
| Commits | 873 (191 on 10-10, the peak; 1 on 10-19) | `git log --format=%ad` |
| Sprints | 122 cards | `sprints/` |
| Python | 49,654 lines (src, tools, scripts, tests) | `wc -l` |
| C, written by hand | about 11,100 lines (guest runtime 9,704, vm_monitor 544, the rest small) | `wc -l` |
| C, generated | `tools/vm_compiler/stencil_table.h`, most of that directory's 99,927 lines | `wc -l` |
| Process documents | 17,939 lines of docs, cards and signal reports; 16 reviews or recenterings | `wc -l`, `ls docs` |
| Milestone A | first held 2026-10-14 (frontier 1791973386975499000, 151.7 s on the instruction clock) | commit d33b5bd7 |
| Milestone B, C | not reached | plan-2026-10-09.md |
| Several-vCPU work | Sprints 107 to 122, 2026-10-17 to 2026-10-21, on Python host threads | `git log` |

## What was checked against evidence outside the project's own records

Each of these was checked for this postmortem on 2026-10-21, not taken from the project's documents.

1. **The Windows reference exists and is unaltered.** GitHub Actions run 38216081440 (workflow `windows-game`,
   runner `windows-2025`, CPU AMD EPYC 7763, 2026-10-12 07:08 to 07:23 UTC, success). Its artifact, downloaded
   fresh, matches `store/game-runs/38216081440/` byte for byte: `unity.log` sha256 741ad2ec…, the call trace
   d87830c1…. The reference set adds ten runs on the Windows laptop (`lap-20261014T032442034Z-c8814547-01` to `-10`).
2. **The CPU reference for instruction classes exists and is unaltered.** Runs 38011862284, 38122967607 and
   38238041162 are on GitHub, each a success, artifacts unexpired. 38238041162's `cpu.txt` reads "INTEL(R) XEON(R)
   PLATINUM 8573C", and all five files checked match the stored copies.
3. **The log matches.** A diff of w2m's `unity.log` (frontier 1792623423346201000) against the reference, numbers
   masked, differs on 5 of 168 lines, all timings ("UnloadTime", "Completed reload, in", GC "Total"). The game itself
   wrote both logs, so milestone A rests on the game's own output, not on w2m's account of it.
4. **What "168 lines" means in code.** The run stops when the log holds at least 168 lines and the main thread has
   made each function's calls (scripts/run_frontier.py, `stop_when`). The verdict "conforms" comes from the call
   comparison, not from the log's text; nothing in the code compares the log's text. The match in item 3 was
   checked by hand here, not by the project.
5. **What "0 divergences" covers.** Rerun from the stored files: w2m's trace holds 10,505 calls; the comparison
   pairs about 3,450 per reference run (3,440 to 3,452 across the eleven). The Windows capture records only the first
   32 calls of each function (`TRACE_CALLS 32`, tools/jit_capture/call_trace.h:33), so 7,012 of w2m's calls (775
   EnterCriticalSection, 628 GetCurrentThreadId, 290 ReadFile beyond the first 32, among others) have no Windows
   call to be compared with, and calls beyond a paired thread's reference length are not flagged. Nine (function,
   field) values are left uncompared as "undetermined", 24 values are ones Windows itself varies. "Conforms" means:
   the first 32 calls of each function match, with those exceptions.
6. **Where the batch run's time goes.** From MEASUREMENT_RECORDED on each record:

   | Run | Clock | Wall | In the VM | Host requests (hvc 8) | Hand-overs (hvc 12) |
   |---|---|---|---|---|---|
   | 1792623423346201000, 1 vCPU | instructions | 34.9 s | 29.2 s | 4.69 s, 41,299 (114 µs each) | none |
   | 1792623682795890000, 1 vCPU | wall | 107.2 s | 39.6 s | 4.41 s, 40,101 (110 µs each) | 61.2 s, 39,814, none with a thread ready |
   | 1792623570672147000, 2 vCPUs | wall | 111.7 s | 52.2 s | 2.76 s, 108,530 (25 µs each) | 79.1 s, 33,993 (25,194 with a thread ready, 8,799 without) |

   On the wall clock most of the run is the host handing over with no guest thread ready: the game waiting on real
   time. Python's serving of requests is 2 to 4% of those runs and 13% of the instruction-clock run. The GIL did not
   decide this run's speed on two vCPUs. Statements made to the user on 2026-10-21 before this table ("Python and its
   lock may hold back a second vCPU"; "the second vCPU added traffic to the queue and no throughput") were not
   measured and are withdrawn as stated.
7. **The instruction classes, rerun.** `class_run.py --all --pool 8` on f2d071af, into scratch, 62 s: 456 of 458
   classes accepted, 247,528 cases, model and VM both conforming to the Xeon's results; FCOS (1,728 cases) and FSIN
   (1,584) refused, as recorded.
8. **Free-threaded Python exists.** PEP 779 (Status: Final, Python 3.14) makes the free-threaded build "officially
   supported but still optional". The project runs a standard 3.12.13 build (`Py_GIL_DISABLED` unset).

## Timeline

All times 2026, local, from `git log`.

- **10-07** First commit; Sprint 006 closes the Windows runner's probe loop.
- **10-08** Sprint 030: lifted x86-64 compiled to AArch64 runs in a Hypervisor.framework VM; the translator and the
  driver are Python on the host.
- **10-09** Decision 0610f1e, the runtime architecture: managed VM, Windows services as C in the guest, "one vCPU per
  guest thread", "Python translator on the host behind one interface". Measured the same day: an exit to the host and
  back about 27 µs, through Python. Sprint 035: kernel32 as C in the guest.
- **10-10** Feasibility review: the 27 µs figure is the Python transport's, and graphics needs an asynchronous ring.
- **10-11** Sprint 051, threads: an M:N scheduler on one vCPU, switched by Python. Ground-up review G3: "The host is
  becoming the Windows kernel, in Python"; it notes no decision supersedes the rule that waits and switches never
  leave the VM.
- **10-12** Review: "All guest threads run on one vCPU, switched by Python"; "Milestone C's 16.7 ms frame cannot be
  met on one core". Windows reference run 38216081440.
- **10-14** Milestone A on the record, 151.7 s on the instruction clock (d33b5bd7). Thread switches move into the VM
  the same day (3c148d26).
- **10-16** Full review; the goal becomes "playable, at speed; w2m runs any game". Its deciding number: about 37,600
  system calls a second in play, from the laptop's recordings; at 14 µs a crossing, 0.53 s of each second. Step 1:
  several vCPUs under M:N.
- **10-17** 360 review: "Python on the hot path". Sprints 107 to 117: the host for several vCPUs, designed (113) as
  one lock held by whichever host thread serves; the kick (117).
- **10-18 to 10-21** Sprints 120 to 122: timed waits, a parked vCPU's wake, processes beside several vCPUs, each in
  Python host threads under one GIL. Instruction-clock batch run at 34.9 s (4.3 times faster than 10-14's).
- **10-21** The user asks whether this is done in Python. This review.

## Root cause and trigger

**Trigger:** Sprint 122's report that two vCPUs ran the batch run no faster, and the user's question that followed.

**Root cause:** the project had no test of feasibility against its goal. Every sprint was judged by conformance
(the game's calls, the Xeon's results, planted faults) and by a speed number on the batch run. No sprint and no
decision asked whether the host design, a single CPython process serving every exit, could meet the goal's own
figure of 37,600 calls a second in play. The architecture decision of 10-09 named the Python host as a choice
"behind one interface" with no condition for revisiting it. When the goal gained "at speed" on 10-16, nothing tied
the new goal back to that choice.

**Contributing factors:**

1. **Reviews were turned into sprints, never into a gate.** Four reviews (10-10, 10-11, 10-12, 10-17) named Python on
   the host path. The working agreement turns each finding into work items, so each finding became a sprint in the
   existing order, and none stopped the order.
2. **Local correctness hid the global question.** Sprints 107 to 122 each closed green: planted faults caught, CI
   success, 2,551 tests. That proved each step right on its own terms and could not show the steps led nowhere.
3. **The batch run as the only speed test.** It runs with no graphics, and on the wall clock it mostly waits on real
   time (item 6). Play, where the 37,600 calls a second arise, has never run under w2m, so no w2m measurement covers
   the case the goal names.
4. **Process load.** 16 reviews and recenterings in 15 days, and four hardening sprints (H1 to H3, 10-17) on
   vocabulary and records before step 1 resumed. The process checked each sprint's record and not the project's
   course.
5. **The founding document's rule on language** ("Changing language alone is not the architectural source of
   equivalence or speed", research direction v0.2) was read as settling the host's language, which it does not.

## Lessons

**What went well**

- The judges rest on real outside evidence: GitHub runs on Windows, an Intel Xeon's results, the laptop's runs, and
  the game's own log (items 1 to 3); the class sweep reproduces on rerun (item 7).
- Translation works: 81.5 billion guest instructions in 29.2 s inside the VM on one vCPU, and the game's start-up in
  batch mode reproduced.
- The instruction-clock batch run went from 151.7 s to 34.9 s between 10-14 and 10-21.

**What went wrong**

- The host design was never tested against the goal (root cause).
- Five days of several-vCPU work (Sprints 107 to 122) were built on Python host threads without a measurement of
  what those threads would cost or gain.
- "Conforms" was reported without its scope: the first 32 calls of each function (item 5).
- Claims made to the user on 10-21 were made before measurement (item 6).

**Where the project got lucky**

- The batch run's time is mostly the game's own waiting, so the host's cost did not yet show as a failure; the same
  cost in play would.

## Action items

| Action | Type | Owner | Priority |
|---|---|---|---|
| Decide whether w2m continues: the goal, and the host design it requires | decision | user | P0 |
| If it continues: measure, on a play-mode run, which calls reach the host and their rate; compare with 37,600 a second at the measured 25 to 114 µs a request | prevent | agent | P0 |
| State the comparison's scope with every "conforms": calls compared, calls unpaired, the 32-call cap | mitigate | agent | P1 |
| Give each architecture decision a revisit condition tied to the goal's numbers | prevent | agent | P1 |
| Let a review finding that questions feasibility stop the plan until a decision answers it | prevent | user | P1 |
| Hold several-vCPU work (step 1) until the first two items are answered | mitigate | agent | P0 |

## Supporting information

- Records: `store/runs/frontier-643a6283510c-1792623423346201000/`, `runs/frontier-643a6283510c-1792623570672147000/`,
  `runs/frontier-643a6283510c-1792623682795890000/`.
- References: GitHub Actions runs 38216081440, 38011862284, 38122967607, 38238041162 on laffeyp/windows2mac.
- Reviews: docs/review-2026-10-10-feasibility.md, docs/review-2026-10-11-ground-up.md (G3), docs/review-2026-10-12.md,
  docs/review-2026-10-17-360.md.
- Format sources: Google SRE book, "Postmortem Culture" and "Example Postmortem"; Google SRE workbook, "Postmortem
  Culture"; PagerDuty's postmortem template; Washburn et al., ICSE 2016; Kerth, *Project Retrospectives*.
