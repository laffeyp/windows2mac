# Sprint 077 — the compiler in C: the key, the table as C data, the emitter

```yaml
---
id: 077
status: open
phase: 5
pass_kind: functional
---
```

## scope

Recentering step 6, under Decision 2026-10-14 (copy-and-patch at the encoder level; the stencil key). The compiler
the VM will carry, for one instruction: its bytes and address in, the arm64 words `vm_backend.compile_insn` gives out,
built from Sprint 073's encoders, Sprint 075's decoder and Sprint 076's table. Block assembly (prologue, flag liveness,
the budget) and the dispatcher stay in Python for this sprint; the C takes the flag arguments as inputs.

- **A. The key in C.** `tools/vm_compiler/vm_compiler.c` computes `stencil_census.decoded_form`'s key and values from
  the decoder's output: the row, iclass, sizes, the operands' shapes, which registers are equal, which equal the row's
  implicit registers, each value's class, the flag arguments. Both sides write the key as the same bytes
  (`scripts/gen_stencil_table.py` `key_bytes`); exit_va leaves the key (every record call has the same one, and it
  reaches the words only through branches to it, item C).
- **B. The table as C data.** `scripts/gen_stencil_table.py` renders Sprint 076's table to
  `tools/vm_compiler/stencil_table.h`: keys found by hash and checked byte for byte, each key's stencil or its
  refinement by a feature, each item's encoder and its arguments mapped to the C encoder's positional parameters
  (keyword arguments placed, defaults filled, names as enum indices, a system register as its five fields), the
  implicit registers per row, the runtime symbols' names.
- **C. The emitter.** Each argument from its constant, its hole expression over the decoded values (the C equal of
  `stencil_census.EXPRESSIONS`), a runtime symbol's address, or, for a branch to the exit, `exit_va` less the branch's
  own address, computed where the code lands; a branch inside the instruction keeps its offset. `mov_imm`'s several
  words; an enc32 item's bit 31 cleared.
- **D. The judge.** The record's distinct compile calls: the C key equal to `key_bytes` of the Python key, and the C
  words equal to `compile_insn`'s, each at a base and exit address drawn at random (not the table's), with instructions
  with no stencil counted apart. The C also builds with `guest_runtime.CFLAGS`.

## prerequisites

- 076 (closed, on CI)

## context_files

- `scripts/stencil_table.py`, `scripts/stencil_census.py`, `tools/vm_compiler/x86_decode.h`,
  `tools/vm_compiler/a64_encoder.h`, `src/w2m/vm_backend.py` (`compile_insn`, `Assembler`)

## signal contract

### Emits

- None new: C sources, a generator and tests; no run, runtime or compiler path changes.

### Invariants

- Vocabulary unchanged. The Python compiler and the runtime unchanged.

## artifact contract

- new: `tools/vm_compiler/vm_compiler.c`, `vm_compiler.h`, `stencil_table.h` (generated), `scripts/gen_stencil_table.py`,
  `tests/test_vm_compiler_c.py`
- `uv run pytest` exit 0; CI `success` on the closing commit

## observation contract

- **A:** record instructions whose C key differs from the Python key's bytes (0 required).
- **B:** keys, items, arguments by kind, the header's size.
- **C/D:** record calls compiled by the C equal to `compile_insn`'s words, differing (0 required, or each cause named),
  with no stencil; the C's time per instruction against `compile_insn`'s.

## done criteria

A to D; D with 0 differing, or each difference's cause named and recorded; CI passes.

## observed
- **B.** `scripts/gen_stencil_table.py` over Sprint 076's table (sha256:1a8f01b5…, from the record sha256:02eb7d3b…):
  38,154 keys share 3,394 distinct stencils, built from 1,763 distinct items with 6,327 arguments, 24 of them wide
  constants in a pool; 124 encoders; 3 runtime symbols (`w2m_cpuid`, `w2m_approximate`, `w2m_pcmpistri`). Arguments
  over all stencils: 351,668 constants, 93,905 holes, 6,374 offsets to the exit, 17 symbols. Every branch offset in the
  table is either an exit (within 16 KB below Sprint 076's NEAR) or inside the instruction (under 4 KB); none is
  neither. `stencil_table.h` is 7.8 MB of source; with the guest runtime's flags the object holds 7 KB of code and
  3.0 MB of tables, of which the key bytes are 1.35 MB.
- **A, C, D.** Over the record's 1,186,511 distinct compile calls (scratch driver `vmc_judge.py`, through ctypes): the
  C key equal to `key_bytes` of the Python key and the C values equal to the Python's in all, and the C words equal
  to `compile_insn`'s in all, each at a base drawn from 4 GB to 4 TB and an exit within 64 MB of it; 0 with no stencil,
  0 failing in either compiler.
- **The test** (`tests/test_vm_compiler_c.py`): `tests/data/stencil_samples.txt.gz` holds 38,229 instructions, the
  first recorded call per key and per stencil, and for the 71 refined keys, whose record reached one stencil each,
  an instance made for each of the 75 other stencils (146 in all). Every one compiles in both, key and words equal,
  in 3.3 s. Four planted faults fail it: the exit offset 4 off, a value class's m&31==1 bit read as m&31==2, the
  implicit-register mask read one bit high, the `-v` expression as `v`.
- **Time:** `compile_insn` 12.5 µs per instruction in the judge; the C 208 to 217 ns natively over the 38,229 samples
  (two runs of 50 passes, M5 Max host, unweighted by how often each key runs), 4.9 words each.
- **Not in this sprint:** the C compiles one instruction; block assembly (the prologue, flag liveness, the budget)
  and the dispatcher in the VM come next. The table is regenerated only from a record, which is a scratch file;
  `stencil_table.h` names the table's sha256.
