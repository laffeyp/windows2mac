# Sprint 089 — the compiler from the rules, first link: each rule's ops as a template, by running it on symbols

```yaml
---
id: 089
status: active
phase: 5
pass_kind: architecture
---
```

## scope

Decision 2026-10-16 (the order; the compiler as per-op lowering from symbolic templates). The chain:

1. **This sprint: templates.** A rule (`src/w2m/rules`) turns one decoded instruction into lifted ops. Run on symbols
   instead of numbers, it gives the ops for every instruction of its shape at once: a template whose holes (register
   numbers, displacement, immediate, the next address, a RIP-relative target) are known by construction, because the
   symbols carry them through. `src/w2m/symbolic_lift.py`: symbolic operands (a register's number, a displacement, an
   immediate, the next address) as values that record what is done to them; a rule that tests one in a branch makes
   the template split on that test's outcome (a key condition), named in the output. Templates per (spec row, operand
   shape, mode); the mode is 64-bit only until the decoder has a 32-bit mode, but it is in the key.
2. The back end in C: `compile_insn`'s lowering, op kind by op kind, taking value-dependent choices (12-bit
   immediates, folded addresses, live flags) on the real values at compile time; judged by word equality with
   `compile_insn`.
3. The C lifter in the VM: decode, find the template, fill its holes; then the VM compiles from it.
4. The class runs through the C path in the VM, against Intel's recorded results (the judge independent of both
   implementations).

This sprint's judge: for every instruction of the instruction census's samples and of the spec-row encodings, the
template filled with that instruction's values equals `lift()`'s ops exactly; and coverage: the spec rows and shapes
templated, of those `lift()` takes.

## prerequisites

- 088 (closed on CI)

## context_files

- `src/w2m/lift.py`, `src/w2m/rules/*.py` (the shared `_int`, `_sse`, `_vex`, `_mmx`, `x87`), `src/w2m/x86ops.py`,
  `specs/x86_decode.json`, `store/census/static/instructions-*.json`, BLACKBOARD Decision 2026-10-16 (the order)

## signal contract

### Emits

- None (a library and data; no run).

### Invariants

- Vocabulary unchanged. `lift()` unchanged: the templates are judged against it.

## artifact contract

- `src/w2m/symbolic_lift.py`, `store/compiler/templates.json.gz` (or a generated module), `tests/test_symbolic_lift.py`
- `scripts/gates.sh` exit 0; CI `success`

## observation contract

- Templates made, by spec row and shape; rows `lift()` takes and no template covers, with the reason.
- The equality judge's counts: equal, differing (0), not templated.
- The rules that branch on a value, and on which.

## done criteria

Every rule that `lift()` runs has templates whose filled ops equal `lift()`'s on every judged instruction, and the
branches on values are named.
