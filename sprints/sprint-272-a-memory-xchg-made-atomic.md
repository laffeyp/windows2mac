# Sprint 272 — A memory XCHG made atomic

```yaml
---
id: 272
status: active
phase: 6
pass_kind: functional
---
```

## scope

Census row D1 (`docs/step-6-census-2026-10-30.md`), the first of the Decision "Sequence for step 6": x86 locks an XCHG
with a memory operand without a LOCK prefix (Intel SDM vol. 2, XCHG, as https://www.felixcloutier.com/x86/xchg renders it: "If
a memory operand is referenced, the processor's locking protocol is automatically implemented for the duration of the
exchange operation, regardless of the presence or absence of the LOCK prefix or of the value of the IOPL"), and both of w2m's compilers lower it as a load
and a plain store, so a second vCPU can write between them. This card makes it atomic in the one lowering that already
knows how: the Rust translator's exclusive pair for a `_LOCK` iclass (`crates/translator/src/lower.rs` 1289-1450); the
C compiler inside the VM declines the form, so its blocks go to the host.

- **Where it stands, by lookup (2026-10-30):**
  - `rules.rs` 629-644: `xchg` stores the memory operand first, then the register, so the load-exclusive falls at the
    memory load and the store-exclusive at the memory store with no guest effect between (the commit rule).
  - `lower.rs` 1290: `locked` is `ins.iclass.ends_with("_LOCK")`; XCHG's iclass is `XCHG`.
  - `store/stencil/table.txt.gz`: 25 memory XCHG forms with stencils, which `tools/vm_compiler/stencil_table.h` holds.
  - `vm_compiler.c` 108-140: `vmc_key` holds the decoded iclass (`d.iclass`, `X86_ICLASS_XCHG` 550) and each operand's
    kind (`X86_OP_M`); `vmc_decode` (236-248) returns its status, `VMC_NO_STENCIL` ending the block for the host.
  - The image judge holds the tag's C since Sprint 271; `the_working_trees_c_is_the_frozen_c` holds the working tree's.
- **The work:**
  - `lower.rs`: a memory XCHG lowers as a locked form.
  - `vm_compiler.c`: `vmc_key` returns `VMC_NO_STENCIL` for XCHG with a memory operand.
  - `the_working_trees_c_is_the_frozen_c` states the departure, the file and the reason.
  - Tests: the lowering of a memory XCHG holds `ldaxr`, `stlxr` and the retry; a register XCHG lowers as before; the C
    compiler declines the 25 forms (a stencil sample of each).
  - The batch run conforms; its compile counts on the record.
- **Not here:** two vCPUs (273).

## prerequisites

- 271 (closed)

## context_files

- `crates/translator/src/rules.rs` (`xchg`), `src/lower.rs` (the locked path); `tools/vm_compiler/vm_compiler.c`
  (`vmc_key`, `vmc_decode`); `crates/guest_runtime/tests/python_image.rs`; `docs/step-6-census-2026-10-30.md` (D1)

## signal contract

### Emits

- None; a lowering and a decline, held by tests and the whole-block judge.

### Invariants

- A register XCHG's words unchanged; every other form's words, in both compilers, unchanged; the frozen image judge
  passing.

## artifact contract

### Files modified

- `crates/translator/src/lower.rs`, `tools/vm_compiler/vm_compiler.c`, `crates/guest_runtime/tests/python_image.rs`,
  tests, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the memory XCHG's words with the exclusive pair; the batch run conforming, its "no stencil"
  and host compiles beside the last record's.
- **Planted faults:**
  - (1) the lowering's XCHG case removed: the lowering test names the missing `ldaxr`.
  - (2) the C compiler's decline removed: the whole-block judge names an in-VM block whose words differ from the
    Rust translator's, or, if the batch run reaches no memory XCHG in the VM, the card says so and names the test that
    holds the decline.

## done criteria

A memory XCHG is one atomic exchange in the code the host runs it with, and the C compiler leaves it to the host;
nothing else in either compiler changes.

## self-review checklist

- [x] Scope names one verifiable property: the memory XCHG atomic.
- [x] context_files cite both compilers, the judge and the census.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: the XCHG's atomicity.

## result
