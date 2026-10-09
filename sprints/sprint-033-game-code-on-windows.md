# Sprint 033 — Hail to the Rainbow on the Windows runner: native code and Mono's generated code

```yaml
---
id: 033
status: closed
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

- [x] Scope concrete. [x] Sources named. [x] No new vocabulary. [x] Observation declared before running. [x] File count recorded (hard rule 6 override, Decision 2026-10-09).
- [ ] Two items differ from the card as written: the observation contract said 298 files (the depot is 297; the 298th is SteamCMD's manifest), and the reader was replaced by `scripts/game_code_differential.py` after review 2026-10-09 H5. The card also absorbed the review's fixes and the runtime architecture Decision. Recorded in observed.

## observed
- Packing: 297 files, 15,584,319,636 bytes (equal to Steam's SizeOnDisk; SteamCMD's `steamapps/appmanifest_1699230.acf` left out), 5 parts, 8,098,738,888 bytes; release `game-1699230-24247624` on laffeyp/w2m (private); GitHub's sha256 for each part equals `release.json`.
- Native files, decoder differential on the Mac (`scripts/decoder_differential.py` over the 7 PE files, 67 s): 5,801,517 instructions on Capstone's recursive traversal; XED decodes all; Capstone's length and ours equal XED's at every position; our iclass differs from XED's at 25 (`lock cmpxchg16b`: ours `CMPXCHG16B`, XED `CMPXCHG16B_LOCK`, bytes f0490fc708). Spec gaps: 8,861 positions in 124 kinds, led by SSE packed single (0F C2 `cmpps` 4,808; 0F 50 `movmskps` 1,442; 0F 52 `rsqrtps` 432; 0F 53 `rcpps` 423; 0F 2B `movntps` 357), MMX (`movq` 0F 6F/7F, `padds*`, `psubsb`, `paddw`), VEX and FMA (`vmulps`, `vaddps`, `vsubps`, `vorpd`, `vpermilps`, `vfmaddps`, `vfmadd231ps`, `vfnmadd231sd`), `cvttpd2dq`, and `int1` (F1). The other 264 disagreements with Capstone are prefix spelling at equal length (`repz ret` 197, `bnd j*`/`bnd ret` 42, `lock cmpxchg16b` 25). Decoder exit criterion (b), spec gaps 0, held on the earlier corpus and does not hold on this game.
- Runner run 37977392678: parts verified, 297 files verified against `files.json` on D: (118 GB free; C: 32.6 GB free); the launcher build failed before clang ran: PowerShell read the comma in `-Wl,/subsystem:windows` as an array separator. Fixed by quoting; the build now runs first. Run 37979176570: the launcher compiles under `-Wall -Werror`.
- Runner run 37979176570 (format 1 capture): unpacked tree verified; the game started in batch mode, loaded its menu scenes (Unity log: "Resolution Set - 1920 X 1080", "New_Input - DONE", 741 then 3,162 unused assets unloaded) and was stopped at 181 s (`exited_by_itself` false). CPU: Intel Xeon Platinum 8370C. Capture: 2,901,740 bytes, 6,671 method records, 4,500 distinct code starts (2,171 repeats), 0 torn bytes; the game's own methods are present (`Intro_Script:Open_MenuLevel ()`, `MenuScriptMain`, `MenuButton_new:.ctor ()`).
- Review `docs/review-2026-10-09-project.md` verified by command before any change. Reproduced: the execution frontier (13 instructions, `Unimplemented: import KERNEL32.dll!GetSystemTimeAsFileTime`); coverage shares 93.5% / 96.8% / 80.9% and UnityPlayer's top unliftable classes (MOVAPS 207,567, MOVSS 128,992, MOVUPS 58,131, MULSS 57,389); 8,713 decoder refusals in UnityPlayer; 634 imported functions (kernel32 294, user32 120, ws2_32 51, advapi32 39, winmm 26); UnityPlayer 490 functions from 18 DLLs, none from d3d11, dxgi or ntdll, and `d3d11.dll`, `dxgi.dll`, `D3D11CreateDevice`, `CreateDXGIFactory1` present as strings; 25 of 61 tags with an emit site; the moffs `IndexError`; `run_skeleton.py:44`; the stale `WORKING_AGREEMENT.md` lines; 298 = 297 + 1 and 122 assemblies; the six uncommitted candidate runs; the Mono source claims (`AddVectoredExceptionHandler(1, ...)` in `exceptions-amd64.c`, `_resetstkoflw`, `gs:[0x1480 + key*8]` and `gs:[0x1780]`, `PAGE_EXECUTE_READWRITE`, `RtlAddGrowableFunctionTable` by `GetProcAddress`, `SuspendThread`/`GetThreadContext`/`SetThreadContext`); bdwgc `copy_ptr_regs` scanning XMM; box64 enabling strong ordering on `libmonobdwgc-2.0.so`; GitHub's documented 14 GB. Differs: the native files import from 22 DLLs, not 21; class counts are 123 / 209 / 373 (the reviewer's figures), with 35 / 47 / 49 lifter-handled classes counting each Jcc and SETcc separately; the .exe has 8 decoder refusals. L1 is contradicted by measurement: the runner had 117,972,156,416 bytes free on D:.
- Part 2 (commit after d7a6f36): capture format 2 (image name per method, `jit_code_buffer` regions, CPUID leaves, checked writes with an `.error` file); `src/w2m/jit_capture.py` (both formats, duplicates removed by code_start) with tests; `scripts/game_code_differential.py` replaces `scripts/jit_capture_read.py` (decodes each position once, lets XED decide where w2m's decoder refuses inside a region, measures criterion (d) by row, records on the signal plane); `Decoded.row` carries the spec row's sha256; spec 0.6.3 adds cmpxchg8b/16b to `prefixes.lockable` (SDM LOCK page) so LOCK CMPXCHG16B decodes as CMPXCHG16B_LOCK; confirmations regenerated: 549 of 809 rows. The lifter raises `LiftError` for a row without an operand template. The workflow checks free disk (26 GiB) and that no unlisted file is present. Gates: 207 passed.
- Game code differential on the format 1 capture (spec 0.6.3): 6,180,403 positions (native 5,801,517; Mono-generated 378,886 after duplicates); agree 6,171,468; length or iclass differences 0; no row 8,935 in 123 kinds (74 of them only in Mono's code). Criterion (d): 577 rows used, 142 unconfirmed, covering 566,888 positions (movdqa, pxor, movd, psrld, pand lead). Region ends: 4,224 exact, 156 at the region's last bytes, 120 where XED refuses (data after code).
- Runner run 37982147515 failed in the new disk check: Windows Python cannot open `/d/` written inside a `-c` string; Git Bash converts only standalone arguments. Fixed by passing the path as an argument.
- Runner run 37982448071 (format 2 capture): parts and 297 files verified, no unlisted file, free-disk check passed; the game ran 190 s in batch mode and was stopped. CPU this time: AMD EPYC 7763 (CPUID: SSE2 through SSE4.2, POPCNT, AVX, FMA, AVX2, BMI1, BMI2; no AVX-512F). The previous run had an Intel Xeon Platinum 8370C: Mono's JIT selects instructions by CPU feature, so each capture is evidence for its CPU only. Capture: 5,474,576 bytes (sha256 c74fe8bf…8db1, kept as an asset of the private release, hash in `store/game-runs/37982448071/`), 16,295 records, 9,624 regions after duplicates: 4,500 methods, 4,840 specific trampolines, 124 generics, 115 IMT, 17 unbox trampolines, 10 helpers, 9 exception-handling and 9 delegate-invoke buffers; 0 torn bytes, no `.error` file. Method images: mscorlib 1,522, UnityEngine.CoreModule 824, Assembly-CSharp 564 (the game's own code), System.Xml 279, Unity.Timeline 274.
- Two reader defects found on the first format 2 run and fixed: (1) every method showed an empty image, because Mono reports each method first as a METHOD code buffer (no image) and duplicate removal kept the first; jit_done records now win. (2) IN, OUT and INT1 "gaps" were data: a specific trampoline is a call followed by a length byte and an inline argument (tramp-amd64.c: "The trampoline code will obtain the argument from the instruction stream"); its code now ends at the call.
- Game code differential, final (spec 0.6.3, record `store/runs/gamediff-1791576553479425000`: SESSION_INIT 1, COMPARISON_COMPLETED 1 verdict discrepancy, DISCREPANCY_REPORTED 125, DISCREPANCY_ROUTED 125 contract_revision, SESSION_COMPLETED 1): 6,192,478 positions (native 5,801,517; Mono-generated 390,961); agree 6,183,516; length differs 0; iclass differs 0; XED refuses 0; no row 8,962 in 125 kinds (101 positions only in Mono's code). Region ends: 4,451 exact, 4,840 at the trampoline call, 222 at the region's last bytes, 111 where XED refuses. Criterion (d): 579 rows used, 438 confirmed, 141 unconfirmed; the largest, `lea` (row 134), covers 483,495 positions.
- Why rows stay unconfirmed: 233 of the 260 unconfirmed rows fail only on `ours_accepts` (w2m decodes generated bytes XED refuses). For lea, movdqa and pxor the refused cases are two rules the spec lacks: LEA with a register operand (mod=3; the SDM gives #UD) and a 66-prefixed SSE opcode that also carries F2 or F3 (F2/F3 take precedence as the mandatory prefix; `_pick` falls back to the 66 row instead of refusing). These are the first work of the decoder sprint.
- Runtime architecture decided (BLACKBOARD 2026-10-09) from the Phase 2 measurements; the sequence that followed the review is revised to match.
- Gates: `scripts/gates.sh` 0.

