# Sprint 087 — the large review of 082-086, the instruction census: precision, 32-bit, the module map

```yaml
---
id: 087
status: closed
phase: 5
pass_kind: functional
---
```

## scope

The reviewer's large review of Sprints 082-086 (H2, H3 in part, M2; Decision 2026-10-16, the order). The census
must be right before it orders work.

- **Jump tables out of the sweep (H2).** `sweep`'s premise was false: MSVC puts jump tables inside `.pdata`
  functions in UnityPlayer.dll (0x130260: RET, NOP, then 4-byte RVAs into the function, read as FADD, ADC, IN). The
  sweep now stops at an unconditional transfer (RET, JMP, an indirect JMP) and resumes only at an address the walk or
  a jump table references; a 4-byte word that is an RVA inside its own function is a table entry, not code. Each
  instruction is marked by how it was found (walk, sweep), and the summary counts sweep-only ones apart.
- **A precision judge (H2).** Sweep-only decodes that overlap a word that is an RVA into its own function, or that
  cross an instruction boundary the walk holds: counted, and 0 after the fix where the reviewer found 3,273 and more.
- **The duplicate depot (M2).** `hail-to-the-rainbow-native` holds the same 7 images as `hail-to-the-rainbow`; the
  census names images by sha256 and counts each once over all depots.
- **The 32-bit instruction census (H3).** w2m's decoder takes 64-bit mode only; BioShock 2's 16 images are sized by
  Capstone in 32-bit mode (mnemonics and instances, walk and sweep as above), written beside the x86-64 census. The
  decoder's 32-bit mode is the first item of the 32-bit work list.
- **The module map in the frontier record (M2).** A frontier run writes each module's base, size and sha256 into its
  record, so a judge reads bases rather than voting them out of bytes.

## prerequisites

- 086 (closed on CI)

## context_files

- `scripts/class_census.py`, `src/w2m/x86ref.py`, `scripts/run_frontier.py`, the large review (BLACKBOARD Decision
  2026-10-16, the order)

## signal contract

### Emits

- None new. The frontier record's state gains the module map (no new tag).

### Invariants

- Vocabulary unchanged. No lifting or compiler change.

## artifact contract

- `scripts/class_census.py`, `src/w2m/x86ref.py` (a 32-bit walk), `scripts/run_frontier.py`,
  `store/census/static/instructions-*.json`, tests
- `scripts/gates.sh` exit 0; CI `success`

## observation contract

- The precision judge's count before (the reviewer's lower bound, 3,273 in UnityPlayer) and after (0).
- The x86-64 census's totals with sweep-only counted apart; CLI, STI, IN and OUT counts after.
- BioShock 2's 32-bit census: instructions and mnemonics.

## done criteria

The census counts code, not jump-table data, says how each instruction was found, covers the 32-bit images, and
counts each image once.

## observed
- **Jump tables.** `class_census.reachable`: Capstone's walk, with the targets of every MSVC switch table it finds
  (`mov r32, dword ptr [base + index*4 + table]`, entries read while each is an RVA inside the load's function)
  walked as extra roots until no new table appears; then the sweep, which after RET, JMP, INT3, UD2 or HLT resumes
  only at an address the walk or a table reaches, and never decodes a table. UnityPlayer.dll: 649 tables, 14,885
  entries; the walk alone now holds 4,996,586 instructions (4,904,229 before); the sweep adds 7.
- **Precision judge** (`precision`, asserted in CI on a probe): sweep-only instructions holding an RVA into their own
  function, 0 (the reviewer's lower bound before: 3,273 of 110,004 sweep-only); overlapping a walked instruction, 0.
  UnityPlayer's CLI 19, STI 2, IN 4, OUT 6 remain, all walked, not swept; why the walk reaches them is not measured.
- **The census** (5 min 53 s): Hail to the Rainbow 6,201,183 instructions in 475 classes; not lifted 10,900 in 46
  classes (INT3 7,889; AES, SHA, CLMUL, ADX); run by the model 1,119 in 32 classes (SHRD 776, SHLD 198, RCR 22,
  PUSHFQ 19, XGETBV 17, POPFQ 16, STC 10).
- **Recall** (`judge_record`): UnityPlayer.dll 679,224 of 694,181 (97.8%; 97.9% with the noisy sweep),
  mono-2.0-bdwgc.dll 200,233 of 200,733 (99.8%), steam_api64.dll 3,691 of 3,697 (99.8%).
- **32-bit** (`census32`, Capstone in 32-bit mode; w2m's decoder takes 64-bit mode only): BioShock 2's 16 images,
  848,970 reachable instructions, 275 mnemonics (mov 224,487, push 164,441, call 68,618). A PE32 image has no `.pdata`,
  so only the walk and its switch tables find code: C++ virtual calls stay unresolved, and the count is a lower bound
  whose recall no judge measures yet (no 32-bit run exists).
- **One count per image.** Images are named by sha256 across depots; hail-to-the-rainbow-native's 7 images count
  under hail-to-the-rainbow, and its file names where (`same_as`).
- **The module map.** A frontier run writes `<execution id>.modules.tsv` beside its call tables: each process's images,
  base, size and timestamp (a scratch run: the exe at 0x140000000, UnityPlayer.dll at 0x180000000, mono at
  0x1180010000, and the crash handler, pid 4100, at 0x140000000). The batch run that made the stencil record predates
  it, so `judge_record` still finds that run's bases from its bytes.
