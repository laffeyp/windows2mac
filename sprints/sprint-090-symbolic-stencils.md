# Sprint 090 — the compiler from the rules, second link: compile_insn run on the templates gives the stencils

```yaml
---
id: 090
status: closed-partial
phase: 5
pass_kind: architecture
---
```

## scope

Decision 2026-10-16 (the back end by symbolic execution of compile_insn, amending the order's point 2). Sprint 089's
templates hold each rule's ops with symbolic holes. Here `vm_backend.compile_insn` runs on them: the Assembler and the
a64 encoders receive the holes as symbols, so each word comes out as an expression of the holes (a stencil whose holes
are known by construction, as copy-and-patch's are), and every value-dependent choice compile_insn or an encoder makes
(a displacement within 4,096, mov_imm's nonzero halfwords, a live flag) forks the run on that exact condition.

- **Domains.** Each hole has the range its operand allows (a register number 0-15, a scale 1, 2, 4 or 8, a 32-bit
  displacement, an immediate of its width, an address); a test the domains decide is answered without a fork, so
  the encoders' register checks do not multiply paths.
- **Flag contexts.** `live_after` is a symbol of six bits and `nzcv_out` a symbol of one; `nzcv_from` (six kinds)
  stays a key. A stencil forks only on the bits it tests.
- **Judge.** For every instruction of Sprint 089's three sources at the flag contexts the record holds and the two
  defaults: the stencil whose conditions its values meet, evaluated, equals compile_insn's words exactly.
- **Measure.** Stencils per template; total; the size their table would take in C; time to make them.

## prerequisites

- 089 (closed on CI)

## context_files

- `src/w2m/symbolic_lift.py`, `src/w2m/vm_backend.py` (`compile_insn`, `Assembler`, `_set_flags`, `_operand_forms`,
  `_folded_addresses`, `_mem`), `src/w2m/a64_encoder.py`, `src/w2m/rules/_sse.py`, `_vex.py`, `_mmx.py` (`lower`)

## signal contract

### Emits

- None (a library; no run).

### Invariants

- Vocabulary unchanged. compile_insn unchanged: the stencils are judged against it.

## artifact contract

- `src/w2m/symbolic_compile.py`, `scripts/stencil_from_rules_judge.py`, `tests/test_symbolic_compile.py`
- `scripts/gates.sh` exit 0; CI `success`

## observation contract

- The judge's counts (equal, differing 0, refused), the stencil counts and size, the conditions by kind.

## done criteria

compile_insn, run on every template, gives stencils whose evaluated words equal its own on every judged instruction.

## observed
- **Exact.** `src/w2m/symbolic_compile.py` runs compile_insn (and the rules' lowerings) on Sprint 089's templates,
  its own arguments symbolic (`base`, `exit`, the runtime's three symbols, `live` six bits, `nzcv_out`); every word
  an expression of the holes. `scripts/stencil_from_rules_judge.py`, 1,000 of the record's instructions at random
  flag contexts, bases and exits: 996 equal to compile_insn's words, 0 differing, 4 with no stencil. The four are SSE
  (CVTSS2SD, MULPS, MULSS, SHUFPS): their lowerings read the XMM number as a plain integer (`int(dst[1])`), which
  pins it to the first instance's register; a stencil table would enumerate the 16 values. In CI
  (`tests/test_symbolic_compile.py`): a move, a load through base and index, ADD of an immediate, a call and a Jcc at
  three flag contexts, all equal.
- **Fixes on the way.** A set compared by hash (`_folded_addresses`: `sizes == {scale}`) cannot see a symbol and took
  the wrong path silently (6 differing of 3,000 in the first run): the scale is in the template's shape now. Forks that
  decide nothing were cut: an expression against itself, a number against None, a test the path took (or its
  negation), equalities in one order, mask tests the path's known bits decide, and tests the symbols' domains decide
  (interval arithmetic).
- **Not done: the count.** Run whole, compile_insn's choices multiply. ADD of an immediate: 98 stencils per NZCV kind
  (five live-flag bits tested one by one, one in a group of two, register equality, nzcv_out); a load through base
  and index: 23 (the displacement against 0 and 4,096 and its halfwords); an instruction with both, their product.
  1,000 instructions made 363,546 stencils over 370 (template, NZCV kind) pairs in 441 s; over 5,504 templates and six
  kinds that is tens of millions and hours. This is the revisit condition of Decision 2026-10-16 (the back end by
  symbolic execution): the next link splits compile_insn's control (register allocation, liveness, fusion) from its
  per-op lowering bodies, generating the bodies as stencils keyed by their own arguments, so the forks add.
