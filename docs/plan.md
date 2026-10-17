# w2m roadmap

This file is the plan's one home and the one live direction document (Decision 2026-10-09; full review
`docs/review-2026-10-16-full.md`, item 6, applied by the user's decision of 2026-10-16). It is edited in place, with
history in git; `BLACKBOARD.md` Decisions record why it changed. `docs/milestone-playable-slice.md`,
`docs/recentering-2026-10-14.md`, `docs/recentering-2026-10-16-knowable-first.md` and `docs/solution-2026-10-16.md`
are history from 2026-10-16 on. Phase tables for Sprints 038 to 071 are in git (this file before 2026-10-16's
rewrite) and in the sprint cards. Every sprint states what it delivers and how it is judged: a bound, an expected
value, or a planted fault that must be caught (WORKING_AGREEMENT, observation contracts).

**Goal:** Hail to the Rainbow (Steam app 1699230, build 24247624; x86-64; Unity 2019.4 with Mono) is playable on this
Mac, at speed; w2m runs any game (user, 2026-10-16). Milestones:

- **A — batch-mode parity:** the game runs under w2m with `-batchmode -nographics` and writes the same `unity.log` the
  Windows runner wrote (168 lines, Sprint 033).
- **B — first frame:** the game draws its first frame through Metal.
- **C — playable:** input, audio, video and Steam work, at speed. The review marks the old bound ("frames under
  16.7 ms") as stale; the paste does not carry its replacement (BLACKBOARD Open questions).

**The number that decides it** (review, from E3 on the laptop): 314 NT system calls and 18 win32k calls a frame at
113 frames a second, about 37,600 a second. The three most common are scheduler work: NtDelayExecution (114 a frame;
the paste cut the name, E3's table gives it), NtWaitForSingleObject (100) and NtReleaseSemaphore (64). At about 14 µs a crossing to the
host, those calls alone would take 0.53 s of every second of real time. The in-VM scheduler on several vCPUs is the
speed gate.

## Do next, in this order

Measured by instructions a second and seconds to the first scene (the paste cuts the heading after "in this order").

1. **Measure the whole run now.** Hail to the Rainbow's batch run on today's tree: instructions, seconds, and the
   fps-equivalent (the rate over 41 million). The last whole-run figure is Sprint 070's 62.42 s warm.
2. **The scheduler, waits and NtDelayExecution inside the VM**, the wake path waking one waiter; per-vCPU queues and
   the GICv3 made before any vCPU. Judges: the 7 threading families and E3's three top calls.
3. **Several vCPUs under M:N.** Gated by the litmus tests (MP, SB, LB, 2+2W and IRIW). The ceiling rises from about
   14 to about 50 (the review's figures; the paste cuts what they count).
4. **Two compiler passes:** flag liveness across blocks, then register allocation over a region.
5. **The ahead-of-time cache** (the review: the cached set retires 76.9% of play's instructions).
6. **The window on AppKit behind the seam: the first frame.** Input follows, using exactly the call list the laptop's
   play runs made. Route: DXMT's d3d11.dll and dxgi.dll loaded in place of Microsoft's, reaching Metal; Microsoft's DLLs
   serve their 200 imports. A UMD of w2m's own comes after, if ever (review item 8; it bends the 2026-10-16 decision
   at this one DLL pair).
7. **Microsoft's DLLs over the w2m kernel**, in parallel and off the critical path. First E1's start-up list (342
   calls of 62 kinds before main) and csrss's `\Windows\ApiPort`; then the families, moved one at a time. The game
   stays on the service path until the new path conforms.

Steps 2 to 5 are where the speed comes from; step 6 is where the picture comes from.

## Cut

- **The equal-words rule** (every compiler in C emits the same arm64 words as Python's `compile_insn`). It made the
  port safe; the review names what it left out: no register allocation across instructions, no flag liveness across
  blocks, nothing faster than Python's output. Behavior is judged by the Xeon class runs (456 of 458 accepted) and the 68 families; passes may change the
  words.
- **Symbolic stencil generation.** Sprint 071 measured 1,287 distinct encoder sequences over every compiled
  instruction; Sprint 090's symbolic run made 363,546 stencils from 1,000 instructions in 441 s. A block without a
  stencil goes to the host once and is cached. The captured stencils stay.
- **Static reach as an ordering tool.** Sprint 093 runs once, to size the work against the traces; its defect-class
  fixes (forwarders, ordinal-only exports, the API set schema) stay, because the loader needs them.
- **The windows-function census** (Sprints 085 to 088): Microsoft's DLLs serve every export, so its coverage figure
  measures nothing. The part the compiler needs (from 087) stays.
- **Bookkeeping as separate commits.** Each closure, tail and record stamp goes into its sprint's commit, with CI on
  that commit.
- **Five live direction documents.** This file is the one; the other four are history.
- **The organism and research-paper line as work.** Only the snapshot stays, as a speed tool: it removes each
  measured change's 300 s rerun. Model learning and bandits stop.
- **A UMD of w2m's own as the route to the first frame** (see step 6).

## Keep

- The judges: the families, the class runs against the Xeon, the call-stream comparisons, planted faults, and the
  laptop as oracle.
- The VM: Hypervisor.framework at EL1, with the runtime inside it.
- The compiler inside the guest (87,887 blocks compiled in the guest).
- Microsoft's DLLs over the service path, kept as the game's path, its record and its fallback until the new path
  passes the 68 families and a milestone (the paste cuts which).
- The laptop's DTrace setup: 486 NT probes, and win32u through the pid provider.

## After the order above

- 32-bit: BioShock 2 Remastered (depot on disk): a second translator mode, the TEB through FS, x87, stdcall and
  thiscall.
- Further profiles: The Blood of Dawnwalker (D3D12/DXIL) after the user's purchase; then by engine and API.

## Standing items

- Comparison subjects, recorded at each phase boundary: Wine and CrossOver (D3DMetal, DXMT, CrossOver's macOS FEX
  port), Rosetta's post-macOS-27 games subset, Prism, box64, FEX.
- Every probe run records its CPU; a verdict names the CPU it came from.
- Gates on every commit include record replay.
