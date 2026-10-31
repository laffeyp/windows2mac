# Sprint 272 — A memory XCHG made atomic

```yaml
---
id: 272
status: done
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

- **The change** (05ae6da85): `lower.rs` counts an XCHG with a memory operand as locked, its load and store the
  exclusive pair with the alignment test and retry; `vm_compiler.c`'s `vmc_key` returns `VMC_NO_STENCIL` for it, by
  `x86_is_xchg` in `x86_decode.c` (`X86_ICLASS_XCHG` lives in the decoder's generated tables, which only
  `x86_decode.c` includes).
- **The judges:** `c_words` first named all 60 of the record's XCHG calls as differing (rust `ldaxr` `c85ffe59` and
  `stlxr` `c810fe5a`, c a plain `ldr` and `str`); with the C's decline, all 60 are "XCHG status -2", and the test now
  holds that the C compiles no memory XCHG. The lowering's unit test holds the pair at 64 and 32 bits and
  RIP-relative, and none for `xchg rax, rcx`. The case runner judges all 1,455 XCHG cases agreeing with Windows; 31
  misaligned ones now step in the model, `store/case_runner/expected-by-class.tsv`'s XCHG row moved from
  `1455 59 0 0 0` to `1455 59 0 31 0`, rewritten by the test's own writer.
- **The image:** `python_image.rs` names the three C files that depart from the tag, `vm_compiler.c`, `x86_decode.c`
  and `x86_decode.h`, each with its reason, and holds the frozen image as before.
- **Found on the way:** with the C changed, three `w2m_vm` tests failed together and passed alone: each built the new
  key's image at once in one shared work directory. `guest_runtime::build` now links in a directory of its own and
  renames onto `<key>.bin`; with the image removed, all 74 `w2m_vm` library tests pass in parallel.
- **Deferred:** a misaligned locked form steps in the model, atomic on one vCPU and not on two.
- **Not as the card said:** plant 2 named the whole-block judge; `c_words`, which links the C compiler itself, names
  it first and by address.
- **The gates' first pass judged only bookkeeping:** the run on 05ae6da85 chose `rdeps(signal_report)`, 36 tests, its
  base `git merge-base HEAD origin/main` being the pushed work commit itself; `origin/main`'s reflog names six passes
  so made, Sprints 267 to 272 (reviewer-3's High 1 at 272, found here first). `scripts/gates.sh --all` then failed in
  three tests: the held first stop, the record's exits against it, and the call stream at row 2535, DefWindowProcW's
  fourth argument `7f000418d304` against the frozen `7f000418d2cc`, both `create_step`, moved 0x38 bytes by the C.
- **Addresses by what they name** (abb33727c; reviewer-3's Medium 1 at 272, option (b)): `guest_runtime::AddressNames`
  reads a runtime-image address as its nearest symbol plus offset; `batch_run.rs` names the Rust stream by the image it
  ran and the frozen stream by `frozen_python_image`, so row 2535 reads `create_step+0x0` on both sides; `frozen_root`
  moved from the image judge into the library. The held first stop moves by the 56 declines: hvc 1 1091 to 1147, no
  stencil 123 to 179, cut by no stencil 143 to 191, compiled 274837 to 274830, the host's compiles 275559 to 275608.
- **The exit ledger aligned** (2b27787dc): the second full run failed on `exit_ledger.rs`, every row after the first
  extra host compile out of step. The judge now holds a Rust `hvc 1` the frozen host did not take, a run of at most 8,
  its next row meeting the frozen row, as an extra by its site: 22 in the game, 7 in its child, every other cell equal.
  A resume reading (executed, warp, process) is accepted on the matched row or an extra after it: the extra moves which
  row carries the reading (746 on the Rust row, 765 on the extra after it, against the frozen 765), a bounded looseness
  whose exact rule is not proved; a parting is still named cell by cell, held by a unit test.
- **Not as the card said:** the judges' change came into the card's scope, its own change requiring it; the card named
  none of the address-holding judges.
- **The gates:** `scripts/gates.sh --all` exit 0 in 830 s on 2b27787dc, `rust=all`, 299 tests run, 299 passed, 1 skipped, copied from `store/gates/passes.tsv`; the timing stage 651 ns an exit at load 3.41. The row of 05ae6da85, exit 0 in 268 s, `rust=crates`, judged only the bookkeeping and is not this card's pass.
- **Plant 3,** restored by `cp` and checked by `cmp`: `AddressNames::name` made to name nothing; the stream judge
  failed, "row 2535 of the call stream differs from the Python batch run's".
- **Plant 1,** restored by `cp` and checked by `cmp`: the lowering's XCHG case made to match no iclass; the test
  failed, "assertion `left == right` failed: [48, 87, 08] left: (false, false)".
- **Plant 2,** restored by `cp` and checked by `cmp`: the C's decline made `if (0 && …)`; the test failed, "the C
  compiled 60 memory XCHG call(s), which x86 locks".
