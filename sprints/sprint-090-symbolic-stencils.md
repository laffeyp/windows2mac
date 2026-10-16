# Sprint 090 — the compiler from the rules, second link: compile_insn run on the templates gives the stencils

```yaml
---
id: 090
status: active
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
