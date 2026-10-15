# Sprint 075 — the decoder in C

```yaml
---
id: 075
status: closed
phase: 5
pass_kind: functional
---
```

## scope

Recentering step 6, under Decision 2026-10-14 (the stencil key): the compiler inside the VM keys a stencil by what
the decoder gives, so it needs the decoder in C, generated from `specs/x86_decode.json` as the Python's tables are.

- **A. The tables.** `scripts/gen_decoder.py` writes `tools/vm_compiler/x86_decode_tables.h` from the spec beside
  `src/w2m/x86dec_tables.py`: each row's fields as a C struct, an index by (map, opcode, encoding), the iclass names
  (with the derived `_LOCK`, `REP_`, `REPE_` and `REPNE_` names) and the row identities; `--check` covers both files.
- **B. The decoder.** `tools/vm_compiler/x86_decode.c` follows `x86dec._decode` step for step (prefixes, VEX, EVEX,
  XOP, mandatory-prefix selection, ModRM, SIB, displacement, immediate, the special cases) and `x86ops.operands` for
  the explicit operands, freestanding with the guest runtime's flags. The CPU-feature fallback is left out: `lift`
  calls `decode` with no feature set, so it never applies. The mnemonic is not an output (nothing in the compiler
  reads it); the iclass is.
- **C. The differential.** `tests/test_x86_decode_c.py` builds the C for the host and compares it with
  `x86dec.decode` and `x86ops.operands`: the error kind (InvalidEncoding, Truncated, DecodeError, no operand template)
  or the length, iclass, row, every `Fields` member and every operand. Over every spec row's sample encodings
  (the rows' own test bytes where the spec has them), seeded random byte strings, and, as an observation, the
  1,183,579 instructions of Sprint 074's record.

## prerequisites

- 074 (closed, on CI)

## context_files

- `src/w2m/x86dec.py`, `src/w2m/x86ops.py`, `scripts/gen_decoder.py`, `specs/x86_decode.json`

## signal contract

### Emits

- None new: no run, compiler or runtime change.

### Invariants

- Vocabulary unchanged. The runtime image and the Python decoder unchanged (`src/w2m/x86dec_tables.py` byte for
  byte).

## artifact contract

- new: `tools/vm_compiler/x86_decode.c`, `tools/vm_compiler/x86_decode.h`, `tools/vm_compiler/x86_decode_tables.h`,
  `tests/test_x86_decode_c.py`; modified: `scripts/gen_decoder.py`
- `uv run pytest` exit 0; CI `success` on the closing commit (its decoder-tables step checks both outputs)

## observation contract

- **C:** byte strings compared, by outcome; the count that differ (0 required), in the test and over the record.

## done criteria

A to C; 0 differ; CI passes.

## observed
- **A.** `scripts/gen_decoder.py` writes `tools/vm_compiler/x86_decode_tables.h` beside the Python tables, which come
  out byte for byte unchanged: 1,067 rows, 106 operand templates as tokens (each resolved by `x86ops.operands`' tests
  in their order; 46 distinct tokens), 653 base iclass names and their `_LOCK`, `REP_`, `REPE_` and `REPNE_` names
  (3,265 in all), the index by (map, encoding, opcode), the legacy-prefix opcodes. `--check` covers both files.
- **B.** `tools/vm_compiler/x86_decode.c` and `.h`; builds with the guest runtime's CFLAGS.
- **C** (`tests/test_x86_decode_c.py`). The spec holds no sample bytes per row, so the test builds them: 12 per row
  from its encoding, map, opcode, mandatory prefix, ModRM constraints, W and L, with random bytes after; 40,000 random
  byte strings (half after a run of prefix-like bytes); every truncation of 3,000 of these. Over the 84,409 strings:
  50,982 decoded, 6,627 InvalidEncoding, 12,824 Truncated, 13,976 DecodeError; 0 differ in outcome, length, iclass,
  row, any `Fields` member or any operand. The samples reach all 1,067 rows (30 per row, a test of its own). The
  first run differed on 1,147 strings, all VEX, EVEX or XOP: their pp field orders F3 before F2, the reverse of the
  tables' prefix codes; fixed by a mapping (x86dec's `VEX_PFX`). Three faults planted in a scratch copy, one at a
  time: a RIP-relative displacement of 2 bytes (754 differ), `_LOCK` on XCHG (9), the SIB index extended by REX.B in
  place of REX.X (343).
- **The record:** Sprint 074's 1,183,579 instructions, each decoded in C and in Python: all decoded, 0 differ.
- **Closed** on 982bb22e (CI run 38679290552, success; its decoder-tables step checks both outputs). `uv run pytest`: 2,427 passed.
