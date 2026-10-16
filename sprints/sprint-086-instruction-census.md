# Sprint 086 — the instruction census: every x86-64 image's reachable code, by how w2m runs it

```yaml
---
id: 086
status: closed
phase: 5
pass_kind: architecture
---
```

## scope

Step 1 of `docs/recentering-2026-10-16-knowable-first.md`, its instruction half. The census exists:
`scripts/class_census.py` (Sprint 044) walks an image's reachable code (Capstone's recursive traversal from the
entry, exports, TLS callbacks and `.pdata` starts, `x86ref.walk`), names each instruction's class with w2m's decoder
and lifts it; its last count, 2026-10-11, was UnityPlayer.dll's 4,879,842 instructions in 460 classes, 110 not lifted.
It was run by hand on one game's files and not since. This sprint makes it the census of every depot:

- **`--depots`:** every x86-64 image of every depot (from `store/census/static`, .NET assemblies left out), one
  process per image, written to `store/census/static/instructions-<depot>.json` with the image's sha256.
- **A third tier.** An instruction that lifts is counted apart when its operations include one the VM sends to the
  Python model (op kinds `x86`, `x87`, `ud`, `fcw`, `xgetbv`; `compile_insn`, hvc #11): it runs, but at the host's
  speed. So each class reads: native, model, not lifted.
- **The work lists** printed per depot and over all: classes not lifted and classes run by the model, by instances.

## prerequisites

- 085 (closed)

## context_files

- `scripts/class_census.py`, `src/w2m/x86ref.py`, `src/w2m/vm_backend.py` (`compile_insn`'s model kinds),
  `store/census/static/*.json`

## signal contract

### Emits

- None (data a script writes, as Sprint 085).

### Invariants

- Vocabulary unchanged. No lifting rule or compiler change.

## artifact contract

- `scripts/class_census.py`, `store/census/static/instructions-*.json`, `tests/test_class_census.py` (or the census
  tests' file)
- `scripts/gates.sh` exit 0; CI `success`

## observation contract

- **Judge from outside the census's sources:** the instructions the game's batch run compiled
  (`store/stencil/record.txt.gz`, 1.3 M) each lie in the reachable set of their image, or are named as reached only
  at run time (mono's JIT code, outside every image).
- The counts: per image and over all, instances and classes native, model and not lifted.

## done criteria

The instruction surface of every x86-64 image on disk is committed data with how w2m runs each class, judged against
the batch run's compiled instructions.

## observed
- **The census** (`class_census.py --depots`, 6 min 41 s; `store/census/static/instructions-<depot>.json`): Hail to
  the Rainbow's 7 x86-64 images hold 6,227,209 reachable instructions in 490 classes.
  - Not lifted: 12,380 in 58 classes. INT3 8,322 (padding and breaks); AES-NI, SHA, CLMUL, ADX and VAES (about 2,700:
    code that runs where CPUID reports them); CLI 258, STI 240, IN 83, OUT 76, INT 77 and MOV 110 (not measured, but
    no Windows user-mode game runs CLI, STI, IN or OUT: most likely bytes inside `.pdata` ranges that are not code,
    decoded by the sweep).
  - Run by the Python model: 1,692 in 37 classes (SHRD 776, SHLD 198, CLC 108, LOOPE 47, STC 44, PUSHFQ 40, CMC 39,
    JRCXZ 34, POPFQ 33, SCASD 30, RCR 30, the string forms, FWAIT, XGETBV).
  - The probe (windows-fonts): 23,122 instructions, 108 not lifted, 3 by the model.
- **The walk misses code.** Capstone's recursive walk alone held 68% of the mono code the batch run compiled and
  97.3% of UnityPlayer's: code reached through jump tables and pointers. The census now joins it with a linear decode
  of every `.pdata` function (`sweep`).
- **Judge** (`judge_record`, `tests/test_class_census.py`, marked `depot`): the batch run's compiled instructions in
  their images' reachable sets: UnityPlayer.dll 679,489 of 694,181 (97.9%), mono-2.0-bdwgc.dll 200,181 of 200,733
  (99.7%), steam_api64.dll 3,691 of 3,697 (99.8%). Each image's base in that run is found from the record (mono and
  steam_api64 were relocated, to 0x1180010000 and 0x1222ff0000). UnityCrashHandler64.exe (75.8%) shares 0x140000000
  with the game's exe in the record, so its figure mixes two images.
- **085's CI failure** (run 38834313370): its two depot tests skipped in CI, and W2M_REQUIRE_REFERENCES=1 fails any
  skip but `hvf`'s. A `depot` marker (tests/conftest.py) now exempts a test that reads ~/w2m-depots where they are
  absent; the local gates run them.
- **Gates:** `scripts/gates.sh` exit 0, 2,450 passed.
