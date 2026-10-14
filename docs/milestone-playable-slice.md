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
- At least 20 frames a second at 1280x720 once the scene is up (a number to confirm with the user).
- Time to the first scene under 5 minutes (the laptop: about 60 s).

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

1. The census, then the largest speed cost (this sprint).
2. A window and a message loop on the Mac that the game drives (plan Phase 6's first step).
3. DXMT behind the seam: the first frame.
4. Input from the Mac's keyboard and mouse.
5. Speed until P's frame rate holds.
