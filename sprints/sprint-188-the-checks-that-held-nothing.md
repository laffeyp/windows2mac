# Sprint 188 — The checks that held nothing

```yaml
---
id: 188
status: active
phase: 6
pass_kind: functional
---
```

## scope

reviewer-2's Medium 1, Medium 5 and Low 4 of its review of Sprints 182-184 (the Decision "Sequence after Sprint
185"): three checks whose input could not produce the defect they name, and one lookup a guest can crash.

- **The layout, asserted** (Medium 1): `a64_mov_imm` (`tools/vm_compiler/a64_encoder.c` 25-32) emits halfword 0 always
  and skips a zero halfword 1, 2 or 3, so a runtime call's address loads in three words, as the C stencil's fixed
  branch expects, when halfwords 1 and 2 are nonzero for every address of the image. `layout.rs` asserts that at
  compile time for `RUNTIME_IMAGE_VA .. + RUNTIME_IMAGE_SIZE` (no 4 GB boundary inside it, halfword 2 nonzero, halfword
  1 of its first address nonzero). `c_words`'s zero-halfword test stops holding the image's symbol list, which a
  Decision retires; `store/census/runtime-symbols-zero-halfword-2026-10-28.tsv` stays, marked retired. The 10-of-24
  count stays, labelled as the C stencil's behaviour at an address the layout rules out.
- **VEX's unjudged paths** (Medium 5): `defer_fp` planted away (each half checked by itself) and the 256-bit store
  planted to write 16 bytes, each run against the case runner and quoted. The store branch is reached by no lowered
  instruction: every VEX move with a 256-bit store form is in `VEX_NATIVE_256` (`lower.rs` 2608-2645), in the Python's
  `_vex.py` as in the Rust; a model unit test steps a VMOVUPS ymm store directly and holds its 32 bytes. A `defer_fp`
  plant the case runner passes adds a VEX.256 #XM to `model_steps.exe` (VADDPS ymm overflowing under an unmasked
  overflow, caught by the vectored handler), with the fixture re-taken from the frozen host.
- **`recon_lookup` bounded** (Low 4): the count read from guest memory is bounded by `RECON_ENTRIES` and the memory's
  length; a count past either is a named error the run loop returns as `Stop::Exit`, not a panic; census row 15a.

## prerequisites

- 187 (closed)

## context_files

- `crates/guest_runtime/src/layout.rs`, `tools/vm_compiler/a64_encoder.c` 20-36, `crates/translator/tests/c_words.rs`
  (the zero-halfword test), `crates/w2m_vm/src/model.rs` (`vex`, `fp_check`), `crates/translator/src/lower.rs`
  2585-2680, `src/w2m/runtime.py` 732-786, `crates/w2m_vm/src/run.rs` (`recon_lookup`), `tests/data/timed_waits/
  model_steps.c`

## signal contract

### Emits

- None: no producer runs; the judges are tests, the case runner and the run-loop judge.

### Invariants

- No Python file added or changed; every count the judges held before holds as before.

## artifact contract

### Files modified

- `crates/guest_runtime/src/layout.rs`, `crates/translator/tests/c_words.rs`, `crates/w2m_vm/src/model.rs` (a unit
  test), `crates/w2m_vm/src/run.rs`, possibly `tests/data/timed_waits/model_steps.c` and `.exe` with its fixture pin,
  `docs/step-4-census-2026-10-28.md`, `WORKING_AGREEMENT.md`

### Command exit codes

- `c_words`, the case runner, the run-loop judges, the model's and `run.rs`'s unit tests and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the layout assertion holds for the image at 0x7f0004170000; the case runner's counts unchanged;
  each VEX plant's case-runner outcome quoted, a pass marking its path unjudged there and judged by the added test or
  guest case.
- **Planted faults:** (1) `RUNTIME_IMAGE` moved onto a 4 GB boundary: the build fails, quoting the assertion; (2) the
  256-bit store writing 16 bytes: the model's unit test fails; (3) `recon_lookup` given a count past `RECON_ENTRIES`:
  its unit test names the error, not a panic.

## done criteria

Each check holds a defect its input can produce, and each VEX path the model steps has a judge.

## self-review checklist

- [x] Scope names one verifiable property: each check reaches its defect.
- [x] context_files cite the code each check lives in.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the checks that held nothing.
