# Sprint 068 — speed toward the playable slice

```yaml
---
id: 068
status: open
phase: 5
pass_kind: functional
---
```

## scope

Milestone P (`docs/milestone-playable-slice.md`) needs the game's first scene in under 5 minutes on the Mac; the
laptop reaches it in about 60 s. The time census (commit d91a9e21) of frontier 1791942295792627000 split its host
time by exit kind: 2,366 s in the vCPU, 2,129 s in hvc 10 (the slow-SSE path), about 555 s in hvc 12 (the scheduler),
about 323 s in hvc 11. Nearly all of hvc 10 was one instruction, UnityPlayer's strstr at 0x18140b914 (PCMPISTRI
imm 0x0C, MXCSR 0x3f). This sprint removes host time in the order the census ranks it, one cause per item, each
measured by the census before and after and each leaving the frontier's comparison conforming.

- **A. PCMPISTRI natively** (done before this card; logged in `WORKING_AGREEMENT.md ## Hand-author log`):
  `tools/guest_runtime/string_compare.c` (d31f35e6, 78768984), index 1ecc22b2. Frontier on 1ecc22b2: hvc 10 down
  from 2,129 s to 0.5 s; 41.7 billion instructions in 186.8 s inside the vCPU.
- **B. Thread switches.** After A, hvc 12 is the largest host share (61.6 s of about 300 s at 41.7 billion), about
  55 µs an exit, and hvc 4 time-slice exits switch threads through the same `_save_thread`/`_load_thread`. A profile
  splits the 55 µs between the text pipe (`vget`, `vset`, `set`, `run`) and Python; the change follows the split.
- **C. Native lowerings for the model's integer SSE classes** (added 2026-10-13 from frontier 1ecc22b2's census past
  41.7 billion instructions, where hvc 10 rose again to 168.5 s): PMADDUBSW (344,613 steps), PSADBW (152,246), PEXTRW
  (150,337), CMPPS (112,370), MOVD (67,206), POPCNT (50,260), CVTPS2DQ (21,393), RSQRTPS (18,965), all with every MXCSR
  exception masked. Each lowering is judged on the Xeon's cases with a plant caught, as Sprint 064's were.
- **D. hvc 4** (budget exits) by the census after B and C.
- **E. The transport** (Sprint 064 item 3, not done there): the text pipe replaced by shared memory, unconditional
  since 2026-10-14 and finished before DXMT (Decision 2026-10-14, user). Measured first: a stub host call's crossing
  in the VM, times the Direct3D calls per frame in play run 38113343972.

Added 2026-10-14 (Decision, user: the review "Phase P: actionable review, 2026-10-14"):

