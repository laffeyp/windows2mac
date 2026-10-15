# Sprint 078 — the block in C: extent, flag liveness inside the block, prologue, exits, the budget

```yaml
---
id: 078
status: open
phase: 5
pass_kind: functional
---
```

## scope

Recentering step 6, after Sprint 077. The C compiles a block as `Monitor._compile` does once the block's
instructions and the flags live at its end are known: the block's words and its marks (host address, x86 address).
The successor scan that gives the flags live at the end, the page protection, the translation cache and the
dispatcher stay in Python for this sprint; the C takes the flags live at the end as an input.

- **A. `assemble_block`.** `Monitor._compile`'s code generation becomes a function of the instructions, the base, the
  exit, the slice stub's address, the symbols and the flags live at the end (`vm_backend.assemble_block`); `_compile`
  calls it. Behavior unchanged: the class runs' tests hold.
- **B. The facts per form.** The C table is keyed by the key without its flag arguments (a form). Each form holds
  what the block needs before its flags are known: whether it ends the block (a branch), whether control falls
  through it, the flags it reads and writes for each `nzcv_from` kind (`insn_flags`), the kind of NZCV it leaves
  (`nzcv_setter`), which kinds its Jcc can branch on (`nzcv_branch`); and its stencils by flag arguments. The facts are
  derived from every recorded instruction; a form whose instructions disagree is reported.
- **C. The block in C.** `vmc_block`: the extent (at most `MAX_BLOCK_INSNS`, up to and including a branch, before a
  thunk, before bytes that do not decode); the flags live after each instruction, walked backward; each instruction
  compiled with `nzcv_from`, its flags live after and `nzcv_out`; the fall-through exit, the budget's SUBS and its stop
  (BL to the slice stub, HVC #4, back to the body). An instruction with no stencil for its flag arguments ends the
  block before it, with every flag live at the new end; a block whose first instruction has none goes to the host.
- **D. The judge.** Blocks made from the record (each recorded address a start, its recorded successors on), each with
  the flags live at its end drawn from those its last instruction was recorded with: the C's words and marks equal to
  `assemble_block`'s over the instructions the C took, at a random base, exit and slice stub; blocks the C cut short
  counted, by cause.

## prerequisites

- 077 (closed, on CI)

## context_files

- `src/w2m/vm_backend.py` (`Monitor._compile`, `compile_insn`, `insn_flags`, `nzcv_setter`, `nzcv_branch`),
  `tools/vm_compiler/vm_compiler.c`, `scripts/gen_stencil_table.py`

## signal contract

### Emits

- None new. `_compile` emits what it did; the C is not yet run in the VM.

### Invariants

- Vocabulary unchanged. `_compile`'s words unchanged (A is a refactor).

## artifact contract

- `vm_backend.assemble_block`; `vmc_block` in `tools/vm_compiler/vm_compiler.c`; the table re-keyed with its facts;
  `tests/data/block_samples.txt.gz`; tests in `tests/test_vm_compiler_c.py`
- `uv run pytest` exit 0; CI `success` on the closing commit

## observation contract

- **B:** forms, forms whose recorded instructions disagree on a fact (0 required, or refined), the header's size.
- **D:** blocks judged, equal, differing (0 required, or each cause named), cut short by cause; the C's time per
  block and per instruction.

## done criteria

A to D; D with 0 differing, or each difference's cause named and recorded; CI passes.

## observed
- **A.** `vm_backend.assemble_block` holds `_compile`'s code generation unchanged; `_compile` keeps the extent, the
  page protection, the successor scan, the cache and the bookkeeping, and passes the census's switch through.
- **B.** 17,328 forms over Sprint 076's 38,154 keys (38,154 flag variants). Facts derived from every recorded
  instruction whose bytes lift to its recorded Lifted: 0 forms whose instructions disagree. Keyed by form, the key bytes
  fall from 1.35 MB to 572 KB and `stencil_table.h` from 7.8 MB to 5.9 MB; with the guest runtime's flags the object
  holds 8.8 KB of code and 2.65 MB of tables.
- **D.** Every recorded address taken as a block start (scratch driver `block_judge.py`; 1,177,340 blocks, each walked
  through its recorded successors, the flags live at its end drawn from those its last instruction was recorded with,
  base, exit and slice stub at random): 1,163,021 blocks compiled by the C, words and marks equal to
  `assemble_block`'s, 0 differing; 8,481,859 instructions in them. 14,319 blocks went to the host and 3,523 were cut
  short, all with status `VMC_NO_STENCIL` at an instruction whose form is in the table: the flag arguments the block
  gives it match none its recorded calls had. Which of these starts the run itself used as block starts is not
  measured (the record holds compile calls, not blocks).
- **The test:** `tests/data/block_samples.txt.gz`, 3,000 of those blocks (seeded), compiled in both, equal; a thunk
  ends a block before it. Three planted faults fail it: the liveness walk reading the instruction's own NZCV kind for
  its predecessor's, the budget's B.MI one word short, `nzcv_out` lost for the last pair. The generator's gzip files
  carry no timestamp: two runs give the same bytes.
- **Time** (native, M5 Max host, 3,000 sample blocks, two runs of 200 passes): 1,288 to 1,325 ns per block, 176 to
  181 ns per instruction, 7.3 instructions and 32.6 words a block; `assemble_block` 78.9 µs per block in the judge.
- **Open, for the VM:** the table covers the flag arguments the record compiled. In the VM a block starts where the
  run's blocks start, so the record's flag contexts recur; a context it never saw goes to the host until a stencil
  for it is derived (a superset of live flags is correct code, but not `compile_insn`'s words).
