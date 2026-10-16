# Sprint 088 — the large review of 082-086, the service census: Windows's own exports, redistributables, P/Invoke, COM

```yaml
---
id: 088
status: active
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
