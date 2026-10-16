# Sprint 087 — the large review of 082-086, the instruction census: precision, 32-bit, the module map

```yaml
---
id: 087
status: active
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
