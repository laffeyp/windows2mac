# Recentering, 2026-10-14

**History from 2026-10-16:** superseded by `docs/plan.md`, the one live direction document (full review `docs/review-2026-10-16-full.md`, item 6; BLACKBOARD Decisions, 2026-10-16 user).

## What w2m is

w2m runs a Windows x86-64 game on an Apple Silicon Mac with no part of Windows. It builds its own x86 compiler, its own
VM and its own Windows layer; no outside tool takes the place of any of those, except as a step toward replacing that
tool itself. Tools for what w2m does not replace come in as they are: DXMT, which turns Direct3D 11 into Metal, is one,
behind a seam w2m owns (the unix calls winemetal makes become w2m host requests; w2m's window owns the Metal layer).
It is two systems and one method:

- **A compiler** from x86-64 machine code to arm64: today `src/w2m/x86dec.py` (a decoder generated from
  `specs/x86_decode.json`), `src/w2m/lift.py` (instructions to w2m's ops), 254 rule modules in `src/w2m/rules/` (4,059
  lines, one per instruction class), and `compile_insn` and `_compile` in `src/w2m/vm_backend.py`.
- **An operating system kernel** with a Windows personality: the guest runtime, 48 C files and 8,787 lines in
  `tools/guest_runtime/`, running at EL1 inside a Hypervisor.framework VM; the host side in `src/w2m/vm_backend.py`
  (3,433 lines), `kernel.py`, `scheduler.py` and `runtime.py`; the monitor, `tools/vm_monitor/vm_monitor.c` (378 lines),
  which owns the vCPU.
- **A verification method** that proves both behave as Windows does, built from contracts mined from real Windows
  machines.

Both systems are solved disciplines with decades of standard forms. w2m builds its own versions of those forms. The
method is w2m's own and is what lets its versions be trusted.

The requirement is milestone P (docs/milestone-playable-slice.md): a person plays the game's first seconds on the Mac,
at the highest frame rate w2m can reach, with frames a second and seconds to the first scene reported on every run.

## Why recenter

The project grew its compiler and kernel run by run: the frontier ran the game until something stopped it, and the next
sprint fixed that. Correctness came out well. Speed and structure did not, because nothing asked what kind of system
was being built. The measurements of 2026-10-14 show where that left w2m:

- The laptop's game retires 325.4 billion instructions to the first scene (62 s) and 41 million a frame at 112 frames a
  second, spread over six busy threads, the busiest 28.9% of the whole 325.4 billion (laptop run
  lap-20261014T041748433Z-73c66e6a-01). w2m's own census covers a different stretch: its main thread runs 98.4% of the
  first 30 billion instructions of the batch-mode start (15cf3cdd). Several vCPUs therefore gain nothing on that
  start-up stretch and pay only in loading and play.
- w2m runs 255.7 million instructions a second, on one vCPU, on a quiet Mac (30 billion in 117.3 s): about 21 minutes
  to the first scene and about 6 frames a second.
- Of those 117.3 s, 64 s are inside the vCPU, where translated code runs about 590 million instructions a second. The
  rest is the host: every dispatch miss, time slice, thread switch, host request and model step leaves the VM and
  returns through Python, at about 14 µs a crossing against 0.72 µs for the exit itself (scripts/measure_exit.py).
- The ceiling each lever reaches, from these numbers: with every host crossing gone, one vCPU at 590 million a second
  runs a 41-million-instruction frame at about 14 frames a second; spread over vCPUs, the busiest thread's 11.9 million
  a frame sets the pace at about 50. Past 50, only the speed of the translated code counts.
- The frontier takes about 2 minutes per 30 billion instructions, and a census needs several alternating runs on this
  Mac because its load moves timings by up to two times. A method that discovers the architecture one stop at a time
  cannot go faster than that loop.

The pieces to fix this are already present; they sit in the wrong places. The compiler translates one block at a time
in Python as the game runs. The kernel's scheduler and dispatcher live in a separate supervisor (Python), so the game
leaves the VM for them. One vCPU carries all 28 of the game's threads.

## What is working: keep

These judge behavior, not code, so they carry over whole to any rebuild, and they are what make a rebuild safe.

1. **Contracts mined from Windows.** 68 probe families with exact rows from runner and laptop runs (72 contracts under
   `contracts/`), each run in w2m and compared row for row. All 68 conform. They found real defects that no review
   would have: FLS shared by every thread, empty time-zone names, a wrong EnumDisplayMonitors answer.
2. **Class runs.** Each instruction class judged on an Intel Xeon's recorded results (`scripts/class_run.py`): 456 of
   458 accepted, FSIN and FCOS pinned open in the last bit of 18 cases. On 2026-10-14 they caught a PMADDUBSW
   lowering bug at once, and planted faults in three new lowerings.
3. **The game's call stream against Windows.** The frontier's calls compared with a set of reference executions
   (`scripts/call_compare.py`): one runner run and ten 4-processor laptop runs, values judged strictly, scheduling
   and Windows-build differences named in the contract with their evidence. The frontier conforms against all eleven.
4. **Plants for every check.** A check counts only once a planted fault fails it. On 2026-10-14 plants exposed two
   blind checks: the threads probes held no vector across a switch, and code_rewrite reached rewritten code only by
   indirect calls.
5. **The Windows reference machine.** The laptop runs probes, game runs and counted play runs on demand
   (`scripts/windows_machine.py`), including instructions per thread from the CPU's counters.
6. **The automation around them.** Sweeps of every class and family, records from committed trees, CI on each closing
   commit, the laptop's jobs. Almost every step of the loop runs without a person.

## What is not working: change

1. **Building the architecture by the frontier.** It is a strong idea only when a run is very fast, or when it is aimed
   at one stretch of the game (one function family, one loop). As the general way to decide the compiler's and the
   kernel's structure, at today's speed, it is too slow and it never names the system it is building. From here the
   frontier is a check, run where it is fast or targeted, and the structure comes from the standard forms below.
2. **Python in the running game's path.** Python stays w2m's language for building, recording, comparing and
   orchestrating. It leaves the per-exit loop, the translator's run-time path, and the scheduler.
3. **Speed one lever at a time from the census alone.** On 2026-10-14 block chaining gained nothing (17.0 s against 17.1
   s inside the vCPU per 10 billion instructions, a78a90a0), and time slices only while another thread is ready gained
   nothing because `wake_all` wakes every waiter on any signal (15cf3cdd), a defect in the wake path rather than proof
   against the lever. Each speed change is now chosen from the parts' standard forms and the ceilings above, and
   measured after.
4. **Identity scopes too wide for the sweeps.** A family's identity takes whole modules such as `vm_backend.py`, so
   any edit there re-runs all 68 families and 458 classes, 10 to 15 minutes each time. The method stays; the scope
   narrows to what each check actually reaches.

## The standard forms w2m moves to

Each part is classified, and moves to the next form its discipline already knows. Every form is built in w2m, by w2m.

**The compiler** (x86-64 to arm64):
- The game's code is fixed PE images held in full, and mono's methods appear at run time. The images can compile
  ahead of time into a cache; how much that gives depends on the share of play time spent in the PE images against
  mono's run-time code (Unity's game logic is C# that mono compiles), which is measured before the cache is sized.
- Translation through an intermediate form with passes across instructions: flag liveness (a flag no later
  instruction reads is not computed), register allocation across a block and its successors, dead-code removal, and
  block linking inside a compiled region.
- The rules stay the single source of each instruction's meaning. A generator emits the compiler's C from the same
  rules, so the class runs judge the generated compiler exactly as they judge today's.

**The kernel** (the Windows personality):
- The guest runtime becomes the kernel: the dispatcher, the scheduler, waits and signals, and the run-time compiler
  for mono live inside the VM, so the game leaves the VM only for devices.
- Several vCPUs, scheduled M:N as Decision N1 (2026-10-09) set: guest threads run on a pool of vCPUs, a waiting thread
  holds none, and the pool grows to the number of busy threads (six on the laptop's count), capped at the Mac's
  cores. The guest scheduler is built for several vCPUs from its first version: per-vCPU run queues, locks on
  acquire/release atomics, a cross-vCPU wakeup through Hypervisor.framework's GICv3 (`hv_gic_create`, macOS 15, created
  after the VM and before any vCPU), run first on one vCPU. Apple's TSO mode is set by the guest on each vCPU, and the
  x86 litmus tests (MP, SB, LB, 2+2W, IRIW) through the compiler are the memory-model check.
- One thread is AppKit's: the process's main thread runs the window and its events, and the monitor's drive loop and
  vCPU threads run beside it.
- The monitor becomes a thin device layer in C: files, clock, window, input, and the graphics seam.

**The verification method** stays as it is, over the new forms: families, class runs, the call stream, plants, the
laptop. Its sweeps run against narrowed identities.

## Next steps

In order, each one measured against the requirement and checked by the method before the next begins. Every run
reports a frame-equivalent rate (instructions a second over the laptop's 41 million a frame) and seconds to the first
scene, over the loading and play stretch, until real frames exist.

1. **This document, read and decided.** The plan (`windows_to_macos_research_direction_v0_2.md`) is not edited; a
   recentered direction lands as a new version if the user decides so.
2. **The channel, landed.** The shared-memory channel between host and monitor is built and passes the VM tests and the
   seven threading and exception families (uncommitted on e626d1c2). All 68 families and the 458 classes run on it, it
   is committed, and then measured against the pipe, the crossing time included, since later steps are ranked on it.
3. **Two measures of where time goes, beside step 2:**
   - inside the vCPU, a census of arm64 instructions run per x86 instruction by instruction class, weighted by how
     often each block runs, which ranks the compiler's passes;
   - over the play stretch, the share of instructions in the PE images against mono's run-time code, which sizes the
     ahead-of-time cache.
4. **Narrow the identity scopes,** so a change to one part re-runs the checks that reach it and no others.
5. **The kernel inside the VM, first piece:** the scheduler, thread switches and time slices in the guest runtime,
   built for several vCPUs and run on one; the wake path wakes one waiter where Windows does (an auto-reset event), and
   time slices only while another thread is ready are measured again. The VM setup creates the GICv3. The first
   compiler pass the step 3 census ranks (flag liveness is the likely one) goes in alongside.
6. **The dispatcher and the run-time compiler inside the VM,** generated from the rules.
7. **Several vCPUs** (M:N, Decision N1), gated by the litmus tests, measured over the loading and play stretch.
8. **The compiler's remaining passes and the ahead-of-time cache,** sized by step 3.
9. **The clock as a run option:** the wall clock for play, the instruction clock for the conforming comparison, each
   run recording which it used.
10. **The frontier as a targeted check:** the snapshot at the end of the conforming batch prefix, so a run starts in the
    loading and play stretch where steps 5-8 pay.

Alongside, since it needs no new kernel: the DXMT spike. DXMT's PE DLLs' imports placed among w2m's served functions,
Wine's unix-call path or missing (done for v0.80: 69 of 200 served, 131 missing, mostly the C runtime), then the
winemetal calls the game's Direct3D 11 calls reach, and crossings a frame against the channel's crossing time.

Graphics, the window (on AppKit's main thread) and input follow as milestone P orders them, on the new forms.

## Open

- Whether the recentered direction becomes `windows_to_macos_research_direction_v0_3.md`.
- Sprint 068's card holds the speed work done under the old method; it closes with its measured results (B and C kept,
  D and K reverted) once the channel lands.
