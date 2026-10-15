# Sprint 073 — the arm64 encoders in C

```yaml
---
id: 073
status: closed
phase: 5
pass_kind: functional
---
```

## scope

Recentering step 6, the second build step of Decision 2026-10-14 (the compiler inside the VM as copy-and-patch at the
encoder level). A stencil is a sequence of `a64_encoder` calls; the compiler in the VM makes them with encoders in C,
which the Decision requires to be judged against `a64_encoder.py` by a differential. The C is freestanding, built
with the guest runtime's flags, and not yet linked into the runtime image: nothing in a run changes this sprint.

- **A. The encoders.** `tools/vm_compiler/a64_encoder.c` and `.h`: one C function per public function of
  `a64_encoder.py` (139), returning the instruction word, or 0 where the Python raises (a register outside 0-31, a
  failed assertion, a size or name with no entry). Arguments named by a string in Python (a condition, a NEON or
  floating-point operation) take an index into the Python table's order. `mov_imm` writes its words and returns their
  count.
- **B. The differential.** `tests/test_a64_encoder_c.py` builds the C as a shared library for the host and calls
  every function over a domain per argument (every register 0-31 and the values just outside, every immediate's
  edges and a seeded sample between); where the Python returns a 32-bit word the C must return the same word, and
  where the Python raises the C must return 0. The C also builds with the guest runtime's flags (freestanding,
  `-Werror`) for arm64.
- **C. The record's calls.** Every distinct encoder call made while compiling Sprint 071's record (the batch run's
  1,186,511 distinct compile calls) run through the C: the count that differ (0 required).

## prerequisites

- 072 (closed: CI run 38670636259)

## context_files

- `src/w2m/a64_encoder.py`, `src/w2m/guest_runtime.py` (CFLAGS), `scripts/stencil_census.py` (`EncoderLog`)

## signal contract

### Emits

- None new: no run, compiler or runtime change.

### Invariants

- Vocabulary unchanged. The runtime image and the compiled words unchanged.

## artifact contract

- new: `tools/vm_compiler/a64_encoder.c`, `tools/vm_compiler/a64_encoder.h`, `tests/test_a64_encoder_c.py`
- `uv run pytest` exit 0; CI `success` on the closing commit

## observation contract

- **B:** calls per function and the count that differ (0 required), reported by the test.
- **C:** distinct encoder calls in the record, the count that differ (0 required).

## done criteria

A to C; B and C with 0 differing; CI passes.

## observed
- **A.** `tools/vm_compiler/a64_encoder.c` and `.h`: 139 functions, one per public function of `a64_encoder.py`; it
  builds with the guest runtime's CFLAGS (freestanding, arm64, `-Werror`) and is not linked into the runtime image.
- **B** (`tests/test_a64_encoder_c.py`, 141 tests, 3.3 s). Over 138 functions (and `mov_imm` separately):
  1,066,482 words compared and 953,795 rejections compared (the Python raised, the C returned 0); 23,098 argument
  sets where the Python returns a value outside 32 bits were not judged; 0 differ. Three faults planted in a scratch copy were each
  caught (a SUB immediate's opcode with bit 0 set: 8,413 differ; LDUR and STUR accepting an offset of 256: 21; C's
  `%` in place of Python's in BFI's rotation: 7,399).
- **C.** The record's 1,186,511 distinct compile calls compiled again with every encoder call logged at any depth:
  5,232,559 calls, 394,453 distinct, over 126 of the 139 functions; each run through the C: 0 differ. The 13 the
  compiler did not call: `adr`, `bl`, `br`, `dmb_ish`, `dup_4s`, `ldp_q`, `ret`, `shl_4s`, `sshl_4s`, `sshr_4s`,
  `stp_q`, `ushl_4s`, `ushr_4s` (B covers them).
- **Closed** on 4394114c (CI run 38677583192, success; the run found by commit). `uv run pytest`: 2,421 passed.
