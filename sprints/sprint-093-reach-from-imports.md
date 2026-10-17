# Sprint 093 — E2: what the games' imports reach, through Microsoft's DLLs, in system calls and information classes

```yaml
---
id: 093
status: closed
phase: 5
pass_kind: functional
---
```

## scope

Step 1 of the solution (docs/solution-2026-10-16.md, section 6), its third recording, E2 (4.2): a computation over
the copied `C:\Windows` (091), no run. For each import the census lists, a static walk through Microsoft's DLLs to the
system calls it can reach, with the information class at each call site where it is a constant.

**Known first (searched before this card).**

- The method is published: NTFUZZ (Choi et al., IEEE S&P 2021; code SoftSec-KAIST/NTFuzz, MIT) walks ntdll,
  kernel32, kernelbase, win32u, gdi32, gdi32full, user32 and dxcore bottom-up, function by function, from the
  system-call stubs back to documented APIs, and keeps each argument as a constant or a linear form of the
  function's own parameters. Its limits, stated in the paper: 32-bit Windows 10 17134 only; indirect calls and
  recursion ignored as no-ops. E2 takes the method, walks 64-bit and 32-bit, and counts every indirect call site it
  does not resolve rather than dropping it.
- The system calls' prototypes and their information-class enumerations and structures are published in System
  Informer's phnt headers (ntdoc.m417z.com renders them); E2 names classes from them.
- The repository already holds: `src/w2m/x86ref.py` (Capstone traversal, 32- and 64-bit, calls through the IAT
  resolved to `dll!name`, indirect calls counted); `src/w2m/pe.py`; the census's imports, P/Invokes, run-time lookup
  candidates and COM GUIDs per game (`store/census/static`).

**Defect classes predicted and measured before the walk** (on the copied build 26200.7840):

- *Forwarded exports.* pe.py reads a forwarder (an export whose RVA lies in the export directory, naming another
  DLL's function) as code. kernel32 forwards 211 of 1,692 named exports, gdi32 294 of 994 (the D3DKMT* calls, the
  frame path E3 measured), advapi32 96, kernelbase 104. The walk follows the forwarder's name.
- *Exports by ordinal only.* pe.py lists named exports; user32 has 272 ordinal-only slots, gdi32 14. An import by
  ordinal is resolved by ordinal.
- *API sets.* `src/w2m/modules.py` maps API sets with a five-entry table written by hand; Windows maps them with
  `apisetschema.dll`'s schema, which is in the copy. The walk reads the schema (format published by Geoff Chappell and
  Quarkslab, "Runtime DLL name resolution: ApiSetSchema").
- *Stubs outside the exports.* ntdll holds 494 stub sites for 488 numbers: six unexported copies (the numbers of
  ZwClose, ZwQueryVirtualMemory, ZwMapViewOfSection, ZwUnmapViewOfSection, ZwOpenFile, ZwCreateSection). Stubs are
  found by their bytes, not their names.
- *Stubs outside ntdll and win32u.* Of System32's 3,786 images, six hold stubs: ntdll, win32u, wow64win and wow64con
  (1,486 each), tprtdll (488), vertdll (24). A 32-bit game's win32k calls leave through SysWOW64's win32u and
  wow64win, not System32's win32u.