- **F. Census runs alone.** A census frontier runs with the Mac to itself, or its `time:` lines record the load
  (the Mac's load average and the other w2m processes at each line). Frontier 1ecc22b2's numbers above shared the
  Mac with a class sweep, six parallel families and test runs.
- **G. The snapshot.** The run saved at the end of the conforming batch prefix and resumed from it, so an item's
  measurement does not wait on a 300 s rerun (supersedes Decision 2026-10-11's decline). The snapshot holds the guest's
  state, not w2m's: the memory file (an APFS clone, `cp -c`, copies it in constant time), each thread's registers,
  vectors and per-thread monitor data, the handles and kernel objects (src/w2m/kernel.py), the file bridge's open
  files, and every process of the Machine. It holds no translated code: the speed items change the translator, so a
  resumed run translates afresh, and a fork of the running process would keep the old translator's blocks and modules.
- **H. The clock, chosen per run.** A run option for the wall clock (play) and the instruction clock (the conforming
  comparison), recorded in the run's baseline; the instruction-clock comparison stays conforming.
- **I. The native workload.** On the laptop, the instructions the game retires per frame and to the first scene,
  per thread (the laptop runs under Hyper-V with VBS on; whether its PMU counters are readable is checked first).
- **J. Spinning.** PAUSE instructions and instructions per thread counted; if spinning on a switched-out thread's
  lock is a large share, PAUSE hands the vCPU on. Counted again after several vCPUs.
- **K. Block chaining** for direct branches, undone when aabe522e drops a block; a stale-chain plant code_rewrite
  must catch.
- **L. Flag liveness within a block**; a live-flag plant the class runs must catch.
- **M. The instruction counter off every block entry.**

## prerequisites

- 066 (closed: CI run 38506623160)

## context_files

- `src/w2m/vm_backend.py` (`steps`, `_schedule`, `_switch`, `_save_thread`, `_load_thread`, `vectors`), `src/w2m/kernel.py`
  (`Machine._next`), `tools/vm_monitor/vm_monitor.c`, `scripts/run_frontier.py`, `docs/milestone-playable-slice.md`

## signal contract

### Emits

- The frontier's records (EXECUTION_STARTED, its stop, TRACE_INGESTED, COMPARISON_COMPLETED, SESSION_COMPLETED)
  after each change; the family sweep's records (CANDIDATE_ACCEPTED for all 68 families) on each runtime change.

### Invariants

- Vocabulary unchanged; no new tags without a proposal.
- The frontier's comparison against runner run 38216081440 stays conforming (0 defects by `call_compare.by_cause`).

## artifact contract

- modified: `src/w2m/vm_backend.py`, `tools/vm_monitor/vm_monitor.c` where an item needs it; a test that fails before
  each change
- `uv run pytest` exit 0; the family sweep 68 of 68 accepted; CI `success` on the closing commit

## observation contract

- **The census before and after each item,** from the frontier's `time:` and `progress:` lines: seconds by exit kind
  per billion instructions, and instructions per second of wall time.
- **The frontier conforms** with the same calls before its stop.

## done criteria

Each item's gain measured on a census run that had the Mac to itself, the comparison conforming. No threshold closes
the sprint (Decision 2026-10-14, user; it replaces "host time outside the vCPU below the vCPU's own time").

## observed

- **A.** Family sweep 68 of 68 accepted on the native runtime (index 1ecc22b2). Frontier on 1ecc22b2 at 41.7 billion
  instructions: in run 186.8 s; host by exit hvc 12 61.6 s, hvc 4 33.8 s, hvc 1 10.4 s, hvc 8 7.3 s, hvc 10 0.5 s.
- **B, the profile.** A cProfile of 400 million instructions (scratch) made 205,614 switches: `_schedule` 10.3 s
  cumulative, about 50 µs a switch, of which `vectors()` (a `vget` round trip and its hex parse) 4.2 s and the
  `vset` about as much. Each pipe command waits about 12.5 µs in `readline` (1,028,724 commands, 12.8 s).
- **B, the change.** `vm_monitor.c` `vswap` copies Q0-Q15, FPCR and FPSR to the outgoing thread's slot
  (`VECTOR_SLOTS`, in the monitor area's padding) and loads the incoming one's, sent in the same write as the next
  command. `tests/test_thread_switch_vectors.py`: the threads and running_thread_context probes keep Windows' rows
  with `vget`/`vset` only for model steps; a direct swap inside a running VM; a dropped load and a dropped save are
  each caught. The two probes alone did not catch the dropped load: neither holds a vector across a switch.
- **B and C, measured (item F's way).** Two scratch frontiers to 30 billion instructions, one at a time, each
  recording the Mac's load: A on 1ecc22b2 (before B and C) 145.3 s, 206.4 million instructions a second; B on
  69ceeee8 117.3 s, 255.7 million. Both made the same exits (hvc 4 1,498,523; hvc 12 608,164; hvc 1 124,201; hvc 8
  37,061; hvc 10 4,923). At the last progress line hvc 12 fell from 21.5 s to 4.7 s and hvc 4 from 13.1 s to 9.7 s.
  A first B run took 145.7 s: Ollama's llama-server used 180-250% CPU as it started (its `load:` line names it), and
  its hvc 1 time rose from 7.4 s to 22.7 s at the same 124,201 exits. Load moves a census by tens of seconds.
- **I, the native workload** (laptop run lap-20261014T041748433Z-73c66e6a-01: play mode, the game's own exe without
  the capture, 90 s, WPR with InstructionRetired on context switches, reduced by `pmc_summary.py`). The counters count
  under VBS. The game's process retired 458.7 billion instructions on 71 threads; 325.4 billion in the first 62 s (the
  screen is black at 30 s and shows the stairwell at 60 s). From 35 s on: 4.6 billion a second and 112 frames a second
  (9,122 Presents), about 41 million instructions a frame. Two threads carry 28.9% and 24.7%, four more about 9% each.
  Against w2m's 255.7 million a second (one vCPU, quiet): the first scene's work takes about 1,270 s; a frame's 41
  million about 0.16 s (about 6 frames a second); with the work across vCPUs, the busiest thread's 11.9 million a
  frame bounds it near 21. The native counts include the in-process NVIDIA user-mode driver, which w2m replaces with
  DXMT and Metal, so they bound the work from above.
