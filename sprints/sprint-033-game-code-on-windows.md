# Sprint 033 — Hail to the Rainbow on the Windows runner: native code and Mono's generated code

```yaml
---
id: 033
status: open
phase: 3
pass_kind: functional
---
```

## scope

Hail to the Rainbow (Steam app 1699230, build 24247624; Decision 2026-10-09) is the x86-64 profile. Its game logic is .NET code that Mono compiles to x86-64 while the game runs, so the code the game executes is the seven native PE files plus what Mono writes. This sprint gets both onto the record:

1. The installed game goes to the w2m repo (private) as release `game-1699230-24247624`: a zstd-compressed tar split into parts under 2 GiB. `games/hail-to-the-rainbow/files.json` (committed) lists every installed file with its size and sha256; `games/hail-to-the-rainbow/release.json` lists each part's sha256.
2. `tools/jit_capture/jit_capture.c`: a launcher that loads the game's own `MonoBleedingEdge/EmbedRuntime/mono-2.0-bdwgc.dll`, registers a JIT-done hook through its exports `mono_profiler_create` and `mono_profiler_set_jit_done_callback` (allowed at any time: Unity's Mono, branch `unity-2019.4-mbe`, `mono/metadata/profiler.c`), then calls `UnityPlayer.dll`'s `UnityMain`, as `Hail to the Rainbow.exe` does. The hook writes each compiled method's full name, code address, and code bytes (`mono_jit_info_get_code_start`, `mono_jit_info_get_code_size`) to a capture file.
3. `.github/workflows/windows-game.yml`: on `windows-2025`, download and verify the parts, unpack, check every file against `files.json`, build the launcher, run the game with `-batchmode -nographics -logFile` (Unity 2019.4 manual, Player command-line arguments) for a fixed time, and upload the capture and the Unity log.
4. On the Mac: the decoder differential over the seven native files and over every captured method body; decoder exit criterion (d) is measured against the rows both use.

## prerequisites

- 032; the depot at `~/w2m-depots/hail-to-the-rainbow` (fetched with Valve's SteamCMD, 2026-10-09)

## context_files

- BLACKBOARD Decisions 2026-10-09 (profile; 32-bit scope); WORKING_AGREEMENT decoder exit criterion (d)
- Unity-Technologies/mono `unity-2019.4-mbe`: `mono/metadata/profiler.c`, `mono/metadata/profiler-events.h` (`jit_done`), `mono/mini/mini-runtime.c` (`--jitmap` compiled under `ENABLE_JIT_MAP`; its format string is absent from this build's DLL)
- docs.unity3d.com 2019.4 PlayerCommandLineArguments (`-batchmode`, `-nographics`)
- docs.github.com: release assets under 2 GiB each

## signal contract

### Emits

None on the guest plane: the capture is input to the decoder differential, not a trace judged by a contract. Harness plane as Sprint 025's decoder differential where it runs.

### Invariants

- No vocabulary change.
- Every unpacked file on the runner matches `files.json` by sha256 before the game starts.

## artifact contract

### Files created

- `sprints/sprint-033-game-code-on-windows.md`, `games/hail-to-the-rainbow/files.json`, `games/hail-to-the-rainbow/release.json`, `scripts/pack_game.py`, `tools/jit_capture/jit_capture.c`, `.github/workflows/windows-game.yml`, `scripts/jit_capture_read.py`

Hard rule 6 exception: packing, launcher, workflow and reader are one path from the depot to the decoder; none is checkable alone.

### Command exit codes

- `scripts/gates.sh` → 0

## observation contract

### Expected runtime signals

- Release `game-1699230-24247624` holds parts whose sha256 values equal `release.json`; the runner's unpacked tree equals `files.json` (298 files, 15,584,319,636 bytes).
- The game starts in batch mode on the runner, Mono compiles methods, and the capture holds at least one method of `Assembly-CSharp` (the game's own code). If the game exits early (Steam not running, missing GPU path), the Unity log says why; that is recorded as observed, not hidden.
- Decoder differential over native files and captured code: whatever the record shows; every spec gap and XED disagreement is listed.

## done criteria

The game's native code and the code Mono generated for it on Windows are both on the record and both run through the decoder differential.

## self-review checklist

- [x] Scope concrete. [x] Sources named. [x] No new vocabulary. [x] Observation declared before running. [ ] ≤2 files: exception recorded.

## observed
