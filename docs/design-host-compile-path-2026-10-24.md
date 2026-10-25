# The host's compile path: the run loop first, then the translator (Sprint 139)

Step 3 of `docs/rust-port-plan-2026-10-24.md` (lines 114-120) has the Rust host run one guest of
`tests/data/timed_waits` on the existing C guest runtime and serve its exits; step 4 (lines 121-126) ports the
translator afterwards. Every one of those guests needs a compiler on the host before it can finish, so the steps
cannot run in that order. This document gives the measurement, weighs six answers, and decides: the skeleton's run
loop runs first, up to the first block the C compiler declines; the translator follows, each part judged through
that run loop; the skeleton's gate stays.

## The measurement

On 2026-10-24 the frozen Python host ran each of the 14 guests in `tests/data/timed_waits` on one vCPU with the C
compiler in the VM on, as the Rust host will run them. The command wrapped `VM._compile` with a counter and read
`VM.vm_compile_counts` at the end, through `tests/test_timed_waits.py`'s `run_guest`, from an inline
`python -c` in the shell: no file was added. Every guest stopped at `ExitProcess` with its expected output.

| Guest | Host compiles | C compiled | Not a code page | No stencil | Cut by no stencil | Needs a split |
|---|---|---|---|---|---|---|
| busychild | 6 | 21 | 11 | 3 | 4 | 1 |
| crosswake | 23 | 18 | 11 | 11 | 7 | 1 |
| handshake | 9 | 18 | 11 | 6 | 4 | 1 |
| mutual | 54 | 141 | 11 | 54 | 46 | 0 |
| mutual_context | 52 | 143 | 11 | 52 | 45 | 0 |
| pingpong | 12 | 18 | 11 | 6 | 4 | 1 |
| qpc | 2 | 6 | 4 | 2 | 3 | 0 |
| qpc2 | 4 | 11 | 8 | 3 | 2 | 1 |
| readied | 6 | 20 | 9 | 5 | 4 | 1 |
| sleeper | 5 | 14 | 9 | 4 | 2 | 1 |
| spinchild | 11 | 17 | 11 | 7 | 5 | 1 |
| statsize | 7 | 14 | 8 | 6 | 4 | 1 |
| twoproc | 8 | 18 | 8 | 5 | 4 | 0 |
| twoready | 10 | 35 | 9 | 10 | 8 | 0 |

The C compiler declines in three places. Most often it declines in `w2m_vm_compile`
(`tools/guest_runtime/block_compiler.c` 231-233), which counts "not a code page" for a target outside the guest's
pages. The host's import thunks sit at `THUNK_BASE`, which is `PROCESS_VA` (`src/w2m/loader.py` 31), so a call
through an import lands here, and the host answers it with a bridge to the runtime's service (`self.bridges`,
`vm_backend.py` 5239-5240), not a compile. Those declines need the bridges, not a compiler. The other two are
`vmc_decode`, when no form matches the instruction's key (`tools/vm_compiler/vm_compiler.c` 244), and `vmc_emit`,
when no variant matches the flag arguments the instruction meets (268); these are the "no stencil" counts. With
"needs a split" (a page the host must split before it protects it) they equal the host compiles in five guests (qpc
2, sleeper 4 + 1, twoready 10, mutual 54, mutual_context 52). In the other nine the host compiled more (crosswake 23
against 11 + 1, pingpong 12 against 6 + 1), which these counts do not explain; the run loop's first link counts each
host compile by cause. No guest runs without at least 2 host compiles, and the blocks are plain code: `CMP JNZ`, `MOV
SUB MOV MOV CALL_NEAR`, `ADD CMP JZ`, `CMP JLE`.

The table holds the 21,182 keys one batch run reached; 89 to 94% of instructions from outside that run have no
stencil (BLACKBOARD Decision 2026-10-15, "the inferred compiler put in the VM"), and 3,564 of its 21,239 key choices
(1,227 of its 4,166 stencils) carry guards that refuse values the samples never varied (`vm_compiler.c` 273-287,
318-319). Decision 2026-10-14 already says what a decline does: "a form with no stencil exits to the host, which
compiles it". The miss reaches the host as `hvc #1` with x17 the target (Sprint 136). So the skeleton needs two
things on the host: the bridges, and a compiler.

## Six answers

**A. The translator first, then the run loop.** About 6,500 lines of Python: the rules (254 files, 4,135 lines), the
lowering and `compile_insn` (`src/w2m/vm_backend.py` 557-1891), `lift.py` (355), `x86dec.py` (287), `x86ops.py`
(167). It puts five or more sprints before any guest runs, and with them the measurements the host design left to
the skeleton (an exit's time, the runtime word's hold times, whether a guest WFE exits). It repeats the error the
user's Decision of 2026-10-24 named: the first end-to-end run moved later.

