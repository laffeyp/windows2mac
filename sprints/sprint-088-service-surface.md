# Sprint 088 — the large review of 082-086, the service census: Windows's own exports, redistributables, P/Invoke, COM

```yaml
---
id: 088
status: closed
phase: 5
pass_kind: functional
---
```

## scope

The reviewer's large review of Sprints 082-086 (H3, M1; Decision 2026-10-16, the order): the service surface of
`store/census/static` made exact.

- **Windows's own export tables (M1).** The Windows laptop's `C:\Windows\System32` and `C:\Windows\SysWOW64` DLLs,
  copied whole (memory: acquire whole inputs) to `~/w2m-depots/windows-system/` and read by `pe.parse`: each DLL's
  exports by name and ordinal, committed as `store/census/windows-exports.json` with the laptop's Windows build. The
  census takes names from it, not from Homebrew's mingw-w64 import libraries, and a name's DLL is every DLL that
  exports it.
- **Each imported DLL classed:** part of Windows (in System32 on a clean install: the laptop's list, less the
  redistributables Microsoft names), a Microsoft redistributable (the Visual C++ runtimes msvcp, msvcr, vcruntime,
  concrt, vcomp; the DirectX redistributable's d3dx9, d3dcompiler, xinput1_3, XAudio2_7, X3DAudio), or shipped by
  the depot. A redistributable is a PE w2m can load rather than a set of services to write; the census says how many
  imports each class holds.
- **.NET P/Invoke from metadata (H3).** Each assembly's ImplMap and ModuleRef tables (ECMA-335 II.22.22, II.22.31):
  every DllImport's DLL and entry point, exactly, in place of strings that happen to match an export.
- **COM and the named-in-data list (H3, M1).** IID and CLSID constants in each image's data matched against the
  Windows SDK's (from mingw-w64's `uuid` library and headers: names and GUIDs); the ASCII names of native images kept
  only where the image also imports `GetProcAddress`, and tiered (`named`: native; .NET strings dropped for the
  metadata's).
- **Steam interface versions and LoadLibrary names (H3):** strings matching `Steam\w+\d{3}` in steam_api64's and the
  game's data; DLL names in each image's data.
- **CI holds the parsing (M3):** the census's JSON carries its script's commit and its export source; a test checks the
  P/Invoke reader and the GUID matcher on committed fixtures.

## prerequisites

- 087 (closed on CI)

## context_files

- `scripts/static_census.py`, `src/w2m/pe.py`, the laptop (`ssh w2m-laptop`; memory: Windows laptop reference)

## signal contract

### Emits

- None new (data a script writes).

### Invariants

- Vocabulary unchanged.

## artifact contract

- `scripts/static_census.py`, a metadata reader (`src/w2m/clr_metadata.py`), `store/census/windows-exports.json`,
  `store/census/static/*.json`, tests
- `scripts/gates.sh` exit 0; CI `success`

## observation contract

- The judge of Sprint 085 still holds (803 of 803 imports; the run-time lookups named but the 9 Unity hooks).
- P/Invoke: the assemblies' DllImports, by DLL; judged against the names the Windows run's `.dynamic.tsv` holds that
  mono looked up for managed code.
- Counts per depot: imports by class (Windows, redistributable, shipped), served or not; COM interfaces named.

## done criteria

The service surface counts Windows's real exports, separates what w2m must write from what it can load, and names
the managed code's native calls and the COM interfaces exactly.

## observed
- **Windows's own tables.** The laptop's System32 (3,415 DLLs) and SysWOW64 (2,268), copied whole to
  ~/w2m-depots/windows-system (2.6 GB; the first copy missed 460 files whose extension is upper or mixed case, since
  scp's remote glob is case-sensitive; recopied and counted), Windows 11 25H2, build 26200, UBR 0x1ea0, Home.
  `store/census/windows-exports.json.gz` (1.3 MB): x86-64 2,978 DLLs with exports, 120,902 names; x86 1,901 DLLs,
  110,695 names. `store/census/windows-com.json.gz` (0.6 MB): 27,915 registered interfaces and 7,505 classes from the
  registry's HKCR\Interface and HKCR\CLSID. The census no longer reads mingw-w64.
- **P/Invoke** (`src/w2m/clr_metadata.py`, ECMA-335 II.22.22 ImplMap, II.22.31 ModuleRef): all 122 assemblies of Hail
  to the Rainbow read, 1,102 distinct DllImports; 910 to steam_api64, of which 902 are exports of the steam_api64.dll
  the game ships (the 8 are newer Steamworks bindings the DLL lacks); the others advapi32, kernel32, iphlpapi and the
  macOS frameworks mono's class library names. A CI test reads two committed assemblies (XInputDotNetPure, mono's
  System.EnterpriseServices); XInputDotNetPure's two imports are the two exports of the XInputInterface.dll shipped.
- **DLL classes.** BioShock 2's 1,114 imports: 931 from Windows DLLs (187 served by name), 164 from Microsoft
  redistributables w2m can load as PE files (msvcp140 63, msvcr110 62, vcruntime140 21, msvcp110 6, d3dx9_43 6,
  d3dcompiler_43 4, xinput1_3 2), and 19 from mss32.dll (Miles Sound System), which no file in the depot has. Hail to
  the Rainbow's 634: all Windows, 238 served.
- **COM.** GUIDs of registered interfaces and classes in each native image's bytes: Hail to the Rainbow 126 (73
  interfaces, 53 classes: WinRT's IInspectable and IAsyncInfo, Media Foundation's IMFTransform); BioShock 2 399 (311
  interfaces, 88 classes; among them Windows Update's IAutomaticUpdates: matched, not explained).
- **Strings.** Steam interface versions: 17 in Hail to the Rainbow (SteamClient020, SteamInput005), 16 in BioShock 2;
  DLL names per image.
- **Candidate lookups.** Names equal to a Windows export, only in native images that import GetProcAddress, .NET
  strings dropped: Hail to the Rainbow 504, of which the Windows run looked up 40 (precision about 8%); labelled
  candidates in the summary.
- **Judge** (Sprint 085's, in CI): 803 of 803 imports; 322 of 331 run-time lookups, the 9 Unity plugin hooks left.
- **Not done:** shader bytecode (the graphics phase's census); the precision of the candidate lookups beyond the
  label (an xref to a GetProcAddress call would narrow them).
- **Gates:** `scripts/gates.sh` exit 0, 2,451 passed.
