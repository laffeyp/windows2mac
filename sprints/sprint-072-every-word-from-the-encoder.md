# Sprint 072 — every word from the encoder

```yaml
---
id: 072
status: open
phase: 5
pass_kind: functional
---
```

## scope

Recentering step 6, the first build step of Decision 2026-10-14 (the compiler inside the VM as copy-and-patch at the
encoder level). A stencil is a sequence of encoder calls, so a word built inline has no stencil. Sprint 071's census
found 394,155 such words in the batch run's first 100 billion instructions, and a probe that records each word's
caller put them on seven lines of `vm_backend.py`:

| line | words | what |
|---|---|---|
| `compile_insn` ADD/SUB with an immediate (`ADD_SUB_IMM`) | 206,740 | ADD, SUB Rd, Rn, #imm12, 32 and 64 bits |
| `compile_insn` computed address, displacement above 0 | 67,736 | ADD Xd, Xd, #disp |
| `_set_flags` X24_CLEAR, X24_KEEP_CF | 48,771 | MOV X24, XZR; AND X24, X24, #1 |
| `compile_insn` computed address, displacement below 0 | 28,218 | SUB Xd, Xd, #-disp |
| `compile_insn` Jcc's target (`a.emit(*target)`) | 27,063 | the census's own fault: a MOVZ/MOVK encoded before the branch emitted ahead of it |
| `_nzcv` 8- and 16-bit | 15,610 | LSL W16, Wa, #s; ADDS/SUBS WZR, W16, Wb, LSL #s |
| `_nzcv` 32-bit | 17 | ADDS/SUBS WZR, Wa, Wb |

- **A. The encoders.** `a64_encoder.py` gains ADD/SUB immediate in 32 bits, ADDS/SUBS in 32 bits with a shift, LSL in
  32 bits and AND with a low-ones immediate, each checked against clang's assembly (`tests/test_a64_encoder.py`).
- **B. The six sites** call them. The words do not change: every call of Sprint 071's record compiled again gives the
  words it gave before, word for word.
- **C. The census** places a call encoded before the words ahead of it, and shows no raw word.

## prerequisites

- 071 (closed)

## context_files

- `src/w2m/vm_backend.py` (`_nzcv`, `_set_flags`, `compile_insn`), `src/w2m/a64_encoder.py`,
  `scripts/stencil_census.py`

## signal contract

### Emits

- The family sweep's records (CANDIDATE_ACCEPTED for all 68 families) and the class runs on the compiler change.

### Invariants

- Vocabulary unchanged. Every recorded call's words unchanged.

## artifact contract

- modified: `src/w2m/a64_encoder.py`, `src/w2m/vm_backend.py`, `scripts/stencil_census.py`,
  `tests/test_a64_encoder.py`
- `uv run pytest` exit 0; 68 of 68 families, 456 of 458 classes; CI `success` on the closing commit

## observation contract

- **The words:** the record's 1,186,511 calls compiled before and after, the count that differ (0 required).
- **The census:** raw words 0.

## done criteria

A to C; the words equal; the sweeps and CI pass.

## observed