**B. The frozen Python compiler called from the Rust host.** Keeps Python on the running path step 4 removes, and
every declined block becomes a request between processes (0.58 to 0.71 µs a round trip on this Mac, the host design
of 2026-10-24) plus the Python's compile. The bridge is built to be thrown away. Not built.

**C. The stencil table widened until the skeleton's guests need no host compile.** It removes the misses of small
guests and not the game's, whose unseen instructions lack a stencil 89 to 94% of the time; the generator,
`scripts/gen_stencil_table.py`, is frozen Python. Not pursued.

**D. A guest the C table covers whole.** Measured above: none of the 14 does. Ruled out.

**E. The declined blocks' words stored from the frozen host as a fixture.** The words of a block depend on where the
host puts it in the code cache, on the exit stub's address and on the runtime's symbols (`compile_insn`'s
`exit_va`, `symbols`), so a fixture binds the Rust host's layout to the Python's, and it judges nothing the
translator's own judges do not. Not built.

**F. The run loop first, up to the first decline; then the translator through it.** The skeleton's run loop is built
now with the bridges and the C compiler in the VM, and its first gate is honest about what it lacks: it runs the guest
until the first `hvc #1` the C declined that no bridge answers, stops there by name, and names the same block the
Python host compiled first. The translator's parts follow, each judged through that run loop, and each sprint lets
the guest run further, until the skeleton's gate holds. **Chosen.**

