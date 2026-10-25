# Sprint 135 — The A64 encoder in Rust

```yaml
---
id: 135
status: active
phase: 6
pass_kind: functional
---
```

## scope

The walking skeleton's fourth link, and the first piece of the translator (S3) the plan's step 4 needs. A crate,
`a64_encoder`, carries `src/w2m/a64_encoder.py` function for function: every AArch64 word w2m emits, the integer,
memory, branch, SIMD and floating-point forms, the system registers, and the fixed words (ERET, the barriers, the TLB
and cache maintenance, the runtime lock's CASA and STLR). Its judge is the Python's: each case of
`tests/test_a64_encoder.py` (248) assembled by clang (`--target=aarch64-linux-gnu`) and read back with llvm-objdump
must give the word the function gives, plus ten words the monitor code uses that the Python test never checked
(`ic ialluis`, `tlbi vmalle1is`, the CASA and STLR, `adr`, the register and unscaled loads and stores).

This card was written after the code, the reverse of the kit's order; the scope is what was decided before the code
began (the plan's step 4 needs the whole encoder, and the stubs of Sprint 136 need it word for word), stated here
after the fact. The slip is in the Rubber Duck Pass.

## prerequisites

- 134 (closed)

## context_files

- `src/w2m/a64_encoder.py` (926 lines, read in full), `tests/test_a64_encoder.py` (read in full)
- `src/w2m/vm_backend.py` 4207-4267, 4343-4400 (the monitor code that uses it)

## signal contract

### Emits

None. An encoder; it puts nothing on a record.

### Consumes

Nothing at run time.

### Invariants

- No Python file is added or changed; no crate is added.

## artifact contract

### Files created

- `crates/a64_encoder/` (`src/lib.rs`, `tests/clang.rs`)

### Files modified

- `Cargo.toml`, `Cargo.lock`, `WORKING_AGREEMENT.md` (canonical home), `BLACKBOARD.md` (Sprint 133's CI run)

### Content assertions

- Every function of `a64_encoder.py` has a Rust function of the same name (`and_` is `and`, `str_` is `str`) and
  the same arguments, the Python's defaults written in.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected value:** 258 encodings equal clang's.
- **Planted fault:** MOVK's base constant off by one bit: the test fails naming the case and both words.

## done criteria

Every AArch64 word w2m emits is produced in Rust and equals clang's assembly of the same instruction.

## self-review checklist

- [x] Scope names one verifiable property: the encoder's words equal clang's.
- [x] context_files: the Python carried and its test.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: an expected value and a planted fault.
- [x] One concept: the encoder.

## result

- `crates/a64_encoder`: every function of the 926-line Python, the same names and arguments (its `and_` and `str_`
  as `and` and `str`), each assert carried as a panic.
- Expected value: 258 encodings equal clang's assembly: the Python test's 248 cases (its default arguments written
  in by pattern, the compiler naming each call that lacked one) and 10 added for the monitor code.
- Planted fault: MOVK's base 0xF2800000 changed to 0xF2800001: "movk x0, #0x7ffe, lsl #32: ours f2cfffc1, clang
  f2cfffc0"; restored, the test passes.
- Tests: a64_encoder 2 (`mov_imm`'s halfwords, the clang judge).
