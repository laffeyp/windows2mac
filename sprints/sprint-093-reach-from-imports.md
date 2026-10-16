# Sprint 093 — E2: what the games' imports reach, through Microsoft's DLLs, in system calls and information classes

```yaml
---
id: 093
status: active
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
