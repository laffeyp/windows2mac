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

## result

- **The layout, asserted:** `layout.rs` asserts at compile time, under `RUNTIME_IMAGE_VA`, that no 4 GB boundary falls
  inside the image, its halfword 2 is nonzero and halfword 1 of its first address is nonzero; it holds for
  0x7f0004170000 + 8 MB. `c_words`'s zero-halfword test holds only the 10-of-24 count, its doc naming it the C
  stencil's behaviour at an address the layout rules out; the symbol list moved by `git mv` to `store/superseded/`
  with a note, the Decision "Sequence after Sprint 185" amended.
- **VEX's paths:** the `defer_fp` plant (`self.defer_fp = false`, each half checked by itself) failed the case runner
  in 27 difference classes, "259 VHADDPD MXCSR: case 20306: [c4, 01, 7d, 7c, 6c, 85, 3b] vm 0x1f8d, cpu 0x1fad", and
  "2 VADDPD fault code: case 13230: [c5, ed, 58, 4f, ee] vm 0xc000008f, cpu 0xc0000090": the case runner judges it, and
  `model_steps.exe` needs no case. The store plant (16 bytes written) passed the case runner, 0 difference classes in
  sets 049 and 049b: no lowered instruction reaches the branch. `a_vex_256_store_writes_all_32_bytes` steps
  `vmovups [rax], ymm0` directly and holds the 32 bytes.
- **`recon_lookup` bounded:** it takes the table's capacity and returns an error naming a count past it or past the
  memory; `x86_address_at` returns that error, and the run loop stops at `Stop::Exit` with it.
  `a_count_past_the_table_is_an_error` holds it; census row 15a names it.
- **Plant 1** (`RUNTIME_IMAGE_VA` at MONITOR_VA + 0x1_0000_0000, a 4 GB boundary): the build failed, "error[E0080]:
  evaluation panicked: a runtime image address with a zero halfword 1 or 2: the C stencils' branch past a runtime call
  would be wrong". Restored byte for byte.
- **Plant 2** (the 256-bit store writing 16 bytes): the case runner passed; `a_vex_256_store_writes_all_32_bytes`
  failed, "left: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0]". Restored byte for byte.
- **Plant 3** (the bound dropped, the slice's end taken as `table + 16 * n`): `a_count_past_the_table_is_an_error`
  failed, "panicked at crates/w2m_vm/src/run.rs:170:27". Restored byte for byte.
- **Sprint 187's tree check, fixed at this close:** after the first work commit, c9c4d54bf, `signal_report --check`
  failed: "git rm -r -q --cached --ignore-unmatch store/gates: error: the following file has staged content different
  from both the file and the HEAD". The removal on the temporary index is forced (`-f`), and
  `a_gates_run_must_have_tested_the_work_commits_trees` commits the passes file and a later line, as closes do; with
  the force dropped it fails with git's refusal quoted above. The gates ran again on the fixed tree, and a second
  "Sprint 188:" commit carries the fix.
- **Departures:** plant 1 moved `RUNTIME_IMAGE_VA` rather than `RUNTIME_IMAGE`, whose sum the stack's assertion pins;
  no VEX.256 #XM was added to the guest, since the case runner judges the deferred check. Every multi-line edit used
  the Edit tool.
