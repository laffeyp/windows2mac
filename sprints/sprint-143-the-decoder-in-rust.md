# Sprint 143 — The x86 decoder in Rust

```yaml
---
id: 143
status: active
phase: 6
pass_kind: functional
---
```

## scope

Link 2 of `docs/design-host-compile-path-2026-10-24.md`: the translator's first part. A crate, `x86_decoder`, carries
`src/w2m/x86dec.py` (287 lines: `Fields`, `Decoded`, the decode errors, `_pick`, `decode`) on tables generated in Rust
from `specs/x86_decode.json`. A generator binary in the style of Sprint 126's `vocabulary_generator` writes them, with
a `--check` mode for the gates, carrying `scripts/gen_decoder.py`'s rules (272 lines). The Python generator stays as
it is, frozen.

Its judges:

- `tests/test_x86dec.py`'s values, carried by hand and cited by line.
- A differential against `tools/vm_compiler/x86_decode.c`, the decoder the VM runs, built for the host by clang. It
  covers every instruction of `store/stencil/record.txt.gz`'s 1,186,512 compile calls and every instruction reached
  linearly from the `.pdata` function starts of the executables under `store/binaries/`. Length, iclass and fields
  must be equal, and so must the refusals.

The same build measures what the design's revisit condition needs: for how many of the record's calls the C emitter
(`vmc_decode`, then `vmc_emit` with the call's flag arguments) gives words.

## prerequisites

- 142 (closed)

## context_files

- `src/w2m/x86dec.py`, `scripts/gen_decoder.py`, `tests/test_x86dec.py` (read in full at the sprint's start)
- `tools/vm_compiler/x86_decode.c`, `x86_decode.h`, `vm_compiler.c` 190-290 (`vmc_key`, `lookup`, `vmc_decode`,
  `vmc_emit`); `specs/x86_decode.json`
- `crates/vocabulary_generator` (the generator's pattern and its `--check`)

## signal contract

### Emits

None. A decoder; it puts nothing on a record.

### Consumes

Nothing at run time.

### Invariants

- No Python file added or changed; no Python run or written.

## artifact contract

### Files created

- `crates/x86_decoder/` (the decoder, the generated tables, the generator, the tests)

### Files modified

- `Cargo.toml`, `Cargo.lock`, `scripts/gates.sh` (the generator's `--check`), `.github/workflows/` (the same, if the
  gates' list is mirrored there), `WORKING_AGREEMENT.md`, `BLACKBOARD.md` (Sprint 142's CI run, in the tail entry)

### Content assertions

- Each item cites the `x86dec.py` or `gen_decoder.py` lines it carries.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- the generator's `--check` returns 0; `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `tests/test_x86dec.py`'s cases; the record's 1,186,512 instructions and the `.pdata` sweep equal
  to the C decoder's, the counts reported; the C emitter's coverage of the record reported as a count of 1,186,512 and
  the rule files those calls reach.
- **Planted fault:** one table row's operand size changed in the generated Rust: the differential names the first
  instruction that differs, with both decodings.

## done criteria

x86 code is decoded from Rust as the VM's C decoder decodes it, over every instruction the batch run compiled.

## self-review checklist

- [x] Scope names one verifiable property: the Rust decoder equals the VM's C decoder on the record and the sweep.
- [x] context_files cite the Python and C carried.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: expected values and a planted fault.
- [x] One concept: the decoder.

## result

- `crates/x86_decoder`: `decode` carries `x86dec.py`'s `_decode` and `_pick`, each block citing its lines; `build.rs`
  carries `gen_decoder.py`'s `build_table` and the Python tables' constants, generating `ROWS`, `INDEX`, `LOCKABLE`,
  `REP_STRING`, `REPE_STRING` and `LEGACY_PREFIX_OPCODES` from `specs/x86_decode.json` at every build. Each row's
  identity is the SHA-256 of Python's `json.dumps(row, sort_keys=True)`, written by a serializer that keeps Python's
  separators and `\uXXXX` escapes (four rows hold non-ASCII text).
- Deviation from the card: no generator binary and no committed tables, so no `--check` in the gates. `build.rs`
  regenerates the tables from the spec at every build, so they cannot go stale. `scripts/gates.sh` is unchanged; the
  frozen `gen_decoder.py --check` still holds the Python and C tables.
- Expected values: `tests/known.rs` carries `tests/test_x86dec.py`'s cases, 8 tests citing their lines; its Capstone
  walk and XED differential are Python tools, and the C differential stands in their place.
  `tests/c_differential.rs` builds `x86_decode.c`, `vm_compiler.c` and `a64_encoder.c` with `xcrun clang` and calls
  them through `dlopen`, with no new C. All 1,186,511 record instructions and 58,684 instructions swept from the
  `.pdata` of 268 executables decode alike in Rust and in the VM's C: length, iclass, spec row index, every field, and
  every refusal.
- Measured, the design's revisit figures: the C emitter gives words for 1,186,246 of the 1,186,511 record calls,
  refusing 265, all `VMC_GUARDED`. That is by construction, since the table was inferred from this record. The
  record's 313 iclasses reach 190 of the 249 rule modules the registry holds (read with the frozen registry), above
  the half the revisit condition names. The record predates the x87 stencils: five of the table's eight symbols are not
  in its header and stand at address 0, which does not change whether the C emits.
- Planted fault: `opsize64` dropped from the `push` rows in `build.rs`: "record 0x1813e9762 [57]", Rust opsize 32
  against the C's 64, both on row 790; restored, the tests pass.
- Tests: x86_decoder 9 (8 carried, 1 differential).
