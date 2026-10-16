# Sprint 085 — the static census: every depot's images and the Windows functions they can call

```yaml
---
id: 085
status: closed
phase: 5
pass_kind: architecture
---
```

## scope

Step 1 of `docs/recentering-2026-10-16-knowable-first.md` (user decisions 2026-10-16: any game, at speed; knowable
first). One script, `scripts/static_census.py`, reads every depot under `~/w2m-depots` and writes committed data,
`store/census/<depot>.json`, with no run of any game:

- **Images.** Every PE file: its path in the depot, sha256, size, machine (x86-64 0x8664, x86 0x14C), subsystem,
  whether it is a .NET assembly (a CLR header), and whether the depot ships it (another image imports it by name).
- **The service surface.** Each image's imports and delay imports, by DLL and name or ordinal; the Windows functions
  its data names (a string equal to an export of a Windows DLL, from mingw-w64's import libraries, 943 of them, for
  x86-64 and i686: a `GetProcAddress` candidate); and, for each function, whether w2m serves it
  (`vm_backend.service_for`). Imports between the depot's own images are kept apart.
- **32-bit images read.** `pe.py` reads PE32 headers, sections and import tables as it reads PE32+ (the loader still
  refuses them), so the census sees BioShock 2 Remastered's 16 images and Hail to the Rainbow's 122.
- **The summary** per depot and over all: images by machine; Windows functions by DLL, served and not; the functions
  named only in data. The order of the next sprints comes from it.

The instruction surface (every function `.pdata` lists, decoded and lifted) is Sprint 086.

## prerequisites

- 084 (closed)

## context_files

- `src/w2m/pe.py`, `src/w2m/vm_backend.py` (`service_for`), `docs/recentering-2026-10-16-knowable-first.md`,
  `store/game-runs/38006665419/jit_capture.bin.imports.tsv` and `.dynamic.tsv` (the Windows run's calls: a judge)

## signal contract

### Emits

- None: the census is data a script writes, not a run (no new tag). A later sprint that plans from it cites its file
  and sha256.

### Invariants

- Vocabulary unchanged. The loader's behavior unchanged (PE32 read, not loaded).

## artifact contract

- `scripts/static_census.py`, `src/w2m/pe.py`, `store/census/*.json`, `tests/test_static_census.py`
- `scripts/gates.sh` exit 0; CI `success`

## observation contract

- **Judge from outside the census's sources** (KIT_DIARY #59): every function the game called in its Windows run
  (`.imports.tsv`, `.dynamic.tsv`) is in Hail to the Rainbow's census, as an import or a named string; the census
  reports any it misses.
- The counts: images by machine per depot; Windows functions declared and served; functions named only in data.
- PE32 parsing judged against `objdump -p` (Xcode's llvm-objdump) import tables for the 32-bit images.

## done criteria

The knowable service surface and image formats of every depot on disk are committed data, judged against the
Windows run's calls, with w2m's coverage beside each.

## observed
- **The census** (`scripts/static_census.py`, 10 s over every depot; `store/census/static/*.json`, byte-reproducible):
  Windows exports from mingw-w64's import libraries, x86-64 849 DLLs and 107,666 names, x86 389 DLLs and 82,552.
  - Hail to the Rainbow: 129 images, 7 x86-64 and 122 marked 0x14C, all 122 .NET assemblies (IL for mono, not native
    32-bit code); 634 Windows functions imported, 238 served; 488 more named in the images' data.
  - BioShock 2 Remastered: 16 images, all native x86 (0x14C), none .NET; 1,114 Windows functions imported, 187 served
    by name (w2m loads no 32-bit image); 152 more named in data. Its DLLs include msvcp140, msvcr110 and the
    api-ms-win-crt sets.
  - windows-fonts-38226907442 (a probe): 77 imported, 69 served.
- **Judge** (`tests/test_static_census.py`, in CI from committed data): the 803 functions the game called on Windows
  (store/game-runs/38006665419) are all in the census as imports of the module that called them; of the 331 names it
  looked up at run time, 40 are Windows functions, all named in its images, 282 are exports of its own images (mono's
  API), and 9 are Unity's native-plugin entry points (UnityPluginLoad and eight more), which no file on disk exports.
- **PE32** read by `pe.parse(x86=True)`: its by-name import tables equal Xcode objdump's for all 16 BioShock 2
  images. The loader still takes PE32+ only.
- **A correction.** I told the user, and docs/recentering-2026-10-16-knowable-first.md first said, that Hail to the
  Rainbow's depot holds 122 32-bit PE files; they are .NET assemblies. The document carries the correction.
- **Next** (from the census): the instruction census of every x86-64 image's `.pdata` functions (Sprint 086); then
  the compiler generated from the rules, the census's instruction and service work lists, and 32-bit images.
