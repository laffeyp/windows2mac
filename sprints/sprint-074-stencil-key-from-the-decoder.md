# Sprint 074 — the stencil key from the decoder

```yaml
---
id: 074
status: open
phase: 5
pass_kind: functional
---
```

## scope

Recentering step 6, under Decision 2026-10-14. Sprint 071 keyed its stencils on the lifted ops (the rules' output)
with registers and values taken out. The compiler inside the VM has no rules: it has the instruction's bytes, the
generated decoder's fields (`x86dec.Fields`: prefixes, REX, encoding, opcode, ModRM, SIB, displacement, immediate,
operand and address size, the spec row) and the compile call's flag arguments. Before the C decoder is generated, this
sprint measures whether a key made from those fields picks one encoder sequence, and whether every hole is a function
of the decoded fields and the instruction's address.

- **A. The record.** A cold batch run to 100 billion instructions, as Sprint 071's, recording each compile call with
  the bytes `lift` decoded for it. Each recorded instruction lifted again from its bytes at its address must give the
  recorded `Lifted`.
- **B. The census by decoder key.** The key: the spec row, the fields with register numbers and values taken out
  (which registers are equal, and RSP and RIP named, kept), the value classes of Sprint 071 on the displacement and
  immediate, and the flag arguments. Per key: the encoder sequences (one: a stencil), and each hole named by the
  decoded field it equals (a register number through the guest register map, the displacement, the immediate, the
  next instruction's address, a piece or an affine function of one) or `other`.
- **C. The outcome.** The keys with more than one sequence and the `other` holes, by cause; what the C decoder must
  output, recorded in the card and, if it changes the Decision's form, as a new Decision.

## prerequisites

- 073 (closed, on CI)

## context_files

- `src/w2m/x86dec.py`, `src/w2m/x86ops.py`, `src/w2m/lift.py`, `scripts/stencil_census.py`

## signal contract

### Emits

- None new: a measurement over a scratch frontier run; no records change.

### Invariants

- Vocabulary unchanged. The compiler and runtime unchanged.

## artifact contract

- modified: `scripts/stencil_census.py` (the key by decoder fields, behind an option), with a test
- `uv run pytest` exit 0

## observation contract

- **A:** distinct compile calls, the count whose bytes lift again to the recorded `Lifted` (all required).
- **B:** keys, keys with one sequence and their share of compiled instructions, distinct sequences, holes by kind.

## done criteria

A recorded and its re-lift exact; B run; C recorded.

## observed
- **A.** A cold batch run (`--no-translation-cache`) through scratch driver `record_insn_bytes.py`, which wraps
  `compile_insn` and `lift`; it stopped on the log's 168 reference lines at 98,714,696,354 instructions (Sprint 071's
  run stopped at `--max-steps`, 100,000,000,020) with the same compile counts: 221,338 blocks, 1,186,511 distinct
  compile calls, 1,313,377 in all. Every one of the 1,183,579 lifted instructions has its bytes, none from more than
  one encoding, and each lifted again from its bytes at its address equals the recorded `Lifted` (1,183,579 of
  1,183,579).
- **B** (`scripts/stencil_census.py --decoder-key`, `decoded_form`, `implicit_registers`, `hole_expression`).
  - The first key (the spec row, iclass, operand and address size, VEX.L, the operands of `x86ops` with registers and
    values out, which registers are equal, value classes of the immediates, displacements, the next address and any
    named address, the flag arguments) gave 38,146 forms, 38,145 with one encoder sequence. The other, one MUL row
    (19 instructions), gave two: MUL RDX takes the high half into its own operand's register, and the operands
    `x86ops` lists leave out MUL's RAX and RDX. Each spec row's implicit registers (those its lifted ops name beyond
    the decoded ones, over the record) and which explicit register equals one now enter the key.
  - With them: 38,154 forms, every one with one encoder sequence (1,313,377 of 1,313,377 compiled instructions);
    1,372 distinct sequences, as by ops.
  - Holes were named first by a vote per instruction (Sprint 071's `name_of`), which left 185 `other`; they are now
    named only by one expression of one value that gives the argument in every instruction of the form (the value, a
    halfword, a 12-bit part, the value masked to 8, 16 or 32 bits, a negation, 16 times an XMM number, a width less a
    value, a value less 1, a bit field of up to 4 bits in the low byte). By decoder key: value 37,520, negated 502,
    field 150 (a shuffle immediate's selectors), affine 79, piece 35, other 0. By ops (`--refine`) under the same
    naming: value 24,606, negated 343, affine 83, other 0.
- **C.** The C decoder must give what `decoded_form` reads: the spec row, the iclass, operand and address size, VEX.L,
  the `x86ops` operands (register numbers, base, index, scale, displacement, immediate bytes and width, a branch's
  displacement) and the instruction's length; plus a table of each row's implicit registers. The record cannot show
  every hole: 17,276 forms (19,304 compiled instructions) were seen with one distinct instruction, and in the others an
  argument that did not vary was not tested. The stencil generator therefore takes the keys from the record and finds
  each key's holes by compiling instances made for it (registers and values changed within their classes), not from
  the record's variation. Recorded as Decision 2026-10-14 (the stencil key and how holes are found).
