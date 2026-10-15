# Sprint 071 — the stencil census: the compiler's form for step 6

```yaml
---
id: 071
status: closed
phase: 5
pass_kind: functional
---
```

## scope

Recentering step 6 (`docs/recentering-2026-10-14.md`): the dispatcher and the run-time compiler inside the VM,
generated from the rules. At equal work (Sprint 070's census, 100 billion instructions, warm cache) the batch run spends
39.5-40.5 s of 50.5-51.6 s inside the VM and 2.7-2.8 s on 275,785 dispatch misses (hvc 1); a cold run compiles 221,338
blocks in Python. The compiler in the VM must make the same arm64 as the Python compiler, so the class runs and the
families judge it as they judge today's.

The standard form for a compiler generated from an interpreter's definitions is copy-and-patch (Xu and Kjolstad,
"Copy-and-Patch Compilation", OOPSLA 2021, arXiv 2011.13127; CPython 3.13's JIT, PEP 744, whose stencils are built from
the same definitions as its interpreter): a library of machine-code stencils, one per operation variant, each with holes
that the compiler fills with the operation's values. Here the rules emit w2m's ops and `compile_insn` lowers them; the
lowering decides by values as well as by form (ADD/SUB immediates when a displacement fits in 4,096, a MOVZ/MOVK
sequence as long as the constant needs, constants carried between ops), so the number of stencils and the share of
instructions they cover are measured before the form is chosen.

- **A. The record.** Every `compile_insn` call of the batch run to 100 billion instructions, compiled cold: each
  distinct (instruction, exit, NZCV source, live flags, NZCV out) and how often it compiled.
- **B. The census.** Each recorded instruction compiled again with every arm64 encoder call logged: its output as a
  sequence of encoder calls and their arguments, and the words no encoder made. Instructions grouped by form (iclass,
  ops with guest registers and values taken out, the flag arguments): per form, the encoder sequences it produces
  (one: a stencil; several: the values choose the structure, and by what), the arguments that vary (the holes) and
  whether each is a guest register, an instruction value or a function of one; words outside any encoder call.
- **C. The decision.** The form of the generated compiler, from B's numbers, recorded as a BLACKBOARD Decision.

## prerequisites

- 070 (closed: CI run 38662524546)

## context_files

- `src/w2m/vm_backend.py` (`compile_insn`, `_compile`, `Assembler`), `src/w2m/a64_encoder.py`, `src/w2m/lift.py`,
  `src/w2m/rules/`

## signal contract

### Emits

- None new: the census is a measurement script over a scratch frontier run; no records change.

### Invariants

- Vocabulary unchanged. The compiler unchanged: every recorded instruction compiled again with the encoder logged gives
  the words it gave in the run.

## artifact contract

- new: `scripts/stencil_census.py` (B), with a test that the logged encoder calls and raw words rebuild each
  instruction's words exactly
- `uv run pytest` exit 0

## observation contract

- **B's numbers:** distinct instructions and forms; forms with one encoder sequence and their share of compiled
  instructions; the forms whose structure follows their values, by cause; words outside any encoder call.

## done criteria

A recorded; B run over A with its rebuild check exact; C recorded as a Decision.

## observed
- **A.** A cold batch run to 100,000,000,020 instructions (`--no-translation-cache`) through a scratch driver that
  wraps `compile_insn`: 1,186,511 distinct calls, 1,313,377 in all (221,338 blocks compiled, 34,495 from the in-run
  translation cache).
- **B** (`scripts/stencil_census.py`, `tests/test_stencil_census.py`). Every recorded instruction compiled again
  rebuilds exactly from its logged encoder calls, the words no encoder made and its branch words (1,313,377 of
  1,313,377). Two instrument faults came first and were fixed before any number was kept: a fixed compile base out of
  B's 128 MB range of the exit (222,462 assertion failures), and encoder words with bit 31 cleared for the 32-bit form
  read as raw words (now their own kind, `enc32`).
  - By form alone (iclass, ops with registers and values out, flag arguments): 5,567 forms, 4,718 with one encoder
    sequence, covering 59.1% of compiled instructions. 849 forms take two to four sequences; nearly all are memory
    operands of base plus displacement (MOV 332,684 instructions, LEA 61,007), where the displacement's range picks
    LDR's scaled offset, LDUR's 9 bits, an ADD/SUB immediate or a MOVZ/MOVK sequence.
  - With value classes (zero, sign, the 12-bit and 9-bit ranges, the scaled offset per access size, nonzero halfwords,
    a shift count's 1 and 0) and which registers are equal: 18,218 forms, every one with one encoder sequence. The
    first pass without the shift count's classes left 6 forms (72 SHR, SHL and SAR instructions), where a count of 1
    defines OF.
  - 1,287 distinct encoder sequences; the commonest 100 cover 93.2% of compiled instructions, the commonest 1,000
    all but a rounding of 100%.
  - Holes in one-sequence forms: 20,709 + 1,112 equal a register or value, 1,243 + 93 a 16-bit or 12-bit piece, 3 a
    negation, 70 other. All 70 were checked, up to three instructions each (200 values): 165 are 16 times an XMM
    number (the YMM upper halves' STP), 17 are 64 or 32 less a shift count and 18 a shift count less 1 (UBFX for SHL,
    SHR and SAR's CF); none is unexplained. The Decision's grounds name the first two rules only. 1,833 hole
    positions sit in raw words.
  - Raw words: 394,155 in 364,521 instructions, built inline in `vm_backend.py` (ADD/SUB immediates at
    `compile_insn`'s computed addresses and `ADD_SUB_IMM`, the flag sequences of `_nzcv`).
- **C.** BLACKBOARD Decision 2026-10-14, the form of the compiler inside the VM: copy-and-patch at the encoder level,
  stencils captured from the Python compiler, a form with no stencil compiled by the host.