A seventh answer, QEMU's shape, is one translator for every miss: either the Rust translator alone, with the C
compiler in the VM removed, or C in the VM generated from the rules (the recentering document's change 2, from which
Decision 2026-10-15 departed). F keeps the C compiler in the VM because it is what keeps the miss inside the VM for
the forms it holds (the host design's budgets), and keeps the host's translator because step 5 translates the PE
images ahead of time on the host, which needs it whole. The C table remains a cache of forms in front of it.

## What judges the translator

Each part is judged in its own sprint, never a step later (fix (c) of the user's Decision of 2026-10-24):

- **The C emitter's words**, where it emits. Where the C has a stencil, its words equal `compile_insn`'s (four judges
  in `scripts/stencil_judge.py`, two from outside the table's sources; Decision 2026-10-15). The C, built for the
  host, compiles the calls of `store/stencil/record.txt.gz` (1,186,512 compile calls: address, bytes, flag
  arguments), and the Rust lowering must give the same words wherever the C does. How many of the 1,186,512 the C
  emits for, and how many of the 254 rule files those calls reach, is measured at the decoder's sprint, the first
  that builds the C for the host; it is not known today.
- **The class cases, through the run loop.** Sprint 039's CPU cases (`probes/semantics*/cases.bin`) with the Windows
  runners' results stored beside them are the class judge (`tests/test_class_runs.py`). They need more than the
  guest's run loop: `scripts/vm_oracle.py` `run_cases` runs them in lockstep (`VM(ld, lockstep=True)`, line 184),
  swaps each case's state in through `on_dispatch` (236) and judges a fault through `on_exception` and `judge_fault`
  (243-246), the guest's exception path. A link of its own builds that case runner (link 3 below). Classes the host
  lowers to the model exit with `hvc #10` or `#11` (`HVC_SLOW_SSE`, `HVC_SLOW_STEP`, `vm_backend.py` 1118, 1125), and
  the model is step 4's; their cases wait for it. Each rule group's sprint runs its other classes' cases through the
  Rust host and compares with Windows.  This judges the forms the C table lacks, which the C emitter cannot.
- **The skeleton's guests.** Each sprint's gate names how far the guest now runs: the next declined block compiled,
  or the guest's stored output.

## Against the goal

The game retires 41 million instructions a frame at 112 frames a second, about 4.6 billion a second over six threads,
the busiest about 1.33 billion; it makes 37,600 system calls a second; its first scene is 325.4 billion instructions
in, at 62 s on the laptop (`docs/recentering-2026-10-14.md` 35-36, 46; `docs/plan-2026-10-09.md` 23-24). Every
instruction runs as translated code, and every block entered for the first time is compiled by the C in the VM or by
the host, so the host's compiler lies on the path to the first scene for every block the C declines. On the 14
guests the C left 22 to 56% of compiled blocks to the host (twoready 10 of 45, sleeper 5 of 19, crosswake 23 of
41); Decision 2026-10-15 reopens the compiler's design when more than 5% of a play stretch's first-entered blocks
go to the host, a line drawn for play, which these test guests do not measure. A host compiler native to the vCPU's
thread (the host design's change 3: a region per vCPU, a lock-free lookup) is the only answer that reaches the first
scene; B stalls a 8.9 ms frame on a Python compile, C and D leave the game's declines unserved, E serves only stored
blocks. F differs from A in order, not in what it builds: it measures the run loop's numbers now (an exit's time, the
runtime word's hold times, WFE), which the host design needs before step 6, and gives the class judge its run loop
before the rules arrive.

## Against the systems that solved it

- **QEMU** looks for the next block on every exit from translated code, "translating it from the guest architecture
  if it isn't already available in memory" (QEMU, TCG internals, https://www.qemu.org/docs/master/devel/tcg.html).
  One translator serves every miss. F keeps two, the C cache of forms in the VM and the whole translator on the host,
  for the reason given under the seventh answer; QEMU's lesson it keeps is that the host's translator is whole.
- **FEX**'s dispatcher, on a lookup miss, spills its registers and calls `CompileBlock` ("Need to create the block",
  https://github.com/FEX-Emu/FEX/blob/main/FEXCore/Source/Interface/Core/Dispatcher/Dispatcher.cpp) on the same
  thread, in the same process. The miss is a call, not a request to another process; B is ruled out by it.
- **Rosetta 2** translates ahead of time at install and compiles code it has not seen "on the fly" (WWDC20 session
  10686, https://developer.apple.com/videos/play/wwdc2020/10686/): two paths, each complete for the code it meets.
  The plan's step 5 is the first; the host translator is the second; the C table, inferred from one run, is neither.
- **Wine** on arm64 does not translate. Wine 9.0 implements "the 32-bit x86 emulation interface", with "no emulation
  library ... provided with Wine": "an external library that exports the interface can be used", and FEX "implements
  this interface when built as PE"; 64-bit x86 emulation was to follow through an ARM64EC build (Wine 9.0 release
  notes, https://gitlab.winehq.org/wine/wine/-/raw/wine-9.0/ANNOUNCE.md). FEX is "a drop-in replacement for xtajit64"
  (FEX wiki, ARM64EC, https://wiki.fex-emu.com/index.php/Development:ARM64EC). The delegate is a whole JIT loaded into
  the same process. B's delegate, the frozen Python compiler, is whole too, but it is not in the process and is not
  kept.
- **crosvm** runs a thread per vCPU and serves its exits there, each device behind its own mutex, and it puts virtual
  devices in their own processes, "A process per virtual device, made using fork", for isolation, crossing to them on
  a device access (https://github.com/google/crosvm/blob/main/ARCHITECTURE.md). It pays that crossing for isolation of
  an untrusted device; a compile miss on w2m's hot path has no such need, so B's crossing buys nothing.
- **Firecracker**: "one or more vCPU threads (one per guest CPU core)", which "execute synchronous I/O and
  memory-mapped I/O operations on devices models"
  (https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md). F's run loop serves the miss exit on
  the vCPU's own thread, as these serve theirs.

## Decision

F. The links that follow the loader (Sprint 138):

1. **The run loop, to the first decline.** The process area (`_init_process`), the bridges (`_bridge`,
   `vm_backend.py` 3733), the C compiler in the VM enabled, and the exits the guest reaches before its first decline.
   Gate: `qpc` runs from its entry until the first `hvc #1` no bridge answers and the C declined, stops there by
   name, and names the block the Python host compiled first. It measures an exit's time and the runtime word's hold
   times, and whether a guest WFE exits.
2. **The decoder**, Rust generated from `specs/x86_decode.json`, judged against `x86_decode.c` over every instruction
   of the stencil record and the PE images' `.pdata` functions; it measures the C emitter's coverage of the record.
3. **The case runner.** Lockstep, a case's state in and out, the fault path through the guest's exception
   record and context, carried from `scripts/vm_oracle.py`. Judged on the cases whose classes the C stencils, each
   case's instruction compiled by the C emitter built for the host (link 2), against the Windows results; it counts
   the cases that can run at step 3 (no model exit) and the rule files they reach.
   *Amended 2026-10-24 (Sprint 145), by measurement:* the C emitter compiles none of the cases at their own address
   (0x2000_0000), because the stencil key carries the code address's value class and the table's keys are the game's,
   above 4 GB. The runner takes its compiler as an argument. Its first run used the frozen Python lowering's words,
   compiled for the runner's fixed block address: 148,614 completed cases from 22 Intel runs agreed with Windows. When
   the Rust lowering exists, it is the compiler the runner judges against Windows, and no second compiler is needed.
4. **The lifter and the operand forms.**
5. **The rules and the lowering, in groups**, each group judged by the C emitter's words on its covered calls and by
   its classes' cases run through the run loop against Windows; each lets `qpc`, then `sleeper`, run further.
6. **The skeleton's gate**: the guest's output and record equal the stored ones.

## Revisit if

- The first link's gate is not met in two sprints: then the run loop's needs are larger than the skeleton assumed,
  and the order is weighed again with what blocked it.
- After the decoder, fewer than half of the 1,186,512 record calls get words from the C emitter, or, after the case
  runner, the cases that can run at step 3 reach fewer than half the 254 rule files: then the translator's judges are
  too thin to port the rules in groups, and the judges come first.
- Eight translator sprints pass and `qpc` has not reached its stored output: the order is weighed again.
