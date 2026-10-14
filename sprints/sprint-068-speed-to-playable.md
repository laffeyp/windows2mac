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
- **J, the spin census** (scratch frontier to 30 billion instructions, alone: 115.3 s, 260.3 million a second). The
  main thread ran 98.4% of the instructions, thread 12 1.3%, the rest under 0.3%; the run made 198 PAUSEs. This stretch
  of the batch-mode start is one thread's work: a PAUSE handoff gains nothing here, nor would several vCPUs. The
  laptop's play frames spread over six threads (item I), so the census runs again on play once the slice plays.
- **D, tried and reverted.** A budget with no time slice while no other thread is ready left hvc 4 at 1,491,764 exits
  (from 1,498,523): at almost every slice end another thread is ready. The cause in the counts: `wake_all` readies
  every blocked thread on any signal, and each runs a few hundred instructions to find its wait unmet and blocks again
  (hvc 12, 524,271). Waking only an object's waiters needs the host to know what each thread waits on, which the
  guest runtime holds (sync_objects.c); recorded as a later item, not built.
- **K, tried and reverted.** Direct branches chained through a per-site literal (`ldr x16, literal; br x16`, the host
  rewriting only the literal, so no instruction cache needs cleaning; unchained when aabe522e drops a block). The
  code_rewrite probe gained row `chained` (a direct call from a second page into a rewritten routine; laptop run
  lap-20261014T044509652Z-627bbd99: 11, 11, 12; family 0.2.0), and a stale-chain plant broke exactly that row.
  Measured by alternating runs to 10 billion instructions, three each: inside the vCPU 16.9, 17.0 and 17.3 s chained
  against 17.1 s twice unchained (one run void under Ollama's load); wall 41.9-47.9 s against 43.6-44.7 s. The
  dispatcher's lookup is not what bounds the vCPU here, so chaining came out; the probe row stays.
- **F, the Mac's cores.** A loaded Mac doubled a census's CPU time for the same work (user 81 s against 146 s), the
  sign of threads moved to the efficiency cores. The monitor's vCPU thread and run_frontier's driver now run at
  user-interactive QoS (0x21), which macOS keeps on the performance cores; its gain is not yet shown (one run still
  took 197 s with llama-server at 98% and WindowServer at 53%). Census runs keep alternating settings and take the
  lower times.
- **E, the channel** (e3a3f4e7; indexes a7fe4090: 456 of 458 classes, 68 of 68 families, 2,246 tests). Commands and
  exits through a shared page, each side waking the other with os_sync_wait_on_address and
  os_sync_wake_by_address_any after a spin of up to 50 µs. Alternating runs to 10 billion instructions on a quiet Mac,
  three each: the pipe 45.1, 45.9 and 47.9 s; the channel 22.4, 23.1 and 22.7 s. User time is the same (34.2-34.6 s
  against 35.9 s), the spin taking what the pipe's wake-ups did. A race found on the way: CPython's struct wrote the
  host's sequence word a byte at a time, a spinning monitor read 0x00 between 0xFF and 0x100, and a run executed
  twice; the word is now one 32-bit store and the monitor waits for exactly the next value.
- **Recentering step 3, the code census** (worktree, BLOCK_CENSUS: each block counts its executions). Over the first 10
  billion instructions each x86 instruction ran 12.26 arm64 words in its body and 1.73 in block prologues (an upper
  bound: a block counts in full when it leaves early). Flag-setting classes and conditional branches take about 75% of
  the words: CMP 16.3% (25.9 words each), ADD 15.4% (26.4), SUB 6.9% (26.3), TEST 4.7% (19.3), JNZ 4.6% (15.0), SHL
  4.3% (41.5), AND 4.3% (21.8); MOV takes 3.5 words, LEA 3.9. Every flag is computed eagerly into its own register.
- **L, flag liveness** (085227a7; index 82b45861, CI 38534809973). A setter computes only the flags a later instruction
  reads: a backward pass over the block from the flags live at its end, which come from scanning each direct successor
  (8 instructions, following direct jumps, only on pages already behind translated code, the block joining those pages
  so a write drops it). Model steps, #UD and DIV read every flag; a load or store does not, so a fault where the flags
  are dead shows an earlier instruction's flags in EFlags (Decision 2026-10-14, agent-resolved). Census over 10 billion
  instructions: 6.05 arm64 words per x86 instruction in bodies to 4.02 (CMP 10.56 to 5.06, ADD 11.07 to 5.68). The
  class runs cannot judge it: they compile one instruction per block (lockstep), so the card's "a live-flag plant the
  class runs must catch" has no reach there. `tests/test_flag_liveness.py` judges it instead: 300 seeded random
  programs of setters, readers, loads, stores, jumps and PUSH-POP pairs, run with the pass and without, equal registers
  and RFLAGS required; plants (SETcc and CMOVcc reading nothing, a successor's reads dropped, INC writing CF) each fail
  it.
- **Operand forms** (74c13a67, 72aba3c6): guest registers read in place (64-bit, and 32-bit where every use takes the
  low half), ADD and SUB immediates, branch targets built straight in x17, results computed into the register they are
  stored to when nothing after can fault or read its old value, W-form 32-bit ALU ops, and a dead flags op reading
  nothing. Bodies 4.02 to 3.31 to 2.91 words. Plants caught by class runs: an alias read after its register is written
  (XCHG, XADD, CMPXCHG), SUB's immediate added (PUSH, CALL_NEAR), a branch target off by one (JMP, JNZ, CALL_NEAR), a
  forward past a store that can fault (one of XCHG, XADD, CMPXCHG, POP). Two guards no class reaches (the lifter
  stores a register before the flags op that reads its old value; every 32-bit lowering reads only low bits) are
  judged by `tests/test_operand_forms.py`.
- **The passes, timed** (alternating runs to 30 billion instructions, three each, 104511ce against the passes): in the
  vCPU 14.1-15.5 s against 9.5-10.1 s; hvc 1, the Python compiler at each dispatch miss, 7.6-8.3 s against 13.6-14.6
  s; wall 52-59 s against 58-60 s. A 3-billion-instruction profile: 89,427 blocks compiled, `_compile` 28.4 of 42.6 s.
  The lift cache (db71015a: lifted instructions kept until their page is written; 863,265 lifts to 486,738) brought
  hvc 1 to 11.6-12.2 s and wall to 53-56 s, level with before the passes; two of those runs shared the Mac with
  another VM and QEMU. The rest of the compiler's time is the per-run Python compile, recentering steps 6 and 8.
- **Recentering step 3b, the PE images' share** (0ea202fd; laptop run lap-20261014T092507362Z-0ea202fd, play mode,
  the game's own exe, 120 s, an interrupt per 1,000,000 retired instructions): 592,230 samples. Play (35-120 s):
  UnityPlayer.dll 76.9%, the NVIDIA user-mode driver 14.5%, ntoskrnl 3.0%, d3d11 1.5%; code in no image (Mono's
  compiled methods) is not among the first six. Load (0-34 s): UnityPlayer.dll 78.9%. Of the work w2m translates in
  play, UnityPlayer.dll is about 90%: the ahead-of-time cache is for the PE images.
- **D, again: the wake path** (recentering step 5's first piece; 44a557dc, index e8c5dc1b). Every blocked wait names
  its key (the section, the lock, the variable, the object; 0 for a wait on several or a pipe; none for Sleep), a
  signal wakes only the threads waiting on its key, and the host publishes the waited keys so a signal nobody waits on
  makes no exit. Frontier to 10 billion instructions: hvc 12 202,183 to 240, hvc 8 12,569 to 3,213; the call stream's
  findings unchanged. The plants on the keys first changed no family row: no probe woke a wait on several objects by a
  signal. wait_handoff 0.2.0 (laptop run lap-20261014T094031671Z-93f645c3) adds the row: Windows gives the waiter the
  second event and the setter's zero-timeout wait 258; w2m gave it to the setter. Fixed (a wait on several is handed
  over), and the plants change that row alone.
- **Slice ends and timed-out waits** (recentering step 5; 76111810). Of 498,839 slice-end exits (hvc 4) over 10 billion
  instructions, 354 found another thread ready; 496,746 had a blocked thread with a deadline. The host publishes before
  each run how far the running thread may go (SLICE_UNTIL, RUN_END), and each block's stop calls one slice stub that
  refills the slice in the VM. The first run kept the exits: a blocked thread whose deadline had passed ran only when no
  thread was ready. Slice ends now make such threads ready, as Windows runs a timed-out wait. The game moved: unity.log
  79 lines within 10 billion instructions (13 before), and the call stream at the cut 0 divergences (76 before, all
  Windows calls on worker threads w2m had not run). Those threads brought new model classes (hvc 11 237,886, hvc 10
  184,027 per 10 billion).
- **C, the classes the worker threads reach** (e84731b6, 37843daf, e6a9b14c, ceda93ac): POPCNT (CNT and ADDV), CMPPS and
  CMPPD (FCMEQ quiet, FCMGE and FCMGT signaling, as x86's predicates), the MMX MOVD and MOVQ with a general register,
  PEXTRW and PSADBW, CVTPS2DQ (CVTPD2DQ's lanes on four singles), and RCPPS, RCPSS, RSQRTPS, RSQRTSS through the Xeon's
  tables in the runtime's C (approximate.c, approximation_tables.h generated from specs/approximations.json). Every
  class accepted on model and VM; two plants each, all caught (LE as LT only by CMPPD: CMPPS's 114 cases hold no equal
  lanes there).
- **The batch run to its end** (3a51f23d, frontier-643a6283510c-1791973061643135000, scratch): the frontier with no step
  cut stopped on its own rule, the log holding the reference's 168 lines, after 152 s and about 32 billion instructions,
  213.8 million a second. The call stream: 0 divergences, 0 presence divergences, 29 values Windows varies, against
  the reference set of eleven runs. Host time by exit at the last progress line: hvc 1 36.7 s, hvc 4 17.0 s, hvc 10
  14.9 s, hvc 8 7.8 s, hvc 12 6.9 s, hvc 11 3.9 s, in the vCPU 62.3 s. Plan row milestone A (batch mode) is met on this
  run; the record from a committed tree comes next.
- **The dispatch table two-way** (d4fee181): the full batch run's 997,681 dispatch misses were 257,593 compiles and
  871,681 blocks already compiled whose one slot another target held; a pair of slots per index, the newest first:
  275,397 misses.
- **The translation cache** (1821c223, recentering step 8): blocks kept with the bytes they read, reused when they hash
  the same; warm, the batch run compiles nothing (255,797 blocks from the cache), hvc 1's host time 37.5 s to 5.1 s,
  154 s to 123 s.
- **C, the last slow classes** (78d5b7d5): VCVTTPD2DQ, VPCMPEQQ, VANDNPD, VPMOVSXDQ, MMX PMADDUBSW, CVTTPS2DQ,
  MOVNTDQ, EMMS: hvc 10 334,637 to 16,356, hvc 11 82,186 to 23,372 in the batch run. Its run showed 2 divergences,
  mono-2.0-bdwgc.dll's VirtualAlloc sizes on thread 108: a native-against-model differential of the conversions on
  6,018 blocks agreed, a bisect in three worktrees named the CVTT* group, and the batch run conforms at clock rate
  1.386 GHz and diverges at 1.400 and 1.414: the size follows the interleaving on one vCPU. The lowerings had ended
  model steps that counted the rest of their block twice. Undetermined, scoped to that image (91f6e329; Decision).
- **The instruction count** (3d052448): the host computed the count from the limit read before a run while the slice
  stub raised it in the VM (a 400,003-instruction loop with refills counted 3; the guest's clock could go back), and a
  model step's block counted its unrun instructions again. With both fixed the batch run counts about 694 million
  instructions a second, not 246: every rate since 76111810 was short. It conforms: 0 divergences, 168 of 168 lines.
  Deferred: a fault's unrun block instructions (BLACKBOARD).
