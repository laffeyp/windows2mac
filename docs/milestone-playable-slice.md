# Milestone P: a person plays the first seconds on the Mac

Status: proposal, 2026-10-13, for the user's decision. It changes the method, not only the next sprint.

## Why the method changes

Each sprint so far makes one more Windows function family, or one more stretch of the game's start, conform with
Windows. That work is sound and it has a cost the plan never priced: nothing in it brings a playable game nearer.
Speed and graphics, the two things that decide whether a person can play, sit at the end of the plan (Phase 6, the
hot path as plan row 071). w2m today runs the game's start headless through 3,451 calls with no defect against
Windows (Sprint 066), at about 39 million x86 instructions a second, and has drawn nothing.

The new method is a thin vertical slice, judged by measured numbers:

1. **One milestone ahead of the rest, P:** the game's window opens on the Mac, the first scene draws, and the
   keyboard and mouse move the camera, at a frame rate stated below. Work the slice does not need waits.
2. **Speed is a gate, chosen by profiling.** Every sprint reports two numbers, seconds to the first scene and
   instructions a second, and fixes the largest measured cost first.
3. **Graphics are adopted, not built** (below).
4. **Correctness stays, scoped to the slice.** The families, the laptop's references and the call comparison check
   what the slice runs; under Decision 2026-10-13 (user) they chase values, not one run's timing.

## What P means, measurably

- The game starts from w2m's command on the Mac, in a window, with no Windows machine involved.
- The first scene (the laptop's play-mode frame at about 62 s: a stairwell, the prompt "Tab - RUN IIS") appears
  in that window; a person judges it the same scene as the laptop's frame.
- W, A, S, D and the mouse move the camera; the laptop's input script `round-trip` replayed on the Mac moves it
  as on Windows.
- Frames a second at 1280x720 once the scene is up, and seconds to the first scene, reported on every run. No
  threshold closes the speed work: the bar is the highest frame rate w2m can reach, and each change is judged by what
  it gains (Decision 2026-10-14, user; it replaces this list's "at least 20 frames a second" and "under 5 minutes").
- The numbers beside them: the laptop's play-mode frame rate (RTX 3050, 1280x720; first scene at about 62 s) and the
  game under CrossOver 26 on this Mac (Wine 11.0, D3DMetal 3.0, DXMT 0.72; released 2026-02-10).
- The native workload: the instructions the game retires per frame and to the first scene on the laptop, per thread,
  against w2m's rate (139 million a second on the wall clock, 223 million inside the vCPU, frontier 1ecc22b2). It
  bounds what each lever can give and ranks the work.

## Speed

*To be filled from the time census (commit d91a9e21) of the frontier run in progress: time inside the vCPU against
host time by exit kind, the share of the translator in Python, and the share of the scheduler.* Known before it:
Sprint 064 reached 442 million instructions a second through compute-bound stretches, and whole runs average about
39 million, so most of a run's time is not translated code. The candidate fixes, each to be ranked by the census:
block chaining (every block exit goes through the dispatcher), flags computed only where read, the translator out of
Python, the text pipe replaced by shared memory, several vCPUs.

## Graphics: DXMT behind a w2m seam

DXMT (github.com/3Shain/dxmt, "a Metal-based translation layer for Direct3D 11 and 10", active: last push
2026-10-13) builds as two halves: Windows PE DLLs (d3d11.dll, dxgi.dll, built with mingw) and one native macOS
library, winemetal, that drives Metal. The DLLs reach winemetal through Wine's unix calls; its developers also
document a native arm64 build of the library. That split fits w2m:

- DXMT's d3d11.dll and dxgi.dll load as ordinary x86-64 PE DLLs and run translated, as UnityPlayer does.
- A unix call becomes a w2m host request; the host answers it with winemetal built for arm64.
- The seam is narrow and explicit: winemetal_thunks.h declares 64 parameter structures, and every pointer that
  crosses it is typed `WMTMemoryPointer`, so the host knows which values are guest addresses to translate.
- Presentation goes through a CAMetalLayer in a Mac window that w2m's window functions own.

Open: whether guest memory can be mapped in the host at the guest's own addresses (no translation at all), the
cost of each crossing against the frame's call count, and DXMT's coverage of the calls the game's play run makes
(game run 38113343972).

## Order

Amended 2026-10-14 (Decision, user): the review "Phase P: actionable review, 2026-10-14". Speed work runs alongside
every step and stops at no number; each item is measured on census runs that have the Mac to themselves.

1. Measures in place: frames a second and seconds to the first scene on every run; the laptop's instructions per
   frame; census runs alone; a run option for each clock (wall clock for play, the instruction clock for the
   conforming comparison), recorded per run; the snapshot at the end of the conforming batch prefix, so a measured
   item does not wait on a 300 s rerun.
2. Speed inside one vCPU (Sprint 068): the census's largest costs, a spin count (PAUSE and instructions per thread),
   block chaining, flag liveness within a block, the instruction counter off block entries.
3. Several vCPUs: M:N scheduling of the guest's threads across vCPUs, guest TSO set on each (ACTLR_EL1.TSOEN), the
   x86 litmus tests (MP, SB, LB, 2+2W, IRIW) through the translator as the gate, frames a second at 1, 2, 4 and 8.
4. The shared-memory transport, finished before DXMT, measured by a stub host call's crossing against the Direct3D
   calls per frame in play run 38113343972.
5. The drive loop off the process's main thread, which AppKit needs for windows and events.
6. A window and a message loop on the Mac that the game drives (plan Phase 6's first step).
7. DXMT behind the seam: the first frame. A spike runs beside the speed work: DXMT's two PE DLLs' imports sorted into
   w2m's 297 served functions, Wine's ntdll (__wine_unix_call) or missing, and the winemetal calls the game's
   Direct3D 11 calls reach.
8. Input: exactly the calls the laptop's play runs (e61fca06) make, from the Mac's keyboard and mouse.
