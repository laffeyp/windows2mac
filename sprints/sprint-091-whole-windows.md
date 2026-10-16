# Sprint 091 — all of Windows, copied once: C:\Windows and the SOFTWARE and SYSTEM hives

```yaml
---
id: 091
status: active
phase: 5
pass_kind: functional
---
```

## scope

Step 0 of the solution (docs/solution-2026-10-16.md, section 3; Decision 2026-10-16 user, Windows's own code as the
specification). The depot at `~/w2m-depots/windows-system` holds System32 and SysWOW64 only: no executables, no
hives, no WinSxS. The plan runs Microsoft's own DLLs and executables and reads the laptop's registry as data, so the
whole input is copied now (memory: acquire whole inputs), not piece by piece as E1 would ask.

- **The tree.** The laptop's `C:\Windows` whole (183,279 files, 64.4 GB measured 2026-10-16; build 26200.7840),
  streamed by the laptop's `tar.exe` over `ssh w2m-laptop` into `~/w2m-depots/windows/Windows/`. Every file the
  stream cannot read is listed with tar's error, not dropped silently.
- **The hives.** `reg save HKLM\SOFTWARE` and `reg save HKLM\SYSTEM` (the session holds SeBackupPrivilege) into
  `~/w2m-depots/windows/hives/`. The live hive files under `System32\config` are locked and are expected among the
  unreadable; the saved hives stand for them.
- **The laptop's own listing.** Every file's path and size under `C:\Windows`, written on the laptop before the
  copy, into `~/w2m-depots/windows/listing.tsv`; the judge of the copy.
- **The build.** `build.txt` with CurrentBuild, UBR, edition and the copy's date.
- **Agreement with the old copy.** Every file of `windows-system/System32` and `windows-system/SysWOW64` compared by
  sha256 with the new copy's: the census's export tables (Sprint 088) came from the same bytes.

The census keeps reading `windows-system` this sprint; pointing it at the new depot is the next sprint's.

## prerequisites

- 090 (committed 4a84cbbb)

## context_files

- docs/solution-2026-10-16.md; sprints/sprint-088-service-surface.md (the first copy); the laptop (`ssh w2m-laptop`)

## signal contract

### Emits

- None new (a copy of data).

### Invariants

- Vocabulary unchanged.

## artifact contract

- `~/w2m-depots/windows/` (`Windows/`, `hives/SOFTWARE`, `hives/SYSTEM`, `listing.tsv`, `unreadable.txt`,
  `build.txt`) on disk, outside the repo
- `store/census/windows-manifest.tsv.gz`: every copied file's path, size and sha256, committed

## observation contract

- The manifest's file count and bytes against the laptop's listing: every listed file is copied with its size, or is
  in `unreadable.txt` with its error.
- The two saved hives open as registry files (the `regf` signature at offset 0).
- The old copy's files equal the new copy's by sha256, every one.

## done criteria

Every file of the laptop's listing is in the depot at its size or named unreadable with its cause; both hives saved;
the old System32 and SysWOW64 equal the new copy's by sha256.

## observed