- *32-bit stubs have their own shapes.* SysWOW64's ntdll: 1,016 of `mov eax, N; mov edx, <transition>; call edx;
  ret`, and 2 of another (NtQueryInformationProcess among them: `call $+5; pop edx; ...`); its win32u: 1,484 of the
  first shape and 23 exports that call a shared stub. Each shape is matched and counted; an Nt export matching no
  shape is listed.
- *Exports that are data.* ntdll exports three variables (NlsMbCodePageTag, NlsMbOemCodePageTag,
  RtlpFreezeTimeBias); the walk does not decode them.
- *Tail calls and import jumps.* kernel32's non-forwarded exports are often a `jmp [IAT]` to kernelbase; a `jmp` to an
  import or another function is an edge, not an end.

## the walk

- **Graph.** For every image of `System32` (64-bit) and `SysWOW64` (32-bit) a game can load: functions (from .pdata,
  exports, call targets), and per function its direct callees, the imports it calls or jumps to, the stub number if it
  is a stub, its unresolved indirect call sites, and, at each call to a stub or to a function that passes a parameter
  on to a stub's class argument, that argument as a constant or as "parameter k" of the caller (NTFUZZ's linear form,
  restricted to a constant or one parameter). Edges across images go through the API set schema and forwarders.
- **Roots, per game.** Its images' imports by name and ordinal (the census); its P/Invokes (Hail to the Rainbow,
  1,102); the start every process makes (ntdll's `LdrInitializeThunk` and the three `KiUser*` dispatchers). The
  run-time lookup candidates (504) and COM classes (126 GUIDs, their in-process servers named in the copied SOFTWARE
  hive) are walked as a second set and reported apart, as candidates.
- **Output.** `store/census/reach/<game>.json`: per system call (ntdll and win32k, by name and number) the roots that
  reach it and the classes seen; the ALPC port names and RPC interface UUIDs in the reached DLLs' data; the count of
  unresolved indirect sites within reach.

## prerequisites

- 091 (the copy), 092 (E1 and E3, the judge's traces)

## context_files

- docs/solution-2026-10-16.md (4.2, 4.3); src/w2m/x86ref.py; src/w2m/pe.py; src/w2m/modules.py;
  scripts/static_census.py; store/laptop-traces/*

## signal contract

### Emits

- None new (a computation, kept as data).

### Invariants

- Vocabulary unchanged.

## artifact contract

- `scripts/reach_census.py`; pe.py reads forwarders and ordinal-only exports; `store/census/reach/*.json` with the
  copy's build and the script's sha256; tests on the copied DLLs' real bytes where CI has them, on small fixtures cut
  from them where it does not
- `scripts/gates.sh` exit 0; CI `success`

## observation contract

- **E1 as judge.** Every system call probes/hello made after its first two (the creating thread's, pid 0) is in the
  set its imports and the process start reach; any that is not is listed with the reason (an indirect call, a
  callback from the kernel).
- **E3 as judge.** Every system call and win32k call Hail to the Rainbow made in E3's window is in the set its roots
  reach; misses are listed the same way.
- The counts: per game, system calls reached (of 488 and 1,486), (call, class) pairs, unresolved indirect sites.

## done criteria

E2's sets are committed for both games with their builds; the two judges hold, or each miss is named with its cause.

## observed

- **Closed once, as the full review of 2026-10-16 directs** (item 3: static reach sizes the work against the traces;
  it does not order it). The walk, `src/w2m/reach.py` (sha256 f8f8cbb1…) and `scripts/reach_census.py`, wrote
  `store/census/reach/{hail-to-the-rainbow,bioshock2-remastered}.json` in 60 min 31 s on this Mac.
- **The counts** (direct tier; pointer tier in brackets, which adds code addresses taken, imports read as values,
  data code pointers and run-time lookups):

  | game, roots | roots | NT calls of 488 | win32k calls of 1,486 | (call, class) rows | unresolved indirect sites |
  |---|---|---|---|---|---|
  | Hail to the Rainbow, imports | 1,446 | 244 (373) | 342 (1,336) | 400 | 31,099 |
  | Hail to the Rainbow, P/Invokes | 101 | 200 (354) | 207 (1,336) | 272 | 1,082 |
  | Hail to the Rainbow, lookups | 852 | 206 (354) | 246 (1,336) | 316 | 1,492 |
  | Hail to the Rainbow, loader | 8 | 124 (137) | 0 | 100 | 269 |
  | BioShock 2 Remastered, imports | 3,081 | 228 (341) | 331 (1,332) | 295 | 41,932 |
  | BioShock 2 Remastered, lookups | 361 | 217 (340) | 254 (1,332) | 282 | 38,390 |
  | BioShock 2 Remastered, loader | 4 | 98 (98) | 0 | 73 | 189 |

  The pointer tier reaches 1,336 of 1,486 win32k calls: an upper bound that covers nearly the table, as the review
  says. Roots left unbound: Hail to the Rainbow 1,001 (910 steam_api64 P/Invokes, E10's; macOS P/Invokes; six
  extension API sets with no host; psmserviceexthost.dll, absent from the copy; shell32 #102), BioShock 2 10.
- **E3 as judge: 0 misses.** The misses went 11, 8, 4, 0; each named a class, and each class was searched whole
  before the next run: run-time loading (UnityPlayer loads d3d11 and dxgi), named forwarders imported by ordinal
  (d3d11's 100 D3DKMT imports), function pointers passed as arguments (CFG's `jmp rax` dispatch), imports stored into
  tables (d3d11 fills the driver's callback table: `mov [rdi+0x250], rax`), and lookups by name inside Windows's own
  DLLs (D3DKMTGetDeviceState in d3d11, dxgi and nvwgf2umx).
- **A person's session as judge: 303 of 305.** A person played 884 s (lap-20261017T022622359Z-ced97d27;
  `store/laptop-traces/session-lap-20261017T022622359Z-ced97d27/`): 155 NT and 150 win32k call kinds, 263 of them outside E3's window. One miss was
  the check's own (a Zw name not mapped to Nt). The other, NtPlugPlayControl (1,230 calls, all in the start's 15 s;
  classes 14 DeviceStatus and 23 GetDeviceInterfaceEnabled), has a user stack whose top is ntdll+0x161b34, the
  return of **NtDeviceIoControlFile**'s stub, called from KernelBase+0x4348c (the laptop's DLLs equal the copy by
  sha256). So the user-mode call was NtDeviceIoControlFile, which the walk reaches; the stack fits a Zw call the kernel
  made on its own, which the syscall provider counts as the process's. Not tested further: the roadmap's order moves
  on (a test would compare each probe's name with the stub atop its user stack).
- **Defect classes fixed, kept for the loader:** pe.py reads forwarders (keyed by ordinal, and by name where named)
  and ordinal-only exports; `Image.export` refuses a forwarded name; x86ref resolves 32-bit absolute IAT slots
  (SysWOW64 kernelbase: 0 to 6,371 calls resolved), so `store/census/static/instructions-bioshock2-remastered.json`
  was regenerated (its 32-bit unresolved indirect counts fell, e.g. 1,500 to 493); `src/w2m/apiset.py` reads the
  version-6 schema with per-importer values.
- **E1 as judge: not run.** hello.exe was fetched from the laptop and matches E1's sha256 (5b520ef1…); its walk was stopped unfinished when the user moved the project to the new roadmap (2026-10-16).
- **Stutter, not settled.** The person reported stutter. The session's 15 s present counts show the menu at the display's 144 Hz and the first level at 47 to 92 frames a second; frame times were not recorded. Two replays of the session (without and with DTrace, with per-present times from `pmc_summary.py`'s new present_times.tsv) were stopped unfinished by the same move.
- **Limits:** pointers passed on the stack (argument 5 on) are not followed; 32-bit code has no parameter tracking;
  calls made only by the vendor's driver (nvwgf2umx) are listed apart.
