# Sprint 089 — the compiler from the rules, first link: each rule's ops as a template, by running it on symbols

```yaml
---
id: 089
status: closed
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

## observed
- **Templates** (`src/w2m/symbolic_lift.py`): a rule run on symbols (register numbers, base, index, scale,
  displacement, immediate, the next address, the instruction's address) gives ops whose numbers are expressions of
  them. The rules are unchanged: `_operand`, `lift_mmx`'s MMX register slice and the immediate parsing in `_sse`,
  `_vex` and `lift` are replaced while a symbolic run lasts. 5,504 templates over 4,648 (class, shape, mode) keys in
  460 classes.
- **Judge** (`scripts/template_judge.py`, 52 s): filled templates against `lift()`: the batch run's 1,183,579
  compiled instructions all equal; spec-row encodings 6,887 equal (3,783 lift() refuses); the class case sets'
  250,840 all equal; 0 differing. In CI (`tests/test_symbolic_lift.py`): the stencil samples and two encodings per
  spec row, with a plant (the immediate's sign extension dropped) that differs.
- **Branches on values:** 34 conditions in 19 classes, 33 of them an immediate's sign bit (lift._operand's sign
  extension) and one a register-equality test; no symbol had to be pinned to its value. Decoder fields the rules
  read, in the keys: the operand template (790 templates), opcode (734), ModRM (704), VEX.L (588), prefixes (87),
  operand size (17).
- **Defects of my own found by the judge:** MMX register numbers baked into templates (`int(o[3:])` in lift_mmx); a
  shape that missed `_operand`'s test on the base register's name (EAX and R13D share a width but not an address
  size). Both fixed; the record judge never saw the second, the spec rows and class cases did.
- **A suspected lift() defect, not measured:** under 0x67 with a base of R8D-R15D, `_operand` gives a 64-bit address
  (its test is the name's first letter), where the SDM's address-size rule (Vol 1 3.3.7) computes it in 32 bits. The
  class case sets hold 11,202 cases that address through R8D-R15D; 944 of them have the base's upper half nonzero,
  and their classes are accepted against Intel's results, which counts against the suspicion without settling it (a
  case where both addresses fault, or reach the same bytes, would agree either way). I first wrote that every such
  case had a zero upper half, from one case examined; the count corrects it. Recorded in Open questions.
- **Not done here:** the templates' serialized form, which the C back end (next link) fixes by what it reads.
- **088's census and its own commit:** the census wrote the commit of its script's last change, so output written
  before the script's commit names the previous one, and the reproduction test failed locally once 088 was committed
  (CI skips depot tests, so 088's CI passed). The census now records the script's sha256.
