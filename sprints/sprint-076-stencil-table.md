# Sprint 076 — the stencil table, from instances made per key

```yaml
---
id: 076
status: closed
phase: 5
pass_kind: functional
---
```

## scope

Recentering step 6, under Decision 2026-10-14 (the stencil key, and holes from instances made per key). The table
the compiler inside the VM will carry: for each key, the encoder sequence, and for each argument either a constant or
the expression of a decoded value that gives it.

- **A. The instances.** `scripts/stencil_table.py` takes each key of Sprint 074's record and makes instances of it by
  changing the instruction's decoded fields in its bytes (ModRM reg and rm, REX.R, X and B, SIB, displacement and
  immediate bytes, VEX.vvvv and VEX's R, X and B) and its address; an instance is kept when it decodes to the same
  length and key and lifts.
- **B. The table.** Each instance compiled with its encoder calls logged. A key whose instances give more than one
  encoder sequence is reported (the Decision's revisit condition). Each argument is a constant where every instance
  gives the same value, else the first expression of `stencil_census.hole_expression` that gives it in every
  instance, or a runtime symbol's address; `other` otherwise.
- **C. The judge.** A second set of instances per key, drawn with another seed and not used for B: each one's words
  rebuilt from the table (each argument from its constant or expression over the instance's decoded values, each
  encoder called) against the Python compiler's words. And the record's own 1,313,377 compiled instructions rebuilt
  the same way.

## prerequisites

- 075 (closed, on CI)

## context_files

- `scripts/stencil_census.py`, `src/w2m/x86dec.py`, `src/w2m/vm_backend.py` (`compile_insn`, `Assembler`)

## signal contract

### Emits

- None new: a script over a scratch record; no run, compiler or runtime change.

### Invariants

- Vocabulary unchanged. The compiler and runtime unchanged.

## artifact contract

- new: `scripts/stencil_table.py`, `tests/test_stencil_table.py` (instances keep their key; a table built from a
  handful of instructions rebuilds fresh instances exactly)
- `uv run pytest` exit 0; CI `success` on the closing commit

## observation contract

- **A:** keys, instances tried and kept per key (distribution), keys with no instance beyond the recorded one.
- **B:** keys with one sequence over their instances; arguments by kind (constant, expression kind, symbol, other).
- **C:** test instances and record instructions rebuilt, the count whose words differ (0 required), by cause.

## done criteria

A to C; C with 0 differing, or each difference's cause named and recorded; CI passes.

## observed
- **Final run** (`scripts/stencil_table.py` over Sprint 074's record, 80 s on 9 processes): 38,154 keys, a stencil for
  every one (71 refined by a feature, below); 903,117 instances kept, 34,906 keys at the cap of 24, the fewest 5.
  Arguments: 339,577 constants, 82,421 the value itself, 9,148 an XMM number, 1,059 negations, 627 bit fields (a
  shuffle's selectors), 318 masked values, 253 sixteen times an XMM number, 178 keyword arguments, 34 masked counts, 23
  clamped counts, 17 runtime symbols, 22 others named by an expression; 0 `other`.
- **C:** 988,840 fresh instances rebuilt from the table, 0 differ; the record's 1,186,511 distinct compile calls
  rebuilt from the table, 0 differ, 0 without a stencil.
- **How it got there.** The first run judged its stencils against instances of the same representative, which share
  its unchanged fields, and passed 98.2% of them while the record's own calls differed on 133,748 of 1,186,511: every
  argument whose field no instance changed had become a constant. The judge then took the key's recorded calls at odd
  positions and derivation the even ones, and each cause the record showed was fixed in the instance maker, not in
  the table:
  - a field changed per try in a fixed cycle, with a quota of kept instances per kind of change;
  - displacements moved by multiples of 16, 32 and 64 and immediates by their low byte's bits, by 32 and 64 and by
    higher bytes, since every value's class holds its divisibility by the access sizes and its low 5 and 6 bits;
  - registers renamed in every place that names them at once (equal registers stay equal), a register's number in an
    opcode's low bits changed, a REX byte added (the start moved back one, so the next address stays), a rename paired
    with a REX byte whose R, X and B agree;
  - disp8 and disp32 swapped (the key does not hold the displacement's width);
  - a longer round of value changes where a value never varied or the key has an immediate.
  Three hole expressions were added to `stencil_census.EXPRESSIONS`: a shift count masked to 5 or 6 bits (SHL, SHR and
  SAR's CF bit; ordered before the unmasked forms, which agree with it below 32 or 64), PSRAD's count clamped to 32
  and PSRAW's to 15; 2-bit fields at even positions go before 1-bit ones (a selector's 1-bit field fits whenever its
  other bit never varied).
- **Refinement.** 58 keys first gave two sequences over their instances (the Decision's revisit condition): an SSE
  shift's count at or past the lane width compiles to a zeroing MOVI, and CMPPS's predicate picks the comparison by its
  string argument (`eq`, `gt`, `ge`), which the first signature left out. A key whose instances split is now refined by
  the first feature of one decoded value that separates them: at least 8, 16, 32 or 64 (52 keys), the low 3 bits
  (19). The C side computes the feature from a per-key entry.
- **Residual:** 4,910 keys keep a value that never varied after the longer round: 7,233 values, of which 6,727 are 0
  (the zero class holds no other value), 144 are 1 or -1, and 362 are other values, most in TEST (102), CMP (52), ADD
  (50) and OR (48) immediates and Jcc displacements. For those 506 values a constant in the table may be a hole the
  instances could not show; the record and the judge find none.
- **Closed** on 1bb36655 (CI run 38683458204, success). `uv run pytest`: 2,428 passed.
