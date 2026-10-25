# Sprint 137 — The PE reader in Rust

```yaml
---
id: 137
status: done
phase: 6
pass_kind: functional
---
```

## scope

The walking skeleton's sixth link, and loading's (S2) first piece: the guest program is a PE file, and the host reads
it before it can place it. A crate, `pe_image`, carries `src/w2m/pe.py` item for item: the headers, the sections, the
imports and delay imports with each IAT slot named, the base relocations, the exports with their forwarders and
ordinals, the TLS directory and callbacks, the `.pdata` function table, the resources, `read_rva`, `resource` and
`code_targets`. Malformed input is refused with the Python's message. Its judge is the Python's
(`tests/test_pe.py`): for every executable under `store/binaries/` (268), the entry RVA, the image base and the imports
equal llvm-objdump's reading of the same file; the structure checks hold (the `.text` section `r-x` and holding the
entry, only DIR64 and padding relocations, the entry among the code targets, the imports a probe may make); the three
malformed inputs and the i386 machine are refused with the Python's words; `read_rva` stops at a section's end; the
delay imports of `corpus/python-3.14.8/_wmi.pyd` are the five `ole32.dll` names; the relocation sites match their
counts.

This card was written after the crate's code was drafted (while Sprint 136's gates ran) and before any of its tests.
The slip is in the Rubber Duck Pass.

## prerequisites

- 136 (closed)

## context_files

- `src/w2m/pe.py` (352 lines, read in full), `tests/test_pe.py` (135 lines, read in full)

## signal contract

### Emits

None yet: the skeleton's record comes with the guest program.

### Consumes

Nothing at run time.

### Invariants

- No Python file is added or changed.

## artifact contract

### Files created

- `crates/pe_image/` (`src/lib.rs`, `tests/llvm_objdump.rs`)

### Files modified

- `Cargo.toml`, `Cargo.lock`, `WORKING_AGREEMENT.md` (canonical home), `BLACKBOARD.md` (Sprint 135's CI run)

### Content assertions

- Each item in `crates/pe_image/src/lib.rs` cites the `pe.py` lines it carries.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the eight checks of `tests/test_pe.py` above, over the same files, each citing the Python
  test's line.
- **Planted fault:** an import's name read one byte past its start (`+ 3` for `+ 2`): the llvm-objdump check fails
  and names the file and the DLL. (Amended before the result: the first plant, `+ 0`, read the hint's bytes, which
  the reader's own ASCII check refused before the comparison ran.)

## done criteria

A PE32+ image is read from Rust as `pe.py` read it, and 268 executables agree with llvm-objdump.

## self-review checklist

- [x] Scope names one verifiable property: the reader's agreement with llvm-objdump and the Python's checks.
- [x] context_files cite every Python line carried.
- [x] Emits: none yet, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: expected values and a planted fault.
- [x] One concept: the PE reader.

## result

- `crates/pe_image`: every item of the 352-line Python, each citing its lines. The Python's ordered dicts of imports
  and delay imports are lists of (DLL, names) in the directory's order, a repeated DLL replaced in place as the dict
  did. Bounds checks read the same widths as the Python's `struct` formats, so a truncated file is refused with the
  same message.
- Deviation: a TLS callback table or callback below the image base is refused; the Python appended a negative RVA.
- Expected values (`tests/llvm_objdump.rs`, 8 tests citing `tests/test_pe.py`): 268 executables under
  `store/binaries/` agree with llvm-objdump on the entry, the image base and every DLL's imports (the card first said
  280; the directory holds 268 `.exe` files beside 12 others); the structure checks; the three malformed inputs and
  the i386 machine refused with the Python's words; `read_rva` stopping at a section's end; `_wmi.pyd`'s five
  `ole32.dll` delay imports; the relocation sites matching their counts.
- Planted fault: names read one byte late (`+ 3`): "0006234b….exe: KERNEL32.dll's imports", "etCurrentProcessId"
  against "GetCurrentProcessId"; restored, the test passes. The first plant (`+ 0`) was refused earlier by the
  reader's own ASCII check ("non-ASCII string at offset 0x2364"), so the card was amended to a plant the comparison
  sees.
- Tests: pe_image 8.
